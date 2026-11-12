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
//   - Exact caller-authored Unicode glyph-stroke placement for CLI exports.
// - Must-Not:
//   - Invent glyph outlines, normalize Unicode, or claim calibrated ink.
// - Allows:
//   - Inputs: Bounded grapheme-to-segment templates and positioned text runs.
//   - Outputs: Physical pen segments or exhaustive exact missing-glyph refusal.
//   - Side effects: None.
// - Split-When:
//   - Persisted stroke vocabulary and semantic text layout are implemented.
// - Merge-When:
//   - Complete handwriting composition replaces CLI-only glyph placement.
// - Summary:
//   - Places explicitly authored glyph ink through the shared Rust compositor.
// - Description:
//   - Never substitutes a glyph or infers its shape from statistics.
// - Usage:
//   - --glyph-stroke 'ñ' 0 0 2500 4000 250 '#16293A'
//   - --place-text 1 25000 30000 'ñ'
// - Defaults:
//   - Four-millimetre horizontal advance; no implicit glyphs.
//

//! Exact, explicitly authored glyph contours for a bounded CLI text overlay.

use std::collections::{HashMap, HashSet};
use std::f64::consts::PI;
use std::fmt::Write as _;
use std::mem::{replace, swap};
use std::ops::Range;
use std::slice::Iter;
use std::str::from_utf8;

use atrament_handwriting_statistics::{
    Dimension, HandwritingStatisticsProfile,
};
use atrament_handwriting_stroke_projection::{
    AuthoredInkSegment, InkPlacement, MeasuredGlyphDimensions,
    PhysicalInkSegment, connect_ink_segments, project_ink_segment,
};
use atrament_notebook_paper_cli::{
    MAX_NOTEBOOK_INK_LINES, NotebookInkLine,
};
use atrament_unicode_grapheme_boundary_port::GraphemeBoundaryProvider as _;
use atrament_unicode_grapheme_segmentation::UnicodeGraphemeSegmentation;

use super::{
    export_integer, is_non_ink_audit_grapheme, strip_leading_source_bom,
    GlyphFallback, GlyphOverlay, GlyphStroke, TextRun, MAX_GLYPH_LIBRARY_BYTES,
};

const ADVANCE_UM: u32 = 4_000;
const MAX_STROKES: usize = 4096;
const MAX_RUNS: usize = 16;
const MAX_RUN_GRAPHEMES: usize = 96;
const MAX_FALLBACKS: usize = 64;
const MAX_TEXT_JOIN_PAIRS: usize = 32;
// An explicit short pen bridge must not silently cross a wide glyph gap.
const MAX_JOIN_GAP_UM: u64 = 1_200;
const GLYPH_LIBRARY_HEADER: &str = "atrament.glyph-strokes.v1";

// Authored local coordinates are only scaled when a separate saved
// statistical profile is explicitly selected by the caller.
// A configured slant additionally shears the upper contours in physical units.
const DEFAULT_TEMPLATE_WIDTH_UM: u32 = 3_500;

/// Opt-in creative jitter, not an inferred distribution or saved profile.
/// Units and the seeded xorshift match the browser glyph comparison.
#[derive(Clone, Copy, Debug)]
#[expect(
    clippy::redundant_pub_crate,
    reason = "CLI sibling module shares this type",
)]
pub(super) struct GlyphVariation {
    height: u32,
    regularity: u32,
    seed: u32,
    slant: u32,
    spacing: u32,
    thickness: u32,
    width: u32,
}

struct XorShift32(u32);

impl GlyphVariation {
    // A new stream is chosen per physical text run. Interpolate seeded
    // anchors eight grapheme columns apart; the shared baseline changes
    // slowly across letters and spaces without deforming authored contours.
    fn line_drift(
        self, run_index: usize, column: u32, maximum_um: u32,
    ) -> Result<i32, String> {
        let run = u32::try_from(run_index)
            .map_err(|_error| "line drift run index overflow")?;
        let anchor = column.div_euclid(8);
        let step = column.rem_euclid(8);
        let sample = |index: u32| -> Result<i32, String> {
            // Avalanche the three identities before xorshift: adjacent
            // document seeds otherwise differ only in low bits and can
            // round to the same physical micrometre at this small spread.
            let mut identity = self.seed
                ^ run.wrapping_add(1).wrapping_mul(0xD1B5_4A32)
                ^ index.wrapping_add(1).wrapping_mul(0x94D0_49BB);
            identity ^= identity >> 16i32;
            identity = identity.wrapping_mul(0x7FEB_352D);
            identity ^= identity >> 15i32;
            identity = identity.wrapping_mul(0x846C_A68B);
            identity ^= identity >> 16i32;
            let mut random = XorShift32::new(identity);
            Self::round_js(random.unit() * f64::from(maximum_um))
        };
        let from = i64::from(sample(anchor)?);
        let to = i64::from(sample(anchor.checked_add(1)
            .ok_or("line drift anchor overflow")?)?);
        let numerator = from.checked_mul(i64::from(8u32.saturating_sub(step)))
            .and_then(|first| to.checked_mul(i64::from(step))
                .and_then(|second| first.checked_add(second)))
            .ok_or("line drift interpolation overflow")?;
        let value = numerator.checked_div(8)
            .ok_or("line drift interpolation division failed")?;
        i32::try_from(value).map_err(|_error| "line drift overflow".into())
    }

    pub(super) fn parse(iter: &mut Iter<'_, String>) -> Result<Self, String> {
        let seed = next_integer(iter, "variation seed", 0, u32::MAX)?;
        let width = next_integer(iter, "variation width", 0, 20)?;
        let height = next_integer(iter, "variation height", 0, 20)?;
        let thickness = next_integer(iter, "variation thickness", 0, 20)?;
        let slant = next_integer(iter, "variation slant", 0, 12)?;
        let spacing = next_integer(iter, "variation spacing", 0, 500)?;
        let regularity = next_integer(iter, "variation regularity", 0, 150)?;
        Ok(Self {
            height, regularity, seed, slant, spacing, thickness, width,
        })
    }

    // JS Math.round is floor(n + 0.5), including negative half-way values.
    #[expect(clippy::as_conversions, clippy::cast_possible_truncation, reason =
        "bounded floating variation is checked before integer conversion")]
    fn round_js(value: f64) -> Result<i32, String> {
        let rounded = (value + 0.5).floor();
        if !rounded.is_finite() || rounded < f64::from(i32::MIN)
            || rounded > f64::from(i32::MAX) {
            return Err("varied glyph coordinate exceeds numeric bounds".into());
        }
        Ok(rounded as i32)
    }

    #[expect(clippy::suboptimal_flops, reason =
        "match browser JavaScript coordinate sampling without fused rounding")]
    fn sample(self, shapes: &[&GlyphStroke], occurrence: u32)
        -> Result<Vec<GlyphStroke>, String> {
        if shapes.len() > 128 {
            return Err("variation is limited to 128 strokes per glyph".into());
        }
        if occurrence > 10_000 {
            return Err("variation occurrence outside 0..10000".into());
        }
        let identity = Self::variation_seed(self.seed, occurrence);
        let mut random = XorShift32::new(identity);
        let sx = 1.0f64 + random.unit() * f64::from(self.width) / 100.0f64;
        let sy = 1.0f64 + random.unit() * f64::from(self.height) / 100.0f64;
        let ink = 1.0f64 + random.unit() * f64::from(self.thickness)
            / 100.0f64;
        let shear = (random.unit() * f64::from(self.slant)).to_radians().tan();
        let point = |x: u32, y: u32| -> Result<(u32, u32), String> {
            let mut vertex = XorShift32::new(identity
                ^ x.wrapping_mul(0x85EB_CA6B)
                ^ y.wrapping_mul(0xC2B2_AE35));
            let changed_x = Self::round_js(
                1750.0 + (f64::from(x) - 1750.0) * sx
                    + (4500.0 - f64::from(y)) * shear
                    + vertex.unit() * f64::from(self.regularity),
            )?;
            let changed_y = Self::round_js(
                2250.0 + (f64::from(y) - 2250.0) * sy
                    + vertex.unit() * f64::from(self.regularity),
            )?;
            Ok((
                u32::try_from(changed_x)
                    .map_err(|_error| "varied glyph exceeds cell" )?,
                u32::try_from(changed_y)
                    .map_err(|_error| "varied glyph exceeds cell" )?,
            ))
        };
        shapes.iter().map(|shape| {
            let start = point(shape.start_um.0, shape.start_um.1)?;
            let end = point(shape.end_um.0, shape.end_um.1)?;
            let width = u32::try_from(Self::round_js(
                f64::from(shape.width_um) * ink,
            )?).map_err(|_error| "varied glyph ink width invalid")?;
            if !(80..=1_500).contains(&width) || start == end {
                return Err("varied glyph pen width or length invalid".into());
            }
            let clearance = width.div_ceil(2);
            for (x, y) in [start, end] {
                if x < clearance || x > 3500u32.saturating_sub(clearance)
                    || y < clearance || y > 4500u32.saturating_sub(clearance) {
                    return Err(
                        "varied ink and pen must fit inside glyph cell".into(),
                    );
                }
            }
            Ok(GlyphStroke {
                start_um: start, end_um: end, width_um: width,
                ..(*shape).clone()
            })
        }).collect()
    }

    // Browser spacing is absolute occurrence tracking, not accumulated
    // advance; layout remains measured from the canonical glyph width.
    fn tracking(self, occurrence: u32) -> Result<i32, String> {
        let mut random = XorShift32::new(Self::variation_seed(
            self.seed, occurrence.checked_add(3)
                .ok_or("variation occurrence overflow")?,
        ));
        Self::round_js(random.unit() * f64::from(self.spacing))
    }

    const fn variation_seed(seed: u32, occurrence: u32) -> u32 {
        seed ^ occurrence.wrapping_add(1).wrapping_mul(0x9E37_79B9)
    }

    // This is one horizontal offset per ASCII-space-delimited word, not
    // independent letter tracking. The distinct seeded stream preserves the
    // line-rhythm samples and keeps all intra-word joins aligned.
    fn word_drift(
        self, run_index: usize, word_index: u32, maximum_um: u32,
    ) -> Result<i32, String> {
        let run = u32::try_from(run_index)
            .map_err(|_error| "word drift run index overflow")?;
        let mut identity = self.seed
            ^ run.wrapping_add(1).wrapping_mul(0x6C8E_9CF5)
            ^ word_index.wrapping_add(1).wrapping_mul(0xA24B_AED4);
        identity ^= identity >> 16i32;
        identity = identity.wrapping_mul(0x7FEB_352D);
        identity ^= identity >> 15i32;
        identity = identity.wrapping_mul(0x846C_A68B);
        identity ^= identity >> 16i32;
        let mut random = XorShift32::new(identity);
        Self::round_js(random.unit() * f64::from(maximum_um))
    }

    // A word shares one height multiplier, anchored at its lower baseline.
    // Keep it independent of glyph-level jitter and all other word rhythms.
    fn word_height(
        self, run_index: usize, word_index: u32, maximum_percent: u32,
    ) -> Result<u32, String> {
        let run = u32::try_from(run_index)
            .map_err(|_error| "word height run index overflow")?;
        let mut identity = self.seed
            ^ run.wrapping_add(1).wrapping_mul(0x94D0_49BB)
            ^ word_index.wrapping_add(1).wrapping_mul(0x369D_EA0F);
        identity ^= identity >> 16i32;
        identity = identity.wrapping_mul(0x7FEB_352D);
        identity ^= identity >> 15i32;
        identity = identity.wrapping_mul(0x846C_A68B);
        identity ^= identity >> 16i32;
        let mut random = XorShift32::new(identity);
        let delta = Self::round_js(
            random.unit() * f64::from(maximum_percent),
        )?;
        u32::try_from(100i32.checked_add(delta)
            .ok_or("word height overflow")?)
            .map_err(|_error| "word height must be positive".into())
    }

    // One independent width multiplier per word. All segments with equal
    // authored pen widths retain equal widths, so optional joins can use
    // the same physical pen throughout an ASCII-space-delimited word.
    fn word_pressure(
        self, run_index: usize, word_index: u32, maximum_percent: u32,
    ) -> Result<u32, String> {
        let run = u32::try_from(run_index)
            .map_err(|_error| "word pressure run index overflow")?;
        let mut identity = self.seed
            ^ run.wrapping_add(1).wrapping_mul(0xC2B2_AE35)
            ^ word_index.wrapping_add(1).wrapping_mul(0x27D4_EB2F);
        identity ^= identity >> 16i32;
        identity = identity.wrapping_mul(0x7FEB_352D);
        identity ^= identity >> 15i32;
        identity = identity.wrapping_mul(0x846C_A68B);
        identity ^= identity >> 16i32;
        let mut random = XorShift32::new(identity);
        let delta = Self::round_js(
            random.unit() * f64::from(maximum_percent),
        )?;
        u32::try_from(100i32.checked_add(delta)
            .ok_or("word pressure overflow")?)
            .map_err(|_error| "word pressure must be positive".into())
    }

    // Separate from horizontal word rhythm: a shared vertical displacement
    // keeps a word's authored contours and short selected joins together.
    fn word_rise(
        self, run_index: usize, word_index: u32, maximum_um: u32,
    ) -> Result<i32, String> {
        let run = u32::try_from(run_index)
            .map_err(|_error| "word rise run index overflow")?;
        let mut identity = self.seed
            ^ run.wrapping_add(1).wrapping_mul(0xB529_7A4D)
            ^ word_index.wrapping_add(1).wrapping_mul(0x68E3_1DA4);
        identity ^= identity >> 16i32;
        identity = identity.wrapping_mul(0x7FEB_352D);
        identity ^= identity >> 15i32;
        identity = identity.wrapping_mul(0x846C_A68B);
        identity ^= identity >> 16i32;
        let mut random = XorShift32::new(identity);
        Self::round_js(random.unit() * f64::from(maximum_um))
    }
    // A single lower-baseline shear per word keeps authored pen directions
    // and neighboring equal-height joins coherent without rewriting a glyph.
    fn word_slant(
        self, run_index: usize, word_index: u32, maximum_mdeg: u32,
    ) -> Result<i32, String> {
        let run = u32::try_from(run_index)
            .map_err(|_error| "word slant run index overflow")?;
        let mut identity = self.seed
            ^ run.wrapping_add(1).wrapping_mul(0x517C_C1B7)
            ^ word_index.wrapping_add(1).wrapping_mul(0x9E37_79B1);
        identity ^= identity >> 16i32;
        identity = identity.wrapping_mul(0x7FEB_352D);
        identity ^= identity >> 15i32;
        identity = identity.wrapping_mul(0x846C_A68B);
        identity ^= identity >> 16i32;
        let mut random = XorShift32::new(identity);
        Self::round_js(random.unit() * f64::from(maximum_mdeg))
    }
    // Each successive glyph in the same word receives the same additional
    // horizontal tracking step. The following word retains its nominal start
    // and flow wrapping remains tied to measured rather than sampled advance.
    fn word_tracking(
        self, run_index: usize, word_index: u32, maximum_um: u32,
    ) -> Result<i32, String> {
        let run = u32::try_from(run_index)
            .map_err(|_error| "word tracking run index overflow")?;
        let mut identity = self.seed
            ^ run.wrapping_add(1).wrapping_mul(0x3C6E_F372)
            ^ word_index.wrapping_add(1).wrapping_mul(0xBB67_AE85);
        identity ^= identity >> 16i32;
        identity = identity.wrapping_mul(0x7FEB_352D);
        identity ^= identity >> 15i32;
        identity = identity.wrapping_mul(0x846C_A68B);
        identity ^= identity >> 16i32;
        let mut random = XorShift32::new(identity);
        Self::round_js(random.unit() * f64::from(maximum_um))
    }

    // One bounded, seeded width multiplier per whole word. The scalar is
    // shared across its letters and independent of pressure and pen tilt.
    // Nominal advances stay unchanged; physical clearance is checked later.
    fn word_width(
        self, run_index: usize, word_index: u32, maximum_percent: u32,
    ) -> Result<u32, String> {
        let run = u32::try_from(run_index)
            .map_err(|_error| "word width run index overflow")?;
        let mut identity = self.seed
            ^ run.wrapping_add(1).wrapping_mul(0xD6E8_FEB9)
            ^ word_index.wrapping_add(1).wrapping_mul(0xA5CB_9243);
        identity ^= identity >> 16i32;
        identity = identity.wrapping_mul(0x7FEB_352D);
        identity ^= identity >> 15i32;
        identity = identity.wrapping_mul(0x846C_A68B);
        identity ^= identity >> 16i32;
        let mut random = XorShift32::new(identity);
        let delta = Self::round_js(
            random.unit() * f64::from(maximum_percent),
        )?;
        u32::try_from(100i32.checked_add(delta)
            .ok_or("word width overflow")?)
            .map_err(|_error| "word width must be positive".into())
    }
}

#[derive(Clone, Copy)]
struct WordInkStyle {
    height_percent: u32,
    pressure_percent: u32,
    slant_mdeg: i32,
    width_percent: u32,
}

impl XorShift32 {
    const fn new(seed: u32) -> Self {
        Self(if seed == 0 { 0x6D2B_79F5 } else { seed })
    }

    #[expect(clippy::suboptimal_flops, reason =
        "match browser JavaScript math without fused rounding")]
    fn unit(&mut self) -> f64 {
        self.0 ^= self.0 << 13i32;
        self.0 ^= self.0 >> 17i32;
        self.0 ^= self.0 << 5i32;
        f64::from(self.0) / f64::from(u32::MAX) * 2.0f64 - 1.0f64
    }
}


#[derive(Clone, Copy)]
struct GlyphMetrics {
    baseline_offset: i32,
    height: u32,
    ink_thickness: u32,
    letter_spacing: i32,
    slant: i32,
    width: u32,
}

impl GlyphMetrics {
    fn from_profile(
        profile: &HandwritingStatisticsProfile, grapheme: &str,
    ) -> Result<Self, String> {
        let mean = |dimension| -> Result<i32, String> {
            profile.statistics_for(grapheme, dimension)
                .map(|stat| stat.mean)
                .ok_or_else(|| format!("missing {dimension:?} statistics"))
        };
        let positive = |dimension| -> Result<u32, String> {
            u32::try_from(mean(dimension)?)
                .map_err(|error| format!("negative {dimension:?}: {error}"))
        };
        Ok(Self {
            baseline_offset: mean(Dimension::BaselineOffset)?,
            height: positive(Dimension::Height)?,
            ink_thickness: positive(Dimension::InkThickness)?,
            letter_spacing: mean(Dimension::LetterSpacing)?,
            slant: mean(Dimension::Slant)?,
            width: positive(Dimension::Width)?,
        })
    }
}

// A word is delimited only by literal ASCII spaces, matching native
// placement's structural gap rule; exact Unicode glyph keys stay intact.
#[derive(Default)]
struct WordColumns {
    in_word: bool,
    index: u32,
    letter_index: u32,
    seen_word: bool,
}

impl WordColumns {
    fn next(&mut self) -> Result<(u32, u32), String> {
        if self.in_word {
            self.letter_index = self.letter_index.checked_add(1)
                .ok_or("word letter index overflow")?;
        } else {
            if self.seen_word {
                self.index = self.index.checked_add(1)
                    .ok_or("word index overflow")?;
            }
            self.letter_index = 0;
            self.in_word = true;
            self.seen_word = true;
        }
        Ok((self.index, self.letter_index))
    }

    const fn space(&mut self) {
        self.in_word = false;
    }
}

impl GlyphOverlay {
    /// Select explicit exit/entry stroke ordinals for a pair. The source
    /// glyph geometry and canonical TSV remain unchanged.
    pub(super) fn add_text_join_anchors(
        &mut self, iter: &mut Iter<'_, String>,
    ) -> Result<(), String> {
        self.add_text_join_pair_with_anchors(iter, true)
    }

    /// Historical last-to-first opt-in pair, preserved for existing callers.
    pub(super) fn add_text_join_pair(
        &mut self, iter: &mut Iter<'_, String>,
    ) -> Result<(), String> {
        self.add_text_join_pair_with_anchors(iter, false)
    }

    fn add_text_join_pair_with_anchors(
        &mut self, iter: &mut Iter<'_, String>, anchored: bool,
    ) -> Result<(), String> {
        let left = iter.next()
            .ok_or("--join-text-pair needs two exact graphemes")?;
        let right = iter.next()
            .ok_or("--join-text-pair needs two exact graphemes")?;
        let boundaries = UnicodeGraphemeSegmentation;
        if !is_exact_visible_grapheme(left, boundaries)
            || !is_exact_visible_grapheme(right, boundaries) {
            return Err(concat!(
                "text join pair needs two visible ",
                "exact graphemes",
            ).into());
        }
        if self.text_joins.iter().any(|join| {
            join.left == *left && join.right == *right
        }) {
            return Err("duplicate text join pair refused".into());
        }
        if self.text_joins.len() == MAX_TEXT_JOIN_PAIRS {
            return Err("text joins limited to 32 selected pairs".into());
        }
        let anchors = if anchored {
            let exit = next_integer(
                iter, "join exit stroke", 1, 4_096,
            )?;
            let entry = next_integer(
                iter, "join entry stroke", 1, 4_096,
            )?;
            Some((usize::try_from(exit)
                .map_err(|error| error.to_string())?.saturating_sub(1),
                usize::try_from(entry)
                .map_err(|error| error.to_string())?.saturating_sub(1)))
        } else { None };
        self.text_joins.push(super::TextJoin {
            left: left.clone(), right: right.clone(), anchors,
        });
        Ok(())
    }

    /// Inspect the actual authored-shape inventory for one bounded text run.
    /// An English/Spanish statistics baseline alone never implies ink.
    pub(super) fn audit_text(&self, text: &str)
        -> Result<Vec<u8>, String> {
        super::validate_glyph_audit_text(text)?;
        self.validate_stroke_library()?;
        // Count each exact authored key once after validating the *whole*
        // library, including unused rows. A bounded Unicode audit can
        // mention over a thousand distinct keys against 4096 strokes;
        // rescanning all strokes per key makes inspection quadratic.
        let mut stroke_counts: HashMap<&str, usize> = HashMap::new();
        for stroke in &self.strokes {
            let count = stroke_counts.entry(stroke.grapheme.as_str())
                .or_default();
            *count = count.checked_add(1)
                .ok_or("glyph audit stroke count overflow")?;
        }
        // Hash membership only suppresses repeats. Emit reports in first
        // source occurrence order, never hash iteration order.
        let mut seen = HashSet::new();
        let mut missing = 0usize;
        let mut details = String::new();
        let mut remaining = strip_leading_source_bom(text);
        while !remaining.is_empty() {
            let (grapheme, rest) = first_grapheme(remaining)?;
            remaining = rest;
            // Only ASCII spaces are implicit 4-mm blank advances in the
            // actual compositor. Other Unicode whitespace may be authored
            // visible ink and must appear in this exact coverage audit.
            // Source line and paragraph separators are non-ink structure.
            if is_non_ink_audit_grapheme(grapheme)
                || !seen.insert(grapheme) {
                continue;
            }
            let strokes = stroke_counts.get(grapheme).copied().unwrap_or(0);
            if strokes == 0 {
                missing = missing.saturating_add(1);
            }
            let codepoints = grapheme.chars()
                .map(|ch| format!("U+{:04X}", u32::from(ch)))
                .collect::<Vec<_>>().join(" ");
            let _written = writeln!(
                details, "\"{}\" [{codepoints}]: {}",
                grapheme.escape_debug(),
                if strokes == 0 {
                    String::from("missing authored strokes")
                } else {
                    format!("{strokes} authored strokes")
                },
            );
        }
        Ok(format!(
            concat!(
                "Exact authored glyph audit (not a font or sampler)\n",
                "Distinct visible graphemes: {}\n",
                "Missing authored glyphs: {}\n",
                "{}",
            ),
            seen.len(), missing, details,
        ).into_bytes())
    }

    fn checked_stroke_position(
        positions: &[usize], ordinal: u32,
    ) -> Result<usize, String> {
        let index = usize::try_from(ordinal.checked_sub(1)
            .ok_or("stroke indices start at 1")?)
            .map_err(|_error| "stroke index outside bounds")?;
        if index >= positions.len() {
            return Err(format!(
                "stroke index {ordinal} exceeds glyph's {} strokes",
                positions.len(),
            ));
        }
        Ok(index)
    }

    /// Explicitly finish one open, same-pen candidate path with a short
    /// straight segment from its directed end to its original start.
    /// The closing line follows the path's last glyph slot, never rewrites
    /// existing lines or folds Unicode identities.
    pub(super) fn close_path(
        &mut self, grapheme: &str, ordinal: u32,
    ) -> Result<(), String> {
        let (positions, paths) = self.selected_candidate_paths(grapheme)?;
        let index = checked_path_position(&paths, ordinal)?;
        let path = paths.get(index).ok_or("path index outside bounds")?;
        if path.len() < 2 {
            return Err("path closure requires two continuous strokes".into());
        }
        let first_index = *positions.get(path.start)
            .ok_or("first path segment outside library")?;
        let last_index = *positions.get(path.end.saturating_sub(1))
            .ok_or("last path segment outside library")?;
        let first = self.strokes.get(first_index)
            .ok_or("first path segment outside library")?;
        let last = self.strokes.get(last_index)
            .ok_or("last path segment outside library")?;
        let start = last.end_um;
        let end = first.start_um;
        if start == end {
            return Err("path is already closed".into());
        }
        let dx = i64::from(start.0).saturating_sub(i64::from(end.0));
        let dy = i64::from(start.1).saturating_sub(i64::from(end.1));
        let squared = dx.unsigned_abs().pow(2)
            .saturating_add(dy.unsigned_abs().pow(2));
        if squared > MAX_JOIN_GAP_UM.pow(2) {
            return Err("path closure exceeds 1200 um gap limit".into());
        }
        if self.strokes.len() == MAX_STROKES {
            return Err("path closure exceeds 4096 authored strokes".into());
        }
        ensure_inserted_bridge_fits_cell(start, end, last.width_um)?;
        let closure = GlyphStroke {
            grapheme: grapheme.to_owned(), start_um: start, end_um: end,
            width_um: last.width_um, rgb: last.rgb,
        };
        self.strokes.insert(last_index.saturating_add(1), closure);
        Ok(())
    }

    /// Copy exact authored pen geometry to a new independent Unicode key.
    /// The source remains untouched; no live dependency is established.
    pub(super) fn copy_glyph(
        &mut self, source: &str, target: &str,
    ) -> Result<(), String> {
        validate_copy_glyph_keys(source, target)?;
        self.validate_stroke_library()?;
        if self.strokes.iter().any(|stroke| stroke.grapheme == target) {
            return Err("glyph copy refuses an already authored target".into());
        }
        let copies = self.strokes.iter()
            .filter(|stroke| stroke.grapheme == source)
            .map(|stroke| GlyphStroke {
                grapheme: target.to_owned(), ..stroke.clone()
            }).collect::<Vec<_>>();
        if copies.is_empty() {
            return Err("glyph copy source has no authored strokes".into());
        }
        if copies.len() > MAX_STROKES.saturating_sub(self.strokes.len()) {
            return Err("glyph copy exceeds 4096 authored strokes".into());
        }
        self.strokes.extend(copies);
        Ok(())
    }

    /// CLI admission for one independently copied glyph pair.
    pub(super) fn copy_glyph_option(
        &mut self, iter: &mut Iter<'_, String>,
    ) -> Result<(), String> {
        let source = iter.next()
            .ok_or("--copy-glyph requires source and target graphemes")?;
        let target = iter.next()
            .ok_or("--copy-glyph requires source and target graphemes")?;
        self.copy_glyph(source, target)
    }

    /// Path numbers are derived from the same exact adjacent-pen test as
    /// --inspect-paths. Edits never introduce v1 path identity metadata.
    pub(super) fn edit_path(
        &mut self, flag: &str, iter: &mut Iter<'_, String>,
    ) -> Result<(), String> {
        let grapheme = iter.next()
            .ok_or("path edit requires an exact grapheme")?;
        let from = next_integer(iter, "path index", 1, 4096)?;
        match flag {
            "--reverse-path" => self.reverse_path(grapheme, from),
            "--close-path" => self.close_path(grapheme, from),
            "--rotate-path" => {
                let start = next_integer(iter, "path start stroke", 1, 4096)?;
                self.rotate_path(grapheme, from, start)
            }
            "--move-path" => {
                let to = next_integer(iter, "path index", 1, 4096)?;
                self.move_path(grapheme, from, to)
            }
            "--translate-path" => {
                let dx = next_signed_path_offset(iter, "path dx", 3500)?;
                let dy = next_signed_path_offset(iter, "path dy", 4500)?;
                self.translate_path(grapheme, from, dx, dy)
            }
            _ => Err("unknown path edit".into()),
        }
    }

    /// Apply a requested native index edit to the in-memory authored library.
    pub(super) fn edit_stroke(
        &mut self, flag: &str, iter: &mut Iter<'_, String>,
    ) -> Result<(), String> {
        let grapheme = iter.next()
            .ok_or("stroke edit requires an exact grapheme")?;
        let from = next_stroke_index(iter)?;
        match flag {
            "--reverse-stroke" => self.reverse_stroke(grapheme, from),
            "--move-stroke" => {
                let to = next_stroke_index(iter)?;
                self.move_stroke(grapheme, from, to)
            }
            "--remove-stroke" => self.remove_stroke(grapheme, from),
            "--join-strokes" => self.join_strokes(grapheme, from),
            "--set-stroke" => {
                let replacement = Self::parse_stroke(grapheme, iter)?;
                self.set_stroke(grapheme, from, replacement)
            }
            _ => Err("unknown stroke edit".into()),
        }
    }

    /// Canonical portable glyph geometry, not placed text or sampled ink.
    pub(super) fn encode_file(&self) -> Result<Vec<u8>, String> {
        if self.strokes.is_empty() {
            return Err(
                "glyph library needs at least one authored stroke".into()
            );
        }
        self.validate_stroke_library()?;
        let mut text = format!("{GLYPH_LIBRARY_HEADER}\n");
        for stroke in &self.strokes {
            let _written = writeln!(
                text,
                "{}\t{}\t{}\t{}\t{}\t{}\t#{:02X}{:02X}{:02X}",
                stroke.grapheme,
                stroke.start_um.0,
                stroke.start_um.1,
                stroke.end_um.0,
                stroke.end_um.1,
                stroke.width_um,
                stroke.rgb[0],
                stroke.rgb[1],
                stroke.rgb[2],
            );
        }
        if text.len() > MAX_GLYPH_LIBRARY_BYTES {
            return Err("glyph library exceeds 524288 bytes".into());
        }
        Ok(text.into_bytes())
    }

    pub(super) fn ensure_can_load_file(&self) -> Result<(), String> {
        if self.loaded_file || self.has_any() {
            return Err(concat!(
                "--glyph-file or --profile must precede authored strokes ",
                "and may appear only once",
            )
            .into());
        }
        Ok(())
    }

    pub(super) fn expand(
        &self, manual: &[NotebookInkLine],
        statistics: Option<&HandwritingStatisticsProfile>,
    ) -> Result<Vec<NotebookInkLine>, String> {
        self.expand_with_run_boundary(manual, statistics, 0)
            .map(|(ink, _boundary)| ink)
    }

    // The prefix is generated flow; any later runs are explicit overlays.
    // Preserve one expansion/variation order and isolate physical ink bounds
    // without making manually placed text obey a flow-only rectangle.
    pub(super) fn expand_with_run_boundary(
        &self, manual: &[NotebookInkLine],
        statistics: Option<&HandwritingStatisticsProfile>,
        boundary_run: usize,
    ) -> Result<(Vec<NotebookInkLine>, usize), String> {
        validate_run_boundary(self.runs.len(), boundary_run)?;
        // A typed sibling can supply a mutated profile without passing
        // through the CLI's saved-JSON decoder. Refuse malformed versions,
        // duplicate dimensions or unbounded means before placing any ink.
        validate_glyph_statistics(statistics)?;
        self.validate_pairing()?;
        validate_manual_ink(manual)?;
        // Index the already-admitted library once per export. A native
        // library can contain 4096 interleaved exact Unicode strokes;
        // repeatedly scanning every unrelated shape for each text glyph
        // makes long paginated documents needlessly quadratic. Append in
        // original order so authored paths and seeded sampling are stable.
        let by_grapheme = self.index_strokes();
        let mut expanded = manual.to_vec();
        let mut boundary_ink_end = (boundary_run == 0)
            .then_some(expanded.len());
        let mut missing = Vec::new();
        let mut occurrence = 0u32;
        let mut joined_pairs = vec![false; self.text_joins.len()];
        for (run_index, run) in self.runs.iter().enumerate() {
            let mut offset_um = 0i64;
            let mut column = 0u32;
            let mut words = WordColumns::default();
            // A pen lift is implicit at every run/row or authored space.
            let mut previous: Option<(&str, Vec<NotebookInkLine>)> = None;
            let mut remaining = run.text.as_str();
            while !remaining.is_empty() {
                let (grapheme, rest) = first_grapheme(remaining)?;
                remaining = rest;
                if grapheme == " " {
                    words.space();
                    previous = None;
                    offset_um = offset_um.checked_add(i64::from(ADVANCE_UM))
                        .ok_or("space advance overflow")?;
                    column = column.checked_add(1).ok_or("column overflow")?;
                    continue;
                }
                let (word_index, letter_index) = words.next()?;
                let authored = self.fallbacks.iter()
                    .find(|entry| entry.missing == grapheme)
                    .map_or(grapheme, |entry| entry.authored.as_str());
                let Some(matches) = by_grapheme.get(authored) else {
                    previous = None;
                    if !missing.contains(&grapheme) {
                        missing.push(grapheme);
                    }
                    continue;
                };
                let metrics = statistics
                    .map(|profile| {
                        GlyphMetrics::from_profile(profile, grapheme)
                    })
                    .transpose()?;
                let tracking = self.variation.map_or(Ok(0i32), |variation| {
                    variation.tracking(occurrence)
                })?;
                let selected = if let Some(variation) = self.variation {
                    variation.sample(matches, occurrence)?
                } else {
                    matches.iter().map(|shape| (*shape).clone()).collect()
                };
                let glyph_x = self.glyph_origin_x(
                    run, run_index, word_index, letter_index,
                    offset_um, tracking)?;
                let glyph_y = self.glyph_origin_y(
                    run, run_index, word_index, column, metrics,
                )?;
                let style = self.glyph_style(run_index, word_index)?;
                let rendered = selected.iter().map(|shape| placed_ink(
                    shape, run, (glyph_x, glyph_y), metrics, style,
                )).collect::<Result<Vec<_>, _>>()?;
                append_text_ink(
                    &mut expanded, &rendered,
                    previous.as_ref().map(|(glyph, strokes)| (
                        *glyph, strokes.as_slice(),
                    )),
                    grapheme, &self.text_joins, &mut joined_pairs,
                )?;
                previous = Some((grapheme, rendered));
                let advance = if let Some(values) = metrics {
                    i64::from(values.width)
                        .checked_add(i64::from(values.letter_spacing))
                        .ok_or("glyph letter advance overflow")?
                } else {
                    i64::from(ADVANCE_UM)
                };
                if advance <= 0 {
                    return Err(concat!(
                        "glyph width plus letter spacing ",
                        "must be positive",
                    ).into());
                }
                offset_um = offset_um.checked_add(advance)
                    .ok_or("glyph text advance overflow")?;
                occurrence = occurrence.checked_add(1)
                    .ok_or("variation occurrence overflow")?;
                column = column.checked_add(1).ok_or("column overflow")?;
            }
            if run_index.saturating_add(1) == boundary_run {
                boundary_ink_end = Some(expanded.len());
            }
        }
        if !missing.is_empty() {
            return Err(missing_shapes_error(&missing));
        }
        self.validate_seen_text_joins(&joined_pairs)?;
        Ok((expanded, boundary_ink_end
            .ok_or("flow run boundary exceeds positioned text runs")?))
    }

    /// Layout uses the same per-grapheme advances as physical ink placement.
    /// The second value is the nominal glyph contour width for clearance.
    pub(super) fn flow_advance_um(
        grapheme: &str,
        statistics: Option<&HandwritingStatisticsProfile>,
    ) -> Result<(u32, u32), String> {
        if grapheme == " " {
            return Ok((ADVANCE_UM, 0));
        }
        let Some(profile) = statistics else {
            return Ok((ADVANCE_UM, DEFAULT_TEMPLATE_WIDTH_UM));
        };
        let metrics = GlyphMetrics::from_profile(profile, grapheme)?;
        let advance = i64::from(metrics.width)
            .checked_add(i64::from(metrics.letter_spacing))
            .ok_or("glyph letter advance overflow")?;
        if advance <= 0 {
            return Err(
                "glyph width plus letter spacing must be positive".into(),
            );
        }
        Ok((u32::try_from(advance)
            .map_err(|_error| "glyph letter advance overflow")?,
            metrics.width))
    }

    fn glyph_height(
        &self, run_index: usize, word_index: u32,
    ) -> Result<u32, String> {
        match (self.variation, self.word_height_percent) {
            (Some(variation), Some(maximum)) =>
                variation.word_height(run_index, word_index, maximum),
            _ => Ok(100),
        }
    }

    fn glyph_origin_x(
        &self, run: &TextRun, run_index: usize, word_index: u32,
        letter_index: u32, offset_um: i64, tracking: i32,
    ) -> Result<i64, String> {
        let drift = match (self.variation, self.word_drift_um) {
            (Some(variation), Some(maximum_um)) => variation
                .word_drift(run_index, word_index, maximum_um)?,
            _ => 0i32,
        };
        let word_step = match (self.variation, self.word_tracking_um) {
            (Some(variation), Some(maximum_um)) => variation
                .word_tracking(run_index, word_index, maximum_um)?,
            _ => 0i32,
        };
        let offset = i64::from(word_step)
            .checked_mul(i64::from(letter_index))
            .ok_or("word tracking offset overflow")?;
        i64::from(run.origin_um.0).checked_add(offset_um)
            .and_then(|value| value.checked_add(offset))
            .and_then(|value| value.checked_add(i64::from(tracking)))
            .and_then(|value| value.checked_add(i64::from(drift)))
            .ok_or_else(|| "glyph horizontal origin overflow".into())
    }

    fn glyph_origin_y(
        &self, run: &TextRun, run_index: usize, word_index: u32,
        column: u32, metrics: Option<GlyphMetrics>,
    ) -> Result<i64, String> {
        let drift = match (self.variation, self.line_drift_um) {
            (Some(variation), Some(maximum_um)) =>
                variation.line_drift(run_index, column, maximum_um)?,
            _ => 0i32,
        };
        let rise = match (self.variation, self.word_rise_um) {
            (Some(variation), Some(maximum_um)) =>
                variation.word_rise(run_index, word_index, maximum_um)?,
            _ => 0i32,
        };
        i64::from(run.origin_um.1)
            .checked_add(i64::from(metrics.map_or(
                0i32, |values| values.baseline_offset,
            )))
            .and_then(|value| value.checked_add(i64::from(drift)))
            .and_then(|value| value.checked_add(i64::from(rise)))
            .ok_or_else(|| "glyph vertical origin overflow".into())
    }

    fn glyph_pressure(
        &self, run_index: usize, word_index: u32,
    ) -> Result<u32, String> {
        match (self.variation, self.word_pressure_percent) {
            (Some(variation), Some(maximum)) =>
                variation.word_pressure(run_index, word_index, maximum),
            _ => Ok(100),
        }
    }

    fn glyph_slant(
        &self, run_index: usize, word_index: u32,
    ) -> Result<i32, String> {
        match (self.variation, self.word_slant_millidegrees) {
            (Some(variation), Some(maximum)) =>
                variation.word_slant(run_index, word_index, maximum),
            _ => Ok(0),
        }
    }

    fn glyph_style(
        &self, run_index: usize, word_index: u32,
    ) -> Result<WordInkStyle, String> {
        Ok(WordInkStyle {
            height_percent: self.glyph_height(run_index, word_index)?,
            pressure_percent: self.glyph_pressure(run_index, word_index)?,
            slant_mdeg: self.glyph_slant(run_index, word_index)?,
            width_percent: self.glyph_width(run_index, word_index)?,
        })
    }

    fn glyph_width(
        &self, run_index: usize, word_index: u32,
    ) -> Result<u32, String> {
        match (self.variation, self.word_width_percent) {
            (Some(variation), Some(maximum)) =>
                variation.word_width(run_index, word_index, maximum),
            _ => Ok(100),
        }
    }

    pub(super) const fn has_any(&self) -> bool {
        !self.runs.is_empty() || !self.strokes.is_empty()
            || !self.fallbacks.is_empty() || self.variation.is_some()
            || !self.text_joins.is_empty() || self.line_drift_um.is_some()
            || self.word_drift_um.is_some()
            || self.word_rise_um.is_some()
            || self.word_pressure_percent.is_some()
            || self.word_slant_millidegrees.is_some()
            || self.word_width_percent.is_some()
            || self.word_height_percent.is_some()
            || self.word_tracking_um.is_some()
    }

    // Index once per expansion, retaining authored order for each exact key.
    fn index_strokes(&self) -> HashMap<&str, Vec<&GlyphStroke>> {
        let mut by_grapheme = HashMap::new();
        for shape in &self.strokes {
            by_grapheme.entry(shape.grapheme.as_str())
                .or_insert_with(Vec::new).push(shape);
        }
        by_grapheme
    }

    /// Inspect consecutive directed pen segments as contiguous candidate
    /// paths; never assert persisted pen-down groups or inferred cursive.
    pub(super) fn inspect_paths(
        &self, grapheme: &str,
    ) -> Result<Vec<u8>, String> {
        validate_inspection_glyph_key(grapheme)?;
        self.validate_stroke_library()?;
        let selected = self.strokes.iter()
            .filter(|stroke| stroke.grapheme == grapheme)
            .collect::<Vec<_>>();
        if selected.is_empty() {
            return Err("no authored strokes for exact glyph inspection".into());
        }
        let codepoints = grapheme.chars()
            .map(|ch| format!("U+{:04X}", u32::from(ch)))
            .collect::<Vec<_>>().join(" ");
        let mut details = String::new();
        let paths = candidate_path_ranges(&selected);
        for (number, path) in paths.iter().enumerate() {
            append_path_summary(
                &mut details, &selected, number.saturating_add(1),
                path.start, path.end,
            )?;
        }
        Ok(format!(
            concat!(
                "Candidate continuous pen paths (exact authored order)\n",
                "Glyph: \"{}\" [{}]\n",
                "Strokes: {}\nPaths: {}\nPen lifts: {}\n{}",
            ),
            grapheme.escape_debug(), codepoints, selected.len(), paths.len(),
            paths.len().saturating_sub(1), details,
        ).into_bytes())
    }

    /// Show the exact caller-authored order and direction without writing
    /// a file, inferring joins, normalizing Unicode or sampling variation.
    pub(super) fn inspect_stroke_order(
        &self, grapheme: &str,
    ) -> Result<Vec<u8>, String> {
        validate_inspection_glyph_key(grapheme)?;
        self.validate_stroke_library()?;
        let selected = self.strokes.iter()
            .filter(|stroke| stroke.grapheme == grapheme)
            .collect::<Vec<_>>();
        if selected.is_empty() {
            return Err("no authored strokes for exact glyph inspection".into());
        }
        let codepoints = grapheme.chars()
            .map(|ch| format!("U+{:04X}", u32::from(ch)))
            .collect::<Vec<_>>().join(" ");
        let mut output = format!(
            concat!(
                "Exact authored stroke sequence (not inferred cursive)\n",
                "Glyph: \"{}\" [{}]\n",
                "Strokes: {}\n",
            ),
            grapheme.escape_debug(), codepoints, selected.len(),
        );
        for (index, stroke) in selected.into_iter().enumerate() {
            let _written = writeln!(
                output,
                concat!(
                    "{}: ({}, {}) -> ({}, {}) ",
                    "width {} um color #{:02X}{:02X}{:02X}",
                ),
                index.saturating_add(1),
                stroke.start_um.0, stroke.start_um.1,
                stroke.end_um.0, stroke.end_um.1,
                stroke.width_um, stroke.rgb[0], stroke.rgb[1],
                stroke.rgb[2],
            );
        }
        Ok(output.into_bytes())
    }

    /// Connect the first segment's pen-down endpoint to the *next* segment's
    /// starting point in one exact Unicode glyph. Explicitly preserve the
    /// authored endpoints, ink color and width; do not infer cursive joins.
    pub(super) fn join_strokes(
        &mut self, grapheme: &str, ordinal: u32,
    ) -> Result<(), String> {
        let positions = self.matching_stroke_positions(grapheme)?;
        let index = Self::checked_stroke_position(&positions, ordinal)?;
        let next = index.checked_add(1).ok_or("stroke index overflow")?;
        let first = *positions.get(index)
            .ok_or("stroke index outside bounds")?;
        let second = *positions.get(next)
            .ok_or("joining requires a following stroke in the same glyph")?;
        let before = self.strokes.get(first)
            .ok_or("stroke index outside library")?;
        let after = self.strokes.get(second)
            .ok_or("stroke index outside library")?;
        if before.width_um != after.width_um || before.rgb != after.rgb {
            return Err("joining requires matching pen width and RGB".into());
        }
        let start = before.end_um;
        let end = after.start_um;
        if start == end {
            return Err("strokes are already continuous".into());
        }
        let dx = i64::from(start.0).saturating_sub(i64::from(end.0));
        let dy = i64::from(start.1).saturating_sub(i64::from(end.1));
        let distance_squared = dx.unsigned_abs().pow(2)
            .saturating_add(dy.unsigned_abs().pow(2));
        if distance_squared > MAX_JOIN_GAP_UM.pow(2) {
            return Err("stroke gap exceeds 1200 um join limit".into());
        }
        if self.strokes.len() == MAX_STROKES {
            return Err("joining exceeds 4096 authored strokes".into());
        }
        ensure_inserted_bridge_fits_cell(start, end, before.width_um)?;
        let bridge = GlyphStroke {
            grapheme: grapheme.to_owned(),
            start_um: start,
            end_um: end,
            width_um: before.width_um,
            rgb: before.rgb,
        };
        // The connector follows the first glyph segment. Unrelated
        // interleaved rows retain their own order and exact shapes.
        self.strokes.insert(first.saturating_add(1), bridge);
        Ok(())
    }

    /// Load a reusable exact-Unicode library with tab-separated columns.
    /// Grapheme identities are never implicitly normalized or substituted.
    pub(super) fn load_file_bytes(
        &mut self, bytes: &[u8],
    ) -> Result<(), String> {
        // Typed callers must retain the check even when the CLI has already
        // refused invalid option order before opening a file or reading stdin.
        self.ensure_can_load_file()?;
        if bytes.len() > MAX_GLYPH_LIBRARY_BYTES {
            return Err("glyph library exceeds 524288 bytes".into());
        }
        let contents =
            from_utf8(bytes).map_err(|_error| "glyph library must be UTF-8")?;
        if !contents.ends_with('\n') || contents.contains('\r') {
            return Err("glyph library requires LF-terminated lines".into());
        }
        let mut lines = contents.split_terminator('\n');
        if lines.next() != Some(GLYPH_LIBRARY_HEADER) {
            return Err("unsupported glyph library header".into());
        }
        // Parse into a detached draft: a bad later TSV line must not leave
        // valid prefix ink behind in a reusable typed caller's overlay.
        let mut staged = Self::default();
        for (index, line) in lines.enumerate() {
            if staged.strokes.len() == MAX_STROKES {
                return Err("glyph library exceeds 4096 strokes".into());
            }
            let fields: Vec<String> = line.split('\t')
                .map(str::to_owned).collect();
            if fields.len() != 7 {
                return Err(format!(
                    "glyph library line {} needs seven tab-separated fields",
                    index.saturating_add(2),
                ));
            }
            // Incrementally validated TSV rows do not need the typed
            // mutation path's repeated full-library preflight.
            let mut tokens = fields.iter();
            let key = tokens.next().ok_or("missing glyph key")?;
            let stroke = Self::parse_stroke(key, &mut tokens)
                .map_err(|error| format!(
                    "glyph library line {}: {error}",
                    index.saturating_add(2),
                ))?;
            staged.strokes.push(stroke);
        }
        if staged.strokes.is_empty() {
            return Err(
                "glyph library needs at least one authored stroke".into()
            );
        }
        self.strokes = staged.strokes;
        self.loaded_file = true;
        Ok(())
    }

    /// Select one-based authored stroke positions without normalizing the
    /// grapheme or disturbing interleaved records for other glyphs.
    fn matching_stroke_positions(&self, grapheme: &str)
        -> Result<Vec<usize>, String> {
        if !is_exact_visible_grapheme(grapheme, UnicodeGraphemeSegmentation) {
            return Err(
                "stroke edit requires one exact visible grapheme".into()
            );
        }
        self.validate_stroke_library()?;
        let positions = self.strokes.iter().enumerate()
            .filter_map(|(index, stroke)| {
                (stroke.grapheme == grapheme).then_some(index)
            }).collect::<Vec<_>>();
        if positions.is_empty() {
            return Err("no authored strokes for exact glyph edit".into());
        }
        Ok(positions)
    }

    /// Swap two adjacent complete path ranges, retaining each constituent
    /// segment direction, pen settings, and unrelated interleaved records.
    pub(super) fn move_path(
        &mut self, grapheme: &str, from: u32, to: u32,
    ) -> Result<(), String> {
        let (positions, paths) = self.selected_candidate_paths(grapheme)?;
        let first = checked_path_position(&paths, from)?;
        let second = checked_path_position(&paths, to)?;
        if first.abs_diff(second) != 1 {
            return Err(
                "path movement requires adjacent different paths".into()
            );
        }
        let lower = paths.get(first.min(second))
            .ok_or("lower path outside bounds")?;
        let upper = paths.get(first.max(second))
            .ok_or("upper path outside bounds")?;
        let selected = |range: Range<usize>|
            -> Result<Vec<GlyphStroke>, String> {
            positions.get(range).ok_or("path range outside glyph")?
                .iter().map(|position| {
                    self.strokes.get(*position).cloned()
                        .ok_or_else(|| {
                            "path segment outside library".to_owned()
                        })
                }).collect()
        };
        let mut reordered = selected(upper.clone())?;
        reordered.extend(selected(lower.clone())?);
        let slots = positions.get(lower.start..upper.end)
            .ok_or("path range outside glyph")?;
        for (position, stroke) in slots.iter().zip(reordered) {
            *self.strokes.get_mut(*position)
                .ok_or("path segment outside library")? = stroke;
        }
        Ok(())
    }

    /// Move one segment to its new one-based ordinal within the *same* exact
    /// glyph, shifting intermediate glyph strokes and leaving other glyph
    /// records in their original TSV slots.
    pub(super) fn move_stroke(
        &mut self, grapheme: &str, from: u32, to: u32,
    ) -> Result<(), String> {
        let positions = self.matching_stroke_positions(grapheme)?;
        let first = Self::checked_stroke_position(&positions, from)?;
        let last = Self::checked_stroke_position(&positions, to)?;
        if first == last {
            return Err("stroke move requires different positions".into());
        }
        let steps = if first < last {
            (first..last).collect::<Vec<_>>()
        } else {
            (last..first).rev().collect::<Vec<_>>()
        };
        for index in steps {
            let next = index.checked_add(1).ok_or("stroke index overflow")?;
            let left = *positions.get(index)
                .ok_or("stroke move index outside bounds")?;
            let right = *positions.get(next)
                .ok_or("stroke move index outside bounds")?;
            self.strokes.swap(left, right);
        }
        Ok(())
    }

    pub(super) fn parse(
        &mut self, flag: &str, iter: &mut Iter<'_, String>,
    ) -> Result<(), String> {
        match flag {
            "--copy-glyph" => self.copy_glyph_option(iter)?,
            "--shear-glyph" => self.shear_glyph_option(iter)?,
            "--rotate-glyph" => self.rotate_glyph_option(iter)?,
            "--scale-glyph-height" => self.scale_glyph_height_option(iter)?,
            "--scale-glyph-width" => self.scale_glyph_width_option(iter)?,
            "--scale-glyph-ink" => self.scale_glyph_ink_option(iter)?,
            "--translate-glyph" => self.translate_glyph_option(iter)?,
            "--glyph-stroke" => {
                // Typed callers can append after altering an earlier row.
                // An invalid existing inventory must not accept more ink.
                self.validate_stroke_library()?;
                if self.strokes.len() == MAX_STROKES {
                    return Err("more than 4096 glyph strokes refused".into());
                }
                let grapheme = iter.next()
                    .ok_or("--glyph-stroke requires one grapheme")?;
                self.strokes.push(Self::parse_stroke(grapheme, iter)?);
            }
            "--place-text" => {
                if self.runs.len() == MAX_RUNS {
                    return Err("more than 16 placed text runs refused".into());
                }
                let page = next_integer(iter, "text page", 1, 32)?;
                let x = next_integer(iter, "text x", 0, 700_000)?;
                let y = next_integer(iter, "text y", 0, 700_000)?;
                let text = iter.next().ok_or("--place-text needs UTF-8 text")?;
                let run = TextRun {
                    heading_ink: None,
                    page: usize::try_from(page).map_err(|error| {
                        format!("invalid text page: {error}")
                    })?,
                    origin_um: (x, y),
                    text: text.clone(),
                };
                validate_text_run(&run)?;
                self.runs.push(run);
            }
            "--fallback-glyph" => self.parse_fallback(iter)?,
            "--glyph-line-drift" => {
                if self.line_drift_um.is_some() {
                    return Err("duplicate --glyph-line-drift".into());
                }
                self.line_drift_um = Some(next_integer(
                    iter, "glyph line drift", 0, 150,
                )?);
            }
            "--glyph-word-drift" => {
                if self.word_drift_um.is_some() {
                    return Err("duplicate --glyph-word-drift".into());
                }
                self.word_drift_um = Some(next_integer(
                    iter, "glyph word drift", 0, 150,
                )?);
            }
            "--glyph-word-rise" => {
                if self.word_rise_um.is_some() {
                    return Err("duplicate --glyph-word-rise".into());
                }
                self.word_rise_um = Some(next_integer(
                    iter, "glyph word rise", 0, 150,
                )?);
            }
            "--glyph-word-slant" => {
                if self.word_slant_millidegrees.is_some() {
                    return Err("duplicate --glyph-word-slant".into());
                }
                self.word_slant_millidegrees = Some(next_integer(
                    iter, "glyph word slant", 0, 5_000,
                )?);
            }
            "--glyph-word-width" => {
                if self.word_width_percent.is_some() {
                    return Err("duplicate --glyph-word-width".into());
                }
                self.word_width_percent = Some(next_integer(
                    iter, "glyph word width", 0, 15,
                )?);
            }
            "--glyph-word-height" => self.parse_height_rhythm(iter)?,
            "--glyph-word-tracking" => self.parse_tracking_rhythm(iter)?,
            "--glyph-word-pressure" => {
                if self.word_pressure_percent.is_some() {
                    return Err("duplicate --glyph-word-pressure".into());
                }
                self.word_pressure_percent = Some(next_integer(
                    iter, "glyph word pressure", 0, 20,
                )?);
            }
            "--glyph-variation" => {
                if self.variation.is_some() {
                    return Err("--glyph-variation may appear only once".into());
                }
                self.variation = Some(GlyphVariation::parse(iter)?);
            },
            _ => return Err(format!("unknown glyph export option: {flag}")),

        }
        Ok(())
    }

    fn parse_fallback(
        &mut self, iter: &mut Iter<'_, String>,
    ) -> Result<(), String> {
        if self.fallbacks.len() == MAX_FALLBACKS {
            return Err("more than 64 glyph fallbacks refused".into());
        }
        let missing = iter.next().ok_or(
            "--fallback-glyph requires MISSING AUTHORED"
        )?;
        let authored = iter.next().ok_or(
            "--fallback-glyph requires MISSING AUTHORED"
        )?;
        let boundaries = UnicodeGraphemeSegmentation;
        if !is_exact_visible_grapheme(missing, boundaries)
            || !is_exact_visible_grapheme(authored, boundaries)
            || missing == authored
        {
            return Err(
                "fallback requires two distinct exact visible graphemes".into()
            );
        }
        if self.fallbacks.iter()
            .any(|entry| entry.missing == *missing)
        {
            return Err("duplicate exact glyph fallback".into());
        }
        self.fallbacks.push(GlyphFallback {
            authored: authored.clone(), missing: missing.clone(),
        });
        Ok(())
    }

    fn parse_height_rhythm(
        &mut self, iter: &mut Iter<'_, String>,
    ) -> Result<(), String> {
        if self.word_height_percent.is_some() {
            return Err("duplicate --glyph-word-height".into());
        }
        self.word_height_percent = Some(next_integer(
            iter, "glyph word height", 0, 15,
        )?);
        Ok(())
    }

    /// Parse the same exact bounded shape for append or targeted replacement.
    fn parse_stroke(
        grapheme: &str, iter: &mut Iter<'_, String>,
    ) -> Result<GlyphStroke, String> {
        let boundaries = UnicodeGraphemeSegmentation;
        if !is_exact_visible_grapheme(grapheme, boundaries) {
            return Err(
                "glyph shape requires one exact visible grapheme".into()
            );
        }
        let x1 = next_integer(iter, "glyph x1", 0, 3_500)?;
        let y1 = next_integer(iter, "glyph y1", 0, 4_500)?;
        let x2 = next_integer(iter, "glyph x2", 0, 3_500)?;
        let y2 = next_integer(iter, "glyph y2", 0, 4_500)?;
        if x1 == x2 && y1 == y2 {
            return Err(concat!(
                "glyph stroke must have two ",
                "distinct points",
            ).into());
        }
        let width_um = next_integer(
            iter, "glyph ink width", 80, 1_500,
        )?;
        let color = iter.next().ok_or("glyph color requires #RRGGBB")?;
        if color.len() != 7 || !color.starts_with('#')
            || !color.bytes().skip(1).all(|byte| byte.is_ascii_hexdigit()) {
            return Err("glyph color requires #RRGGBB".into());
        }
        let channel = |index: usize| -> Result<u8, String> {
            u8::from_str_radix(
                color.get(index..index.saturating_add(2))
                    .ok_or("glyph color requires #RRGGBB")?,
                16,
            ).map_err(|_error| "glyph color requires #RRGGBB".into())
        };
        Ok(GlyphStroke {
            grapheme: grapheme.to_owned(),
            start_um: (x1, y1),
            end_um: (x2, y2),
            width_um,
            rgb: [channel(1)?, channel(3)?, channel(5)?],
        })
    }

    fn parse_tracking_rhythm(
        &mut self, iter: &mut Iter<'_, String>,
    ) -> Result<(), String> {
        if self.word_tracking_um.is_some() {
            return Err("duplicate --glyph-word-tracking".into());
        }
        self.word_tracking_um = Some(next_integer(
            iter, "glyph word tracking", 0, 150,
        )?);
        Ok(())
    }

    /// Admit typed edit operands without consulting an imported glyph file.
    /// The final edit still checks whether selected strokes or paths exist.
    pub(super) fn preflight_stroke_or_path_edit(
        flag: &str, iter: &mut Iter<'_, String>,
    ) -> Result<(), String> {
        let path = matches!(flag,
            "--reverse-path" | "--close-path" | "--rotate-path"
            | "--move-path" | "--translate-path",
        );
        let grapheme = iter.next().ok_or(if path {
            "path edit requires an exact grapheme"
        } else {
            "stroke edit requires an exact grapheme"
        })?;
        if path {
            let _from = next_integer(iter, "path index", 1, 4096)?;
            match flag {
                "--rotate-path" => {
                    let _start = next_integer(
                        iter, "path start stroke", 1, 4096,
                    )?;
                }
                "--move-path" => {
                    let _to = next_integer(iter, "path index", 1, 4096)?;
                }
                "--translate-path" => {
                    let dx = next_signed_path_offset(iter, "path dx", 3500)?;
                    let dy = next_signed_path_offset(iter, "path dy", 4500)?;
                    if dx == 0i32 && dy == 0i32 {
                        return Err(
                            "path shift needs nonzero bounded whole um".into()
                        );
                    }
                }
                "--reverse-path" | "--close-path" => {}
                _ => return Err("unknown path edit".into()),
            }
        } else {
            let _from = next_stroke_index(iter)?;
            match flag {
                "--move-stroke" => {
                    let _to = next_stroke_index(iter)?;
                }
                "--set-stroke" => {
                    let _stroke = Self::parse_stroke(grapheme, iter)?;
                }
                "--reverse-stroke" | "--join-strokes"
                | "--remove-stroke" => {}
                _ => return Err("unknown stroke edit".into()),
            }
        }
        if !is_exact_visible_grapheme(grapheme, UnicodeGraphemeSegmentation) {
            return Err(
                "stroke edit requires one exact visible grapheme".into()
            );
        }
        Ok(())
    }

    /// Parse independent transform numbers before reading a selected library.
    /// Actual transformed contour bounds still need imported glyph strokes.
    pub(super) fn preflight_transform_option(
        flag: &str, iter: &mut Iter<'_, String>,
    ) -> Result<(), String> {
        let grapheme = iter.next()
            .ok_or_else(|| format!("{flag} requires an exact grapheme"))?;
        match flag {
            "--rotate-glyph" => {
                let _angle = next_signed_path_offset(
                    iter, "rotation angle", 20,
                )?;
                let _pivot_x = next_integer(
                    iter, "rotation X pivot", 0, 3500,
                )?;
                let _pivot_y = next_integer(
                    iter, "rotation Y pivot", 0, 4500,
                )?;
            }
            "--translate-glyph" => {
                let _dx = next_signed_path_offset(iter, "glyph dx", 3500)?;
                let _dy = next_signed_path_offset(iter, "glyph dy", 4500)?;
            }
            "--shear-glyph" => {
                let _angle = next_integer(iter, "italic angle", 0, 20)?;
                let _baseline = next_integer(
                    iter, "italic baseline", 0, 4500,
                )?;
            }
            "--scale-glyph-height" => {
                let _percent = next_integer(
                    iter, "glyph height percent", 50, 150,
                )?;
                let _baseline = next_integer(
                    iter, "glyph height baseline", 0, 4500,
                )?;
            }
            "--scale-glyph-width" => {
                let _percent = next_integer(
                    iter, "glyph width percent", 50, 150,
                )?;
                let _anchor = next_integer(
                    iter, "glyph width anchor", 0, 3500,
                )?;
            }
            "--scale-glyph-ink" => {
                let _percent = next_integer(
                    iter, "glyph ink percent", 50, 150,
                )?;
            }
            _ => return Err("unknown glyph transformation".into()),
        }
        if !is_exact_visible_grapheme(grapheme, UnicodeGraphemeSegmentation) {
            return Err(
                "stroke edit requires one exact visible grapheme".into()
            );
        }
        Ok(())
    }

    /// Remove every exact template segment for one grapheme in the library.
    /// A missing exact match is a refusal, not a normalization or fallback.
    pub(super) fn remove_glyph(&mut self, grapheme: &str)
        -> Result<(), String> {
        validate_removal_glyph_key(grapheme)?;
        self.validate_stroke_library()?;
        let before = self.strokes.len();
        self.strokes.retain(|stroke| stroke.grapheme != grapheme);
        if self.strokes.len() == before {
            return Err("no authored strokes for exact glyph".into());
        }
        Ok(())
    }

    /// Delete one selected segment without disturbing any other glyph rows.
    pub(super) fn remove_stroke(
        &mut self, grapheme: &str, ordinal: u32,
    ) -> Result<(), String> {
        let positions = self.matching_stroke_positions(grapheme)?;
        let index = Self::checked_stroke_position(&positions, ordinal)?;
        let selected = *positions.get(index)
            .ok_or("stroke index outside bounds")?;
        if self.strokes.get(selected).is_none() {
            return Err("stroke index outside library".into());
        }
        let _removed = self.strokes.remove(selected);
        Ok(())
    }

    /// Reverse all segments of one continuous pen-down candidate path.
    /// A one-segment path remains the domain of --reverse-stroke.
    pub(super) fn reverse_path(
        &mut self, grapheme: &str, ordinal: u32,
    ) -> Result<(), String> {
        let (positions, paths) = self.selected_candidate_paths(grapheme)?;
        let index = checked_path_position(&paths, ordinal)?;
        let path = paths.get(index).ok_or("path index outside bounds")?;
        if path.len() < 2 {
            return Err("path reversal needs at least two segments".into());
        }
        let slots = positions.get(path.clone())
            .ok_or("path range outside glyph")?;
        let mut reversed = slots.iter().map(|position| {
            self.strokes.get(*position).cloned()
                .ok_or("path segment outside library")
        }).collect::<Result<Vec<_>, _>>()?;
        reversed.reverse();
        for stroke in &mut reversed {
            swap(&mut stroke.start_um, &mut stroke.end_um);
        }
        for (position, stroke) in slots.iter().zip(reversed) {
            *self.strokes.get_mut(*position)
                .ok_or("path segment outside library")? = stroke;
        }
        Ok(())
    }

    /// Reverse the physical pen travel of one segment without changing its
    /// position in the original TSV or its thickness/pigment.
    pub(super) fn reverse_stroke(
        &mut self, grapheme: &str, ordinal: u32,
    ) -> Result<(), String> {
        let positions = self.matching_stroke_positions(grapheme)?;
        let index = Self::checked_stroke_position(&positions, ordinal)?;
        let selected = *positions.get(index)
            .ok_or("stroke index outside bounds")?;
        let shape = self.strokes.get_mut(selected)
            .ok_or("stroke index outside library")?;
        swap(&mut shape.start_um, &mut shape.end_um);
        Ok(())
    }

    /// Rotate all ordered segments of an exact glyph clockwise in physical
    /// page coordinates, using browser-compatible integer rounding.
    /// Refuse the entire transformation before modifying any canonical ink.
    #[expect(clippy::suboptimal_flops, reason =
        "match browser angle-to-radians arithmetic before trig")]
    pub(super) fn rotate_glyph(
        &mut self, grapheme: &str, degrees: i32,
        pivot_x: u32, pivot_y: u32,
    ) -> Result<(), String> {
        if degrees.unsigned_abs() > 20 || pivot_x > 3500
            || pivot_y > 4500 {
            return Err("glyph rotation or pivot outside native bounds".into());
        }
        let positions = self.matching_stroke_positions(grapheme)?;
        if degrees == 0i32 { return Ok(()); }
        let radians = f64::from(degrees) * PI / 180.0f64;
        let (sin, cos) = radians.sin_cos();
        let mut revised = self.strokes.clone();
        for position in positions {
            let stroke = revised.get_mut(position)
                .ok_or("glyph stroke outside library")?;
            let rotate = |(x, y): (u32, u32)|
                -> Result<(u32, u32), String> {
                let relative_x = f64::from(x) - f64::from(pivot_x);
                let relative_y = f64::from(y) - f64::from(pivot_y);
                let next_x = GlyphVariation::round_js(
                    f64::from(pivot_x) + cos * relative_x
                        - sin * relative_y,
                )?;
                let next_y = GlyphVariation::round_js(
                    f64::from(pivot_y) + sin * relative_x
                        + cos * relative_y,
                )?;
                let pen = i64::from(stroke.width_um);
                let check = |value: i32, max: i64| -> Result<u32, String> {
                    let twice = i64::from(value).checked_mul(2)
                        .ok_or("rotated pen edge overflow")?;
                    let upper = max.checked_mul(2)
                        .and_then(|limit| limit.checked_sub(pen))
                        .ok_or("rotated pen limit overflow")?;
                    if twice < pen || twice > upper {
                        return Err(
                            "rotated pen contour clips glyph cell".into()
                        );
                    }
                    u32::try_from(value)
                        .map_err(|_error| {
                            "rotated coordinate out of bounds".into()
                        })
                };
                Ok((check(next_x, 3500)?, check(next_y, 4500)?))
            };
            let start = rotate(stroke.start_um)?;
            let end = rotate(stroke.end_um)?;
            if start == end {
                return Err("glyph rotation collapses a pen segment".into());
            }
            stroke.start_um = start;
            stroke.end_um = end;
        }
        let original = replace(&mut self.strokes, revised);
        if let Err(error) = self.encode_file() {
            self.strokes = original;
            return Err(error);
        }
        Ok(())
    }

    pub(super) fn rotate_glyph_option(
        &mut self, iter: &mut Iter<'_, String>,
    ) -> Result<(), String> {
        let grapheme = iter.next()
            .ok_or("--rotate-glyph requires an exact grapheme")?;
        let degrees = next_signed_path_offset(iter, "rotation angle", 20)?;
        let pivot_x = next_integer(iter, "rotation X pivot", 0, 3500)?;
        let pivot_y = next_integer(iter, "rotation Y pivot", 0, 4500)?;
        self.rotate_glyph(grapheme, degrees, pivot_x, pivot_y)
    }

    /// Choose a new first segment of a closed candidate pen loop without
    /// moving any geometry, changing direction, or touching other glyphs.
    /// The start is one-based *within* the selected candidate path.
    pub(super) fn rotate_path(
        &mut self, grapheme: &str, ordinal: u32, start: u32,
    ) -> Result<(), String> {
        let (positions, paths) = self.selected_candidate_paths(grapheme)?;
        let index = checked_path_position(&paths, ordinal)?;
        let path = paths.get(index).ok_or("path index outside bounds")?;
        let offset = usize::try_from(start.checked_sub(1)
            .ok_or("closed path start indices begin at 1")?)
            .map_err(|_error| "closed path start outside bounds")?;
        if path.len() < 2 || offset == 0 || offset >= path.len() {
            return Err(
                "closed path rotation needs a later existing stroke".into()
            );
        }
        let slots = positions.get(path.clone())
            .ok_or("path range outside glyph")?;
        let mut rotated = slots.iter().map(|position| {
            self.strokes.get(*position).cloned()
                .ok_or("path segment outside library")
        }).collect::<Result<Vec<_>, _>>()?;
        let first = rotated.first().ok_or("missing first path segment")?;
        let last = rotated.last().ok_or("missing last path segment")?;
        if first.start_um != last.end_um {
            return Err("path rotation requires a closed pen loop".into());
        }
        rotated.rotate_left(offset);
        for (position, stroke) in slots.iter().zip(rotated) {
            *self.strokes.get_mut(*position)
                .ok_or("path segment outside library")? = stroke;
        }
        Ok(())
    }

    /// Scale authored vertical coordinates around an explicit Y baseline.
    /// The entire selected glyph is admitted before any TSV row is changed;
    /// rounding follows the browser, including positive half micrometres.
    pub(super) fn scale_glyph_height(
        &mut self, grapheme: &str, percent: u32, baseline: u32,
    ) -> Result<(), String> {
        if !(50..=150).contains(&percent) || baseline > 4500 {
            return Err(
                "height percent or baseline outside native bounds".into()
            );
        }
        let positions = self.matching_stroke_positions(grapheme)?;
        let mut revised = self.strokes.clone();
        for position in positions {
            let stroke = revised.get_mut(position)
                .ok_or("glyph stroke outside library")?;
            let pen = i64::from(stroke.width_um);
            for x in [stroke.start_um.0, stroke.end_um.0] {
                let twice = i64::from(x).saturating_mul(2);
                if twice < pen || twice > 7000i64.saturating_sub(pen) {
                    return Err("resized ink and pen clip glyph cell".into());
                }
            }
            let resize_y = |y: u32| -> Result<u32, String> {
                let next = GlyphVariation::round_js(
                    f64::from(baseline) + (f64::from(y)
                        - f64::from(baseline)) * f64::from(percent) / 100.0,
                )?;
                let twice = i64::from(next).saturating_mul(2);
                if twice < i64::from(stroke.width_um)
                    || twice > 9000i64
                        .saturating_sub(i64::from(stroke.width_um)) {
                    return Err("resized ink and pen clip glyph cell".into());
                }
                u32::try_from(next)
                    .map_err(|_error| "resized coordinate out of bounds".into())
            };
            let start = resize_y(stroke.start_um.1)?;
            let end = resize_y(stroke.end_um.1)?;
            if stroke.start_um.0 == stroke.end_um.0 && start == end {
                return Err("height scaling collapses a pen segment".into());
            }
            stroke.start_um.1 = start;
            stroke.end_um.1 = end;
        }
        let original = replace(&mut self.strokes, revised);
        if let Err(error) = self.encode_file() {
            self.strokes = original;
            return Err(error);
        }
        Ok(())
    }

    pub(super) fn scale_glyph_height_option(
        &mut self, iter: &mut Iter<'_, String>,
    ) -> Result<(), String> {
        let grapheme = iter.next()
            .ok_or("--scale-glyph-height requires an exact grapheme")?;
        let percent = next_integer(iter, "glyph height percent", 50, 150)?;
        let baseline = next_integer(iter, "glyph height baseline", 0, 4500)?;
        self.scale_glyph_height(grapheme, percent, baseline)
    }

    /// Rescale the physical pen thickness of one exact authored grapheme.
    /// Endpoint positions, drawing order, colors and other glyphs do not move.
    /// Refuse the entire candidate if a thicker cap would clip the cell.
    pub(super) fn scale_glyph_ink(
        &mut self, grapheme: &str, percent: u32,
    ) -> Result<(), String> {
        if !(50..=150).contains(&percent) {
            return Err("ink percent outside native bounds".into());
        }
        let positions = self.matching_stroke_positions(grapheme)?;
        let mut revised = self.strokes.clone();
        for position in positions {
            let stroke = revised.get_mut(position)
                .ok_or("glyph stroke outside library")?;
            let width = u32::try_from(GlyphVariation::round_js(
                f64::from(stroke.width_um) * f64::from(percent) / 100.0,
            )?).map_err(|_error| "scaled pen width out of bounds")?;
            if !(80..=1_500).contains(&width) {
                return Err("scaled pen width outside 80..1500 um".into());
            }
            let pen = i64::from(width);
            for (x, y) in [stroke.start_um, stroke.end_um] {
                let twice_x = i64::from(x).saturating_mul(2);
                let twice_y = i64::from(y).saturating_mul(2);
                if twice_x < pen || twice_x > 7000i64.saturating_sub(pen)
                    || twice_y < pen
                    || twice_y > 9000i64.saturating_sub(pen) {
                    return Err("scaled ink and pen clip glyph cell".into());
                }
            }
            stroke.width_um = width;
        }
        let original = replace(&mut self.strokes, revised);
        if let Err(error) = self.encode_file() {
            self.strokes = original;
            return Err(error);
        }
        Ok(())
    }

    pub(super) fn scale_glyph_ink_option(
        &mut self, iter: &mut Iter<'_, String>,
    ) -> Result<(), String> {
        let grapheme = iter.next()
            .ok_or("--scale-glyph-ink requires an exact grapheme")?;
        let percent = next_integer(iter, "glyph ink percent", 50, 150)?;
        self.scale_glyph_ink(grapheme, percent)
    }

    /// Horizontally resize exactly one authored grapheme about an X anchor.
    /// Rejected candidates leave every existing v1 TSV row untouched.
    pub(super) fn scale_glyph_width(
        &mut self, grapheme: &str, percent: u32, anchor: u32,
    ) -> Result<(), String> {
        if !(50..=150).contains(&percent) || anchor > 3500 {
            return Err("width percent or anchor outside native bounds".into());
        }
        let positions = self.matching_stroke_positions(grapheme)?;
        let mut revised = self.strokes.clone();
        for position in positions {
            let stroke = revised.get_mut(position)
                .ok_or("glyph stroke outside library")?;
            let pen = i64::from(stroke.width_um);
            for y in [stroke.start_um.1, stroke.end_um.1] {
                let twice = i64::from(y).saturating_mul(2);
                if twice < pen || twice > 9000i64.saturating_sub(pen) {
                    return Err("resized ink and pen clip glyph cell".into());
                }
            }
            let resize_x = |x: u32| -> Result<u32, String> {
                let next = GlyphVariation::round_js(
                    f64::from(anchor) + (f64::from(x)
                        - f64::from(anchor)) * f64::from(percent) / 100.0,
                )?;
                let twice = i64::from(next).saturating_mul(2);
                if twice < pen || twice > 7000i64.saturating_sub(pen) {
                    return Err("resized ink and pen clip glyph cell".into());
                }
                u32::try_from(next)
                    .map_err(|_error| "resized coordinate out of bounds".into())
            };
            let start = resize_x(stroke.start_um.0)?;
            let end = resize_x(stroke.end_um.0)?;
            if stroke.start_um.1 == stroke.end_um.1 && start == end {
                return Err("width scaling collapses a pen segment".into());
            }
            stroke.start_um.0 = start;
            stroke.end_um.0 = end;
        }
        let original = replace(&mut self.strokes, revised);
        if let Err(error) = self.encode_file() {
            self.strokes = original;
            return Err(error);
        }
        Ok(())
    }

    pub(super) fn scale_glyph_width_option(
        &mut self, iter: &mut Iter<'_, String>,
    ) -> Result<(), String> {
        let grapheme = iter.next()
            .ok_or("--scale-glyph-width requires an exact grapheme")?;
        let percent = next_integer(iter, "glyph width percent", 50, 150)?;
        let anchor = next_integer(iter, "glyph width anchor", 0, 3500)?;
        self.scale_glyph_width(grapheme, percent, anchor)
    }

    /// Match the inspection view without folding NFC/NFD or allowing
    /// unrelated interleaved TSV rows to move.
    fn selected_candidate_paths(
        &self, grapheme: &str,
    ) -> Result<(Vec<usize>, Vec<Range<usize>>), String> {
        let positions = self.matching_stroke_positions(grapheme)?;
        let selected = positions.iter().map(|index| {
            self.strokes.get(*index).ok_or("path segment outside library")
        }).collect::<Result<Vec<_>, _>>()?;
        let paths = candidate_path_ranges(&selected);
        Ok((positions, paths))
    }

    /// Replace exactly one authored segment with newly validated geometry.
    pub(super) fn set_stroke(
        &mut self, grapheme: &str, ordinal: u32,
        replacement: GlyphStroke,
    ) -> Result<(), String> {
        if replacement.grapheme != grapheme {
            return Err(
                "replacement glyph identity must match selection".into()
            );
        }
        // A typed sibling can bypass CLI parse_stroke; use the same
        // serializable-shape admission as canonical TSV encoding.
        validate_serializable_stroke(&replacement)?;
        let positions = self.matching_stroke_positions(grapheme)?;
        let index = Self::checked_stroke_position(&positions, ordinal)?;
        let selected = *positions.get(index)
            .ok_or("stroke index outside bounds")?;
        let shape = self.strokes.get_mut(selected)
            .ok_or("stroke index outside library")?;
        *shape = replacement;
        Ok(())
    }

    /// Apply the same explicit, baseline-relative shear as the browser ink
    /// editor. The native v1 file retains sampled integer coordinates,
    /// not a slant property or a second named glyph variant.
    /// Preserve other Unicode rows and pen metadata, and refuse the entire
    /// edit before mutation when any selected endpoint would clip its pen.
    #[expect(clippy::suboptimal_flops, reason =
        "match the browser's angle-to-radians and coordinate rounding")]
    pub(super) fn shear_glyph(
        &mut self, grapheme: &str, degrees: u32, baseline: u32,
    ) -> Result<(), String> {
        if degrees > 20 || baseline > 4500 {
            return Err("italic angle or baseline outside native bounds".into());
        }
        let positions = self.matching_stroke_positions(grapheme)?;
        let slope = (f64::from(degrees) * PI
            / 180.0f64).tan();
        let mut revised = self.strokes.clone();
        for position in positions {
            let stroke = revised.get_mut(position)
                .ok_or("glyph stroke outside library")?;
            let shear_x = |x: u32, y: u32| -> Result<u32, String> {
                let shifted = GlyphVariation::round_js(
                    f64::from(x) + slope
                        * (f64::from(baseline) - f64::from(y)),
                )?;
                let twice = i64::from(shifted).saturating_mul(2);
                if twice < i64::from(stroke.width_um)
                    || twice > 7000i64
                        .saturating_sub(i64::from(stroke.width_um)) {
                    return Err("italic ink and pen clip glyph cell".into());
                }
                u32::try_from(shifted)
                    .map_err(|_error| "italic coordinate out of bounds".into())
            };
            let start = shear_x(stroke.start_um.0, stroke.start_um.1)?;
            let end = shear_x(stroke.end_um.0, stroke.end_um.1)?;
            stroke.start_um.0 = start;
            stroke.end_um.0 = end;
        }
        let original = replace(&mut self.strokes, revised);
        if let Err(error) = self.encode_file() {
            self.strokes = original;
            return Err(error);
        }
        Ok(())
    }

    pub(super) fn shear_glyph_option(
        &mut self, iter: &mut Iter<'_, String>,
    ) -> Result<(), String> {
        let grapheme = iter.next()
            .ok_or("--shear-glyph requires an exact grapheme")?;
        let degrees = next_integer(iter, "italic angle", 0, 20)?;
        let baseline = next_integer(iter, "italic baseline", 0, 4500)?;
        self.shear_glyph(grapheme, degrees, baseline)
    }

    /// Move every path of one exact authored glyph together. Refuse the
    /// entire candidate if any round pen cap clips the physical template.
    pub(super) fn translate_glyph(
        &mut self, grapheme: &str, dx: i32, dy: i32,
    ) -> Result<(), String> {
        if dx.unsigned_abs() > 3500 || dy.unsigned_abs() > 4500 {
            return Err("glyph shift outside signed native bounds".into());
        }
        let positions = self.matching_stroke_positions(grapheme)?;
        let mut revised = self.strokes.clone();
        for position in positions {
            let stroke = revised.get_mut(position)
                .ok_or("glyph stroke outside library")?;
            let move_axis = |value: u32, offset: i32, cell: i64|
                -> Result<u32, String> {
                let next = i64::from(value).checked_add(i64::from(offset))
                    .ok_or("translated coordinate overflow")?;
                let twice = next.checked_mul(2)
                    .ok_or("translated pen edge overflow")?;
                let pen = i64::from(stroke.width_um);
                let upper = cell.checked_mul(2)
                    .and_then(|limit| limit.checked_sub(pen))
                    .ok_or("translated pen limit overflow")?;
                if twice < pen || twice > upper {
                    return Err(
                        "translated pen contour clips glyph cell".into()
                    );
                }
                u32::try_from(next)
                    .map_err(|_error| {
                        "translated coordinate out of bounds".into()
                    })
            };
            stroke.start_um = (
                move_axis(stroke.start_um.0, dx, 3500)?,
                move_axis(stroke.start_um.1, dy, 4500)?,
            );
            stroke.end_um = (
                move_axis(stroke.end_um.0, dx, 3500)?,
                move_axis(stroke.end_um.1, dy, 4500)?,
            );
        }
        let original = replace(&mut self.strokes, revised);
        if let Err(error) = self.encode_file() {
            self.strokes = original;
            return Err(error);
        }
        Ok(())
    }

    pub(super) fn translate_glyph_option(
        &mut self, iter: &mut Iter<'_, String>,
    ) -> Result<(), String> {
        let grapheme = iter.next()
            .ok_or("--translate-glyph requires an exact grapheme")?;
        let dx = next_signed_path_offset(iter, "glyph dx", 3500)?;
        let dy = next_signed_path_offset(iter, "glyph dy", 4500)?;
        self.translate_glyph(grapheme, dx, dy)
    }

    /// Shift every directed segment in one exact candidate pen path,
    /// checking the full contour and explicit v1 output before commit.
    pub(super) fn translate_path(
        &mut self, grapheme: &str, ordinal: u32,
        dx: i32, dy: i32,
    ) -> Result<(), String> {
        if dx.unsigned_abs() > 3500 || dy.unsigned_abs() > 4500
            || (dx == 0i32 && dy == 0i32) {
            return Err("path shift needs nonzero bounded whole um".into());
        }
        let (positions, paths) = self.selected_candidate_paths(grapheme)?;
        let index = checked_path_position(&paths, ordinal)?;
        let path = paths.get(index).ok_or("path index outside bounds")?;
        let slots = positions.get(path.clone())
            .ok_or("path range outside glyph")?;
        // The pen's radius must be inside every cell edge, including odd
        // widths: twice the coordinate must be >= ink width.
        let translate = |value: u32, offset: i32, cell: i64, width: u32|
            -> Result<u32, String> {
            let moved = i64::from(value)
                .checked_add(i64::from(offset))
                .ok_or("translated coordinate overflow")?;
            let twice = moved.checked_mul(2)
                .ok_or("translated pen edge overflow")?;
            let maximum = cell.checked_mul(2)
                .and_then(|limit| limit.checked_sub(i64::from(width)))
                .ok_or("translated pen limit overflow")?;
            if twice < i64::from(width) || twice > maximum {
                return Err("translated pen contour clips glyph cell".into());
            }
            u32::try_from(moved)
                .map_err(|_error| "translated coordinate out of bounds".into())
        };
        let mut revised = self.strokes.clone();
        for selected in slots {
            let stroke = revised.get_mut(*selected)
                .ok_or("path segment outside library")?;
            stroke.start_um = (
                translate(stroke.start_um.0, dx, 3500, stroke.width_um)?,
                translate(stroke.start_um.1, dy, 4500, stroke.width_um)?,
            );
            stroke.end_um = (
                translate(stroke.end_um.0, dx, 3500, stroke.width_um)?,
                translate(stroke.end_um.1, dy, 4500, stroke.width_um)?,
            );
        }
        let original = replace(&mut self.strokes, revised);
        if let Err(error) = self.encode_file() {
            self.strokes = original;
            return Err(error);
        }
        Ok(())
    }

    pub(super) fn validate_optional_rhythm(&self) -> Result<(), String> {
        if let Some(maximum_um) = self.line_drift_um {
            if maximum_um > 150 {
                return Err("glyph line drift must be within 0..150 um".into());
            }
            if self.variation.is_none() {
                return Err("line drift requires --glyph-variation".into());
            }
        }
        if let Some(maximum_um) = self.word_drift_um {
            if maximum_um > 150 {
                return Err("glyph word drift must be within 0..150 um".into());
            }
            if self.variation.is_none() {
                return Err("word drift requires --glyph-variation".into());
            }
        }
        if let Some(maximum_um) = self.word_rise_um {
            if maximum_um > 150 {
                return Err("glyph word rise must be within 0..150 um".into());
            }
            if self.variation.is_none() {
                return Err("word rise requires --glyph-variation".into());
            }
        }
        if let Some(maximum) = self.word_slant_millidegrees {
            if maximum > 5_000 {
                return Err(
                    "glyph word slant must be within 0..5000 mdeg".into()
                );
            }
            if self.variation.is_none() {
                return Err("word slant requires --glyph-variation".into());
            }
        }
        if let Some(maximum) = self.word_width_percent {
            if maximum > 15 {
                return Err("glyph word width must be within 0..15%".into());
            }
            if self.variation.is_none() {
                return Err("word width requires --glyph-variation".into());
            }
        }
        if let Some(maximum_um) = self.word_tracking_um {
            if maximum_um > 150 {
                return Err(
                    "glyph word tracking must be within 0..150 um".into()
                );
            }
            if self.variation.is_none() {
                return Err("word tracking requires --glyph-variation".into());
            }
        }
        if let Some(maximum) = self.word_height_percent {
            if maximum > 15 {
                return Err("glyph word height must be within 0..15%".into());
            }
            if self.variation.is_none() {
                return Err("word height requires --glyph-variation".into());
            }
        }
        if let Some(maximum) = self.word_pressure_percent {
            if maximum > 20 {
                return Err("glyph word pressure must be within 0..20%".into());
            }
            if self.variation.is_none() {
                return Err("word pressure requires --glyph-variation".into());
            }
        }
        Ok(())
    }

    pub(super) fn validate_pairing(&self) -> Result<(), String> {
        self.validate_optional_rhythm()?;
        // A typed sibling may supply unused shapes directly. Reject the
        // complete invalid library before rendering just the selected text,
        // rather than silently overlooking malformed unselected strokes.
        self.validate_stroke_library()?;
        for run in &self.runs {
            validate_text_run(run)?;
        }
        if self.strokes.is_empty() != self.runs.is_empty() {
            return Err(concat!(
                "--glyph-stroke and --place-text require one another; ",
                "all visible graphemes need an authored shape",
            ).into());
        }
        if self.variation.is_some() && self.runs.is_empty() {
            return Err("--glyph-variation requires placed text".into());
        }
        if !self.text_joins.is_empty() && self.runs.is_empty() {
            return Err("--join-text-pair requires placed text".into());
        }
        // The CLI caps pairs on insertion, but a typed sibling can construct
        // the join list directly. Keep the same bounded export admission.
        if self.text_joins.len() > MAX_TEXT_JOIN_PAIRS {
            return Err("text joins limited to 32 selected pairs".into());
        }
        if !self.fallbacks.is_empty() && self.runs.is_empty() {
            return Err(concat!(
                "--fallback-glyph requires authored glyphs ",
                "and placed text",
            ).into());
        }
        // Sibling typed callers can change fallback entries without going
        // through --fallback-glyph parsing. Preserve exact Unicode identity
        // and deterministic single-owner mappings at export admission.
        if self.fallbacks.len() > MAX_FALLBACKS {
            return Err("more than 64 glyph fallbacks refused".into());
        }
        let boundaries = UnicodeGraphemeSegmentation;
        for (index, fallback) in self.fallbacks.iter().enumerate() {
            if !is_exact_visible_grapheme(&fallback.missing, boundaries)
                || !is_exact_visible_grapheme(&fallback.authored, boundaries)
                || fallback.missing == fallback.authored {
                return Err(
                    "fallback requires two distinct exact visible graphemes"
                        .into()
                );
            }
            if self.fallbacks.iter().skip(index.saturating_add(1))
                .any(|entry| entry.missing == fallback.missing) {
                return Err("duplicate exact glyph fallback".into());
            }
            if self.strokes.iter()
                .any(|stroke| stroke.grapheme == fallback.missing)
            {
                return Err(concat!(
                    "fallback source already has authored strokes; ",
                    "remove or use the exact source shape",
                ).into());
            }
            if self.fallbacks.iter()
                .any(|entry| entry.missing == fallback.authored)
            {
                return Err("chained glyph fallbacks refused".into());
            }
            if !self.strokes.iter()
                .any(|stroke| stroke.grapheme == fallback.authored)
            {
                return Err(format!(
                    "fallback target has no authored strokes: \"{}\"",
                    fallback.authored.escape_debug(),
                ));
            }
        }
        Ok(())
    }

    fn validate_seen_text_joins(&self, seen_pairs: &[bool])
    -> Result<(), String> {
        for (join, seen) in self.text_joins.iter().zip(seen_pairs) {
            if !seen {
                return Err(format!(
                    concat!(
                        "selected text join pair not adjacent in a run: ",
                        "\"{}\" \"{}\"",
                    ),
                    join.left, join.right,
                ));
            }
        }
        Ok(())
    }

    // Audit, serialization and rendering must agree about every authored
    // stroke, including unused rows injected by typed callers.
    fn validate_stroke_library(&self) -> Result<(), String> {
        if self.strokes.len() > MAX_STROKES {
            return Err("glyph library exceeds 4096 strokes".into());
        }
        for stroke in &self.strokes {
            validate_serializable_stroke(stroke)?;
        }
        Ok(())
    }
}

// Manual ink from a typed caller follows the same bounded admission as the
// CLI's explicit --ink-line and selected TSV source, before glyph expansion.
fn validate_manual_ink(manual: &[NotebookInkLine]) -> Result<(), String> {
    if manual.len() > MAX_NOTEBOOK_INK_LINES {
        return Err("manual ink exceeds 4096 total pen strokes".into());
    }
    for line in manual {
        super::validate_native_ink_line(line)?;
    }
    Ok(())
}

// Generated flow runs are measured separately; the CLI's 16-run cap only
// applies to the explicit suffix. Typed callers can bypass option parsing.
fn validate_run_boundary(total_runs: usize, flow_runs: usize)
-> Result<(), String> {
    let placed = total_runs.checked_sub(flow_runs)
        .ok_or("flow run boundary exceeds positioned text runs")?;
    if placed > MAX_RUNS {
        return Err("more than 16 placed text runs refused".into());
    }
    Ok(())
}

// Both explicit CLI placements and internally composed Markdown runs must
// obey the same physical-page, text and heading-ink boundaries. Typed callers
// cannot silently drop empty text or position ink on nonexistent pages.
fn validate_text_run(run: &TextRun) -> Result<(), String> {
    if !(1..=32).contains(&run.page)
        || run.origin_um.0 > 700_000 || run.origin_um.1 > 700_000 {
        return Err("placed text page or origin outside native bounds".into());
    }
    let count = UnicodeGraphemeSegmentation.grapheme_count(&run.text);
    if run.text.is_empty() || run.text.len() > 384
        || count > MAX_RUN_GRAPHEMES
        || run.text.chars().any(|ch| ch.is_control()
            || matches!(ch, '\u{2028}' | '\u{2029}'))
        || !run.text.chars().any(|ch| ch != ' ')
    {
        return Err(concat!(
            "placed text requires 1..96 graphemes, ",
            "at least one non-space, and no controls or ",
            "structural separators",
        ).into());
    }
    if run.heading_ink.is_some_and(|style| {
        !(100..=250).contains(&style.scale_percent)
    }) {
        return Err("heading ink scale must be 100..250".into());
    }
    Ok(())
}

// Typed sibling callers may construct strokes without CLI parsing. Keep
// canonical writer output loadable under its own exact TSV v1 admission.
fn validate_serializable_stroke(stroke: &GlyphStroke)
-> Result<(), String> {
    if !is_exact_visible_grapheme(
        &stroke.grapheme, UnicodeGraphemeSegmentation,
    ) || stroke.start_um == stroke.end_um
        || [stroke.start_um.0, stroke.end_um.0]
            .iter().any(|x| *x > 3500)
        || [stroke.start_um.1, stroke.end_um.1]
            .iter().any(|y| *y > 4500)
        || !(80..=1_500).contains(&stroke.width_um) {
        return Err("glyph stroke geometry outside native bounds".into());
    }
    Ok(())
}

fn validate_glyph_statistics(
    statistics: Option<&HandwritingStatisticsProfile>,
) -> Result<(), String> {
    if let Some(profile) = statistics {
        profile.validate(&UnicodeGraphemeSegmentation).map_err(|error| {
            format!("invalid glyph handwriting statistics: {error:?}")
        })?;
    }
    Ok(())
}

/// Consecutive matching pen segments form a candidate path; only the
/// concrete segment order is persisted in the current TSV v1.
fn candidate_path_ranges(strokes: &[&GlyphStroke]) -> Vec<Range<usize>> {
    if strokes.is_empty() {
        return Vec::new();
    }
    let mut paths = Vec::new();
    let mut start = 0usize;
    for (index, pair) in strokes.windows(2).enumerate() {
        let [before, after] = pair else { continue };
        // A directed pen path continues at the next segment's START.
        let endpoint = before.end_um;
        let next_start = after.start_um;
        if endpoint != next_start
            || before.width_um != after.width_um
            || before.rgb != after.rgb
        {
            let end = index.saturating_add(1);
            paths.push(start..end);
            start = end;
        }
    }
    paths.push(start..strokes.len());
    paths
}

fn checked_path_position(
    paths: &[Range<usize>], ordinal: u32,
) -> Result<usize, String> {
    let index = usize::try_from(ordinal.checked_sub(1)
        .ok_or("path indices start at 1")?)
        .map_err(|_error| "path index outside bounds")?;
    if index >= paths.len() {
        return Err(format!(
            "path index {ordinal} exceeds glyph's {} paths", paths.len(),
        ));
    }
    Ok(index)
}

fn append_path_summary(
    details: &mut String, strokes: &[&GlyphStroke], number: usize,
    start: usize, end: usize,
) -> Result<(), String> {
    let first = strokes.get(start).ok_or("missing first path segment")?;
    let last = strokes.get(end.saturating_sub(1))
        .ok_or("missing last path segment")?;
    let count = end.saturating_sub(start);
    let unit = if count == 1 { "segment" } else { "segments" };
    let closure = if count > 1 && first.start_um == last.end_um {
        " [closed loop]"
    } else {
        ""
    };
    let _written = writeln!(
        details,
        concat!(
            "Path {}: strokes {}..{} ({} {}) ",
            "({}, {}) -> ({}, {}) width {} um color #{:02X}{:02X}{:02X}{}",
        ),
        number, start.saturating_add(1), end, count, unit,
        first.start_um.0, first.start_um.1,
        last.end_um.0, last.end_um.1,
        first.width_um, first.rgb[0], first.rgb[1], first.rgb[2],
        closure,
    );
    Ok(())
}

// Imported source segments may be centerline-valid but physically clipped.
// A newly authored bridge must fit with its entire round pen, not just its
// two endpoints. Straight lines between admitted endpoints stay within the
// convex glyph cell, including an exact half-width tangent.
fn ensure_inserted_bridge_fits_cell(
    start: (u32, u32), end: (u32, u32), width: u32,
) -> Result<(), String> {
    let inside = |position: u32, limit: u32| {
        position.saturating_mul(2) >= width
            && position.saturating_mul(2)
                <= limit.saturating_mul(2).saturating_sub(width)
    };
    if [start, end].into_iter().any(|(x, y)| {
        !inside(x, 3500) || !inside(y, 4500)
    }) {
        return Err("inserted pen bridge clips glyph cell".into());
    }
    Ok(())
}

// A cluster made entirely of nonmarking scalars has no ink identity.
// Keep a nonmarking scalar attached to a visible base as an exact key.
const fn is_nonmarking_scalar(character: char) -> bool {
    matches!(character,
            '\u{00ad}' | '\u{034f}' | '\u{0600}'..='\u{0605}'
                | '\u{061c}' | '\u{06dd}' | '\u{070f}'
                | '\u{0890}'..='\u{0891}' | '\u{08e2}'
                | '\u{115f}'..='\u{1160}' | '\u{17b4}'..='\u{17b5}'
                | '\u{180b}'..='\u{180f}' | '\u{200b}'..='\u{200f}'
                | '\u{202a}'..='\u{202e}' | '\u{2060}'..='\u{206f}'
                | '\u{2800}' | '\u{3164}'
                | '\u{fe00}'..='\u{fe0f}' | '\u{feff}'
                | '\u{ffa0}' | '\u{fff0}'..='\u{fffb}'
                | '\u{110bd}' | '\u{110cd}'
                | '\u{13430}'..='\u{13442}'
                | '\u{13447}'..='\u{13455}'
                | '\u{1bca0}'..='\u{1bca3}'
                | '\u{1d173}'..='\u{1d17a}'
                | '\u{e0000}'..='\u{e0fff}')
}

// Unicode noncharacters have no interchange glyph identity. Reserve the
// FDD0..FDEF range and the final two scalar values of every plane.
fn is_unicode_noncharacter(character: char) -> bool {
    let scalar = u32::from(character);
    (0xfdd0..=0xfdef).contains(&scalar) || scalar & 0xfffe == 0xfffe
}

fn is_exact_visible_grapheme(
    grapheme: &str, boundaries: UnicodeGraphemeSegmentation,
) -> bool {
    !grapheme.is_empty() && grapheme.len() <= 64
        && !grapheme.chars().any(|ch| ch.is_control()
            || is_unicode_noncharacter(ch)
            || matches!(ch, '\u{2028}' | '\u{2029}'))
        && grapheme != " "
        // Refuse all-nonmarking clusters, not only standalone controls.
        // Joined emoji or visible glyphs with such marks keep exact keys.
        && !grapheme.chars().all(is_nonmarking_scalar)
        && boundaries.grapheme_count(grapheme) == 1
}

// Pure exact-key admission shared by standalone CLI preflight and final
// library-dependent copy/inspection operations. No Unicode normalization.
#[expect(
    clippy::redundant_pub_crate,
    reason = "CLI sibling module shares this exact-key validator",
)]
pub(super) fn validate_copy_glyph_keys(
    source: &str, target: &str,
) -> Result<(), String> {
    let boundaries = UnicodeGraphemeSegmentation;
    if !is_exact_visible_grapheme(source, boundaries)
        || !is_exact_visible_grapheme(target, boundaries)
        || source == target
    {
        return Err(
            "glyph copy needs two distinct exact visible graphemes".into()
        );
    }
    Ok(())
}

#[expect(
    clippy::redundant_pub_crate,
    reason = "CLI sibling module shares this exact-key validator",
)]
pub(super) fn validate_inspection_glyph_key(
    grapheme: &str,
) -> Result<(), String> {
    if !is_exact_visible_grapheme(grapheme, UnicodeGraphemeSegmentation) {
        return Err("inspection requires one exact visible grapheme".into());
    }
    Ok(())
}

#[expect(
    clippy::redundant_pub_crate,
    reason = "CLI sibling module shares this exact-key validator",
)]
pub(super) fn validate_removal_glyph_key(
    grapheme: &str,
) -> Result<(), String> {
    if !is_exact_visible_grapheme(grapheme, UnicodeGraphemeSegmentation) {
        return Err("glyph removal requires one exact visible grapheme".into());
    }
    Ok(())
}

// The renderer-independent Rust handwriting engine owns every physical
// transform; this CLI adapter maps admitted profile and output fields.
fn placed_ink(
    shape: &GlyphStroke, run: &TextRun, origin: (i64, i64),
    metrics: Option<GlyphMetrics>, word_style: WordInkStyle,
) -> Result<NotebookInkLine, String> {
    // Compose word pressure and heading treatment before delegating final
    // width admission to the renderer-independent physical pen engine.
    let ink_scale_percent = run.heading_ink.map_or(100, |style| {
        style.scale_percent
    }).checked_mul(word_style.pressure_percent)
        .and_then(|value| value.checked_add(50))
        .and_then(|value| value.checked_div(100))
        .ok_or("combined ink scale exceeds numeric bounds")?;
    // When uncalibrated, use the projection engine's exact reference cell
    // only if an actual word shear was selected. Zero stays on the legacy
    // no-dimensions path, preserving byte identity and authored geometry.
    let dimensions = match (
        metrics, word_style.slant_mdeg, word_style.width_percent,
        word_style.height_percent,
    ) {
        (None, 0i32, 100, 100) => None,
        (maybe_metrics, slant, width_percent, height_percent) => {
            let values = maybe_metrics.unwrap_or(GlyphMetrics {
                baseline_offset: 0, height: 4_500, ink_thickness: 300,
                letter_spacing: 0, slant: 0, width: 3_500,
            });
            Some(MeasuredGlyphDimensions {
                width_um: u32::try_from(
                    u64::from(values.width)
                        .checked_mul(u64::from(width_percent))
                        .and_then(|value| value.checked_add(50))
                        .and_then(|value| value.checked_div(100))
                        .ok_or("combined glyph word width overflow")?,
                ).map_err(|_error| "combined glyph word width overflow")?,
                height_um: u32::try_from(
                    u64::from(values.height)
                        .checked_mul(u64::from(height_percent))
                        .and_then(|value| value.checked_add(50))
                        .and_then(|value| value.checked_div(100))
                        .ok_or("combined glyph word height overflow")?,
                ).map_err(|_error| "combined glyph word height overflow")?,
                ink_thickness_um: values.ink_thickness,
                slant_millidegrees: values.slant.checked_add(slant)
                    .ok_or("combined glyph word slant exceeds numeric bounds")?,
            })
        }
    };
    // Retain the original lower baseline when letters become taller or
    // shorter; a word's chosen height is not an additional row offset.
    let projected_y = if let Some(values) = dimensions {
        origin.1.checked_add(i64::from(metrics.map_or(
            4_500, |mean| mean.height,
        )))
            .and_then(|value| value.checked_sub(i64::from(values.height_um)))
            .ok_or("word height baseline origin overflow")?
    } else {
        origin.1
    };
    let projected = project_ink_segment(AuthoredInkSegment {
        start_um: shape.start_um, end_um: shape.end_um,
        width_um: shape.width_um, rgb: shape.rgb,
    }, InkPlacement {
        page: run.page, origin_um: (origin.0, projected_y),
        dimensions,
        ink_scale_percent,
        color_override: run.heading_ink.and_then(|style| style.color),
    }).map_err(str::to_owned)?;
    Ok(NotebookInkLine {
        page: projected.page, start_um: projected.start_um,
        end_um: projected.end_um, width_um: projected.width_um,
        rgb: projected.rgb,
    })
}

// Respect the shared physical-ink budget even for newly added connectors.
fn append_text_ink(
    expanded: &mut Vec<NotebookInkLine>,
    rendered: &[NotebookInkLine],
    previous: Option<(&str, &[NotebookInkLine])>,
    grapheme: &str,
    text_joins: &[super::TextJoin],
    joined_pairs: &mut [bool],
) -> Result<(), String> {
    if let Some((left, prior)) = previous
        && let Some((pair_index, join)) = text_joins.iter()
            .enumerate().find(|(_, join)| {
                join.left == left && join.right == grapheme
            })
    {
        let (exit, entry) = join.anchors.unwrap_or((
            prior.len().checked_sub(1)
                .ok_or("text join needs an authored exit stroke")?, 0,
        ));
        let last = prior.get(exit)
            .ok_or("selected text join exit stroke outside glyph")?;
        let first = rendered.get(entry)
            .ok_or("selected text join entry stroke outside glyph")?;
        if let Some(bridge) = text_join_bridge(*last, *first)? {
            if expanded.len() == MAX_NOTEBOOK_INK_LINES {
                return Err("text exceeds 4096 total pen strokes".into());
            }
            expanded.push(bridge);
        }
        let observed = joined_pairs.get_mut(pair_index)
            .ok_or("text join pair index outside bounds")?;
        *observed = true;
    }
    for line in rendered {
        if expanded.len() == MAX_NOTEBOOK_INK_LINES {
            return Err("text exceeds 4096 total pen strokes".into());
        }
        expanded.push(*line);
    }
    Ok(())
}

// Bridges use the actual final placed endpoints, after seeded variation,
// statistics, slant, heading weights and colors. No source stroke is edited.
fn text_join_bridge(
    before: NotebookInkLine, after: NotebookInkLine,
) -> Result<Option<NotebookInkLine>, String> {
    let as_physical = |line: NotebookInkLine| PhysicalInkSegment {
        page: line.page, start_um: line.start_um, end_um: line.end_um,
        width_um: line.width_um, rgb: line.rgb,
    };
    connect_ink_segments(
        as_physical(before), as_physical(after), MAX_JOIN_GAP_UM,
    ).map(|bridge| bridge.map(|segment| NotebookInkLine {
        page: segment.page,
        start_um: segment.start_um, end_um: segment.end_um,
        width_um: segment.width_um, rgb: segment.rgb,
    })).map_err(str::to_owned)
}

fn missing_shapes_error(missing: &[&str]) -> String {
    let mut message = String::from("missing authored glyph shapes:");
    for grapheme in missing {
        let codepoints = grapheme.chars()
            .map(|ch| format!("U+{:04X}", u32::from(ch)))
            .collect::<Vec<_>>().join(" ");
        let _written = write!(
            message, " \"{}\" [{codepoints}]", grapheme.escape_debug(),
        );
    }
    message
}

/// A visible glyph can have at most the entire bounded library of strokes.
fn next_signed_path_offset(
    iter: &mut Iter<'_, String>, label: &str, maximum: i32,
) -> Result<i32, String> {
    let raw = iter.next()
        .ok_or_else(|| format!("{label} requires whole micrometres"))?;
    let digits = raw.strip_prefix('-').unwrap_or(raw);
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(format!("{label} needs a signed whole micrometre"));
    }
    let offset = raw.parse::<i32>()
        .map_err(|_error| format!("{label} outside native bounds"))?;
    if offset.unsigned_abs() > maximum.unsigned_abs() {
        return Err(format!("{label} outside native bounds"));
    }
    Ok(offset)
}

fn next_stroke_index(
    iter: &mut Iter<'_, String>,
) -> Result<u32, String> {
    next_integer(iter, "stroke index", 1, 4096)
}

fn next_integer(
    iter: &mut Iter<'_, String>, label: &str, minimum: u32, maximum: u32,
) -> Result<u32, String> {
    export_integer(
        iter.next().map(String::as_str), label, minimum, maximum,
    )
}

/// Read one exact Unicode grapheme from the remaining UTF-8 suffix.
/// Sharing this boundary with flow prevents quadratic indexed prefix scans.
#[expect(
    clippy::redundant_pub_crate,
    reason = "Sibling flow module shares Unicode boundary authority"
)]
pub(super) fn first_grapheme(text: &str) -> Result<(&str, &str), String> {
    let end = UnicodeGraphemeSegmentation.byte_offset(text, 1)
        .ok_or("flow grapheme boundary refused")?;
    let glyph = text.get(..end).filter(|span| !span.is_empty())
        .ok_or("flow grapheme span refused")?;
    let rest = text.get(end..).ok_or("flow grapheme suffix refused")?;
    Ok((glyph, rest))
}
