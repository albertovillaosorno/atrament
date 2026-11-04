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

use std::fmt::Write as _;
use std::slice::Iter;
use std::str::from_utf8;

use atrament_handwriting_statistics::{
    Dimension, HandwritingStatisticsProfile,
};
use atrament_notebook_paper_cli::{
    MAX_NOTEBOOK_INK_LINES, NotebookInkLine,
};
use atrament_unicode_grapheme_boundary_port::GraphemeBoundaryProvider as _;
use atrament_unicode_grapheme_segmentation::UnicodeGraphemeSegmentation;

use super::{
    export_integer, GlyphFallback, GlyphOverlay, GlyphStroke, TextRun,
    MAX_GLYPH_LIBRARY_BYTES,
};

const ADVANCE_UM: u32 = 4_000;
const MAX_STROKES: usize = 4096;
const MAX_RUNS: usize = 16;
const MAX_RUN_GRAPHEMES: usize = 96;
const MAX_FALLBACKS: usize = 64;
const GLYPH_LIBRARY_HEADER: &str = "atrament.glyph-strokes.v1";

// Authored local coordinates are only scaled when a separate saved
// statistical profile is explicitly selected by the caller.
// A configured slant additionally shears the upper contours in physical units.
const DEFAULT_TEMPLATE_WIDTH_UM: u32 = 3_500;
const DEFAULT_TEMPLATE_HEIGHT_UM: u32 = 4_500;
const DEFAULT_TEMPLATE_INK_UM: u32 = 300;

/// Opt-in creative jitter, not an inferred distribution or saved profile.
/// Units and the seeded xorshift match the browser glyph comparison.
#[derive(Clone, Copy, Debug)]
#[expect(clippy::redundant_pub_crate, reason = "CLI sibling module shares this type")]
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
                    return Err("varied ink and pen must fit inside glyph cell".into());
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

impl GlyphOverlay {
    /// Inspect the actual authored-shape inventory for one bounded text run.
    /// An English/Spanish statistics baseline alone never implies ink.
    pub(super) fn audit_text(&self, text: &str)
        -> Result<Vec<u8>, String> {
        let boundaries = UnicodeGraphemeSegmentation;
        let mut seen = Vec::new();
        let mut missing = 0usize;
        let mut details = String::new();
        for index in 0..boundaries.grapheme_count(text) {
            let grapheme = segment_at(boundaries, text, index)?;
            if grapheme.chars().all(char::is_whitespace)
                || seen.contains(&grapheme) {
                continue;
            }
            seen.push(grapheme);
            let strokes = self.strokes.iter()
                .filter(|stroke| stroke.grapheme == grapheme).count();
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

    /// Canonical portable glyph geometry, not placed text or sampled ink.
    pub(super) fn encode_file(&self) -> Result<Vec<u8>, String> {
        if self.strokes.is_empty() {
            return Err(
                "glyph library needs at least one authored stroke".into()
            );
        }
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

    pub(super) fn expand(
        &self, manual: &[NotebookInkLine],
        statistics: Option<&HandwritingStatisticsProfile>,
    ) -> Result<Vec<NotebookInkLine>, String> {
        self.validate_pairing()?;
        let boundaries = UnicodeGraphemeSegmentation;
        let mut expanded = manual.to_vec();
        let mut missing = Vec::new();
        let mut occurrence = 0u32;
        for run in &self.runs {
            let mut offset_um = 0i64;
            for index in 0..boundaries.grapheme_count(&run.text) {
                let grapheme = segment_at(boundaries, &run.text, index)?;
                if grapheme == " " {
                    offset_um = offset_um.checked_add(i64::from(ADVANCE_UM))
                        .ok_or("space advance overflow")?;
                    continue;
                }
                let authored = self.fallbacks.iter()
                    .find(|entry| entry.missing == grapheme)
                    .map_or(grapheme, |entry| entry.authored.as_str());
                let matches = self.strokes.iter()
                    .filter(|shape| shape.grapheme == authored)
                    .collect::<Vec<_>>();
                if matches.is_empty() {
                    if !missing.contains(&grapheme) {
                        missing.push(grapheme);
                    }
                    continue;
                }
                let metrics = statistics
                    .map(|profile| {
                        GlyphMetrics::from_profile(profile, grapheme)
                    })
                    .transpose()?;
                let tracking = self.variation.map_or(Ok(0i32), |variation| {
                    variation.tracking(occurrence)
                })?;
                let selected = if let Some(variation) = self.variation {
                    variation.sample(&matches, occurrence)?
                } else {
                    matches.iter().map(|shape| (*shape).clone()).collect()
                };
                let glyph_x = i64::from(run.origin_um.0)
                    .checked_add(offset_um)
                    .and_then(|value| value.checked_add(i64::from(tracking)))
                    .ok_or("glyph horizontal origin overflow")?;
                let glyph_y = i64::from(run.origin_um.1)
                    .checked_add(i64::from(metrics.map_or(
                        0i32, |values| values.baseline_offset,
                    )))
                    .ok_or("glyph vertical origin overflow")?;
                for shape in &selected {
                    if expanded.len() == MAX_NOTEBOOK_INK_LINES {
                        return Err(
                            "text exceeds 4096 total pen strokes".into()
                        );
                    }
                    expanded.push(placed_ink(
                        shape, run, (glyph_x, glyph_y), metrics,
                    )?);
                }
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
            }
        }
        if !missing.is_empty() {
            return Err(missing_shapes_error(&missing));
        }
        Ok(expanded)
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
            return Err("glyph width plus letter spacing must be positive".into());
        }
        Ok((u32::try_from(advance)
            .map_err(|_error| "glyph letter advance overflow")?,
            metrics.width))
    }

    pub(super) const fn has_any(&self) -> bool {
        !self.runs.is_empty() || !self.strokes.is_empty()
            || !self.fallbacks.is_empty() || self.variation.is_some()
    }

    /// Show the exact caller-authored order and direction without writing
    /// a file, inferring joins, normalizing Unicode or sampling variation.
    pub(super) fn inspect_stroke_order(
        &self, grapheme: &str,
    ) -> Result<Vec<u8>, String> {
        if !is_exact_visible_grapheme(grapheme, UnicodeGraphemeSegmentation) {
            return Err("inspection requires one exact visible grapheme".into());
        }
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

    /// Load a reusable exact-Unicode library with tab-separated columns.
    /// Grapheme identities are never implicitly normalized or substituted.
    pub(super) fn load_file_bytes(
        &mut self, bytes: &[u8],
    ) -> Result<(), String> {
        if self.loaded_file || self.has_any() {
            return Err(concat!(
                "--glyph-file or --profile must precede authored strokes ",
                "and may appear only once",
            )
            .into());
        }
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
        for (index, line) in lines.enumerate() {
            if self.strokes.len() == MAX_STROKES {
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
            self.parse("--glyph-stroke", &mut fields.iter())
                .map_err(|error| format!(
                    "glyph library line {}: {error}",
                    index.saturating_add(2),
                ))?;
        }
        self.loaded_file = true;
        Ok(())
    }

    pub(super) fn parse(
        &mut self, flag: &str, iter: &mut Iter<'_, String>,
    ) -> Result<(), String> {
        match flag {
            "--glyph-stroke" => {
                if self.strokes.len() == MAX_STROKES {
                    return Err("more than 4096 glyph strokes refused".into());
                }
                let grapheme = iter.next()
                    .ok_or("--glyph-stroke requires one grapheme")?;
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
                if color.len() != 7 || !color.starts_with('#') {
                    return Err("glyph color requires #RRGGBB".into());
                }
                let channel = |index: usize| -> Result<u8, String> {
                    u8::from_str_radix(
                        color.get(index..index.saturating_add(2))
                            .ok_or("glyph color requires #RRGGBB")?,
                        16,
                    ).map_err(|_error| "glyph color requires #RRGGBB".into())
                };
                self.strokes.push(GlyphStroke {
                    grapheme: grapheme.clone(),
                    start_um: (x1, y1),
                    end_um: (x2, y2),
                    width_um,
                    rgb: [channel(1)?, channel(3)?, channel(5)?],
                });
            }
            "--place-text" => {
                if self.runs.len() == MAX_RUNS {
                    return Err("more than 16 placed text runs refused".into());
                }
                let page = next_integer(iter, "text page", 1, 32)?;
                let x = next_integer(iter, "text x", 0, 700_000)?;
                let y = next_integer(iter, "text y", 0, 700_000)?;
                let text = iter.next().ok_or("--place-text needs UTF-8 text")?;
                let boundaries = UnicodeGraphemeSegmentation;
                let count = boundaries.grapheme_count(text);
                if text.is_empty() || text.len() > 384
                    || count > MAX_RUN_GRAPHEMES
                    || text.chars().any(char::is_control)
                    || !text.chars().any(|ch| ch != ' ')
                {
                    return Err(concat!(
                        "placed text requires 1..96 graphemes, ",
                        "at least one non-space, and no control characters",
                    ).into());
                }
                self.runs.push(TextRun {
                    heading_ink: None,
                    page: usize::try_from(page).map_err(|error| {
                        format!("invalid text page: {error}")
                    })?,
                    origin_um: (x, y),
                    text: text.clone(),
                });
            }
            "--fallback-glyph" => self.parse_fallback(iter)?,
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

    /// Remove every exact template segment for one grapheme in the library.
    /// A missing exact match is a refusal, not a normalization or fallback.
    pub(super) fn remove_glyph(&mut self, grapheme: &str)
        -> Result<(), String> {
        let before = self.strokes.len();
        self.strokes.retain(|stroke| stroke.grapheme != grapheme);
        if self.strokes.len() == before {
            return Err("no authored strokes for exact glyph".into());
        }
        Ok(())
    }

    pub(super) fn validate_pairing(&self) -> Result<(), String> {
        if self.strokes.is_empty() != self.runs.is_empty() {
            return Err(concat!(
                "--glyph-stroke and --place-text require one another; ",
                "all visible graphemes need an authored shape",
            ).into());
        }
        if self.variation.is_some() && self.runs.is_empty() {
            return Err("--glyph-variation requires placed text".into());
        }
        if !self.fallbacks.is_empty() && self.runs.is_empty() {
            return Err(concat!(
                "--fallback-glyph requires authored glyphs ",
                "and placed text",
            ).into());
        }
        for fallback in &self.fallbacks {
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
}

fn is_exact_visible_grapheme(
    grapheme: &str, boundaries: UnicodeGraphemeSegmentation,
) -> bool {
    !grapheme.is_empty() && grapheme.len() <= 64
        && !grapheme.chars().any(char::is_control)
        && grapheme != " "
        && boundaries.grapheme_count(grapheme) == 1
}

fn glyph_point(
    glyph_x: i64, glyph_y: i64, point: (u32, u32),
    metrics: Option<GlyphMetrics>,
) -> Result<(u32, u32), String> {
    let (x, y) = if let Some(values) = metrics {
        (
            scale(point.0, values.width, DEFAULT_TEMPLATE_WIDTH_UM)?,
            scale(point.1, values.height, DEFAULT_TEMPLATE_HEIGHT_UM)?,
        )
    } else {
        point
    };
    let x_on_page = if let Some(values) = metrics {
        slanted_x(glyph_x, x, y, values.height, values.slant)?
    } else {
        coordinate(glyph_x, x)?
    };
    Ok((x_on_page, coordinate(glyph_y, y)?))
}

// Lean the upper edge of each caller-authored shape around its lower
// baseline. Positive millidegrees lean right; negative values lean left.
// Refuse nonfinite or unprintable positions rather than silently clipping.
fn slanted_x(
    origin_um: i64, local_x: u32, local_y: u32,
    height: u32, slant_millidegrees: i32,
) -> Result<u32, String> {
    let radians = (f64::from(slant_millidegrees) / 1_000.0).to_radians();
    let delta = (f64::from(height) - f64::from(local_y)) * radians.tan();
    if !delta.is_finite() || delta.abs() > f64::from(i32::MAX) {
        return Err("glyph slant exceeds coordinate limits".into());
    }
    #[expect(clippy::as_conversions, clippy::cast_possible_truncation, reason =
        "bounded millidegree shear rounded to a whole micrometre")]
    let shift_um = delta.round() as i32;
    let absolute_um = origin_um
        .checked_add(i64::from(local_x))
        .and_then(|x| x.checked_add(i64::from(shift_um)))
        .ok_or("slanted glyph x position overflow")?;
    u32::try_from(absolute_um)
        .map_err(|_error| "slanted glyph must stay on page".into())
}

fn coordinate(origin_um: i64, local_um: u32) -> Result<u32, String> {
    let translated = origin_um.checked_add(i64::from(local_um))
        .ok_or("glyph placement overflow")?;
    u32::try_from(translated)
        .map_err(|_error| {
            "glyph placement needs nonnegative coordinates".into()
        })
}

// Keep title lettering styling as the last, export-only ink treatment:
// authored glyph coordinates, colors and dimensions never mutate.
fn placed_ink(
    shape: &GlyphStroke, run: &TextRun, origin: (i64, i64),
    metrics: Option<GlyphMetrics>,
) -> Result<NotebookInkLine, String> {
    let translate = |point| glyph_point(origin.0, origin.1, point, metrics);
    let mean_width = if let Some(values) = metrics {
        scale(shape.width_um, values.ink_thickness, DEFAULT_TEMPLATE_INK_UM)?
    } else {
        shape.width_um
    };
    let width_um = if let Some(style) = run.heading_ink {
        scale(mean_width, style.scale_percent, 100)?
    } else {
        mean_width
    };
    if !(80..=1_500).contains(&width_um) {
        return Err("scaled glyph ink width exceeds 80..1500 um".into());
    }
    Ok(NotebookInkLine {
        page: run.page,
        start_um: translate(shape.start_um)?,
        end_um: translate(shape.end_um)?,
        width_um,
        rgb: run.heading_ink.and_then(|style| style.color)
            .unwrap_or(shape.rgb),
    })
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

fn next_integer(
    iter: &mut Iter<'_, String>, label: &str, minimum: u32, maximum: u32,
) -> Result<u32, String> {
    export_integer(
        iter.next().map(String::as_str), label, minimum, maximum,
    )
}

fn scale(local_um: u32, target_um: u32, reference_um: u32)
    -> Result<u32, String> {
    let scaled = u64::from(local_um)
        .checked_mul(u64::from(target_um))
        .ok_or("glyph scale overflow")?
        .checked_add(u64::from(reference_um.checked_div(2)
            .ok_or("glyph scale reference is zero")?))
        .ok_or("glyph rounding overflow")?
        .checked_div(u64::from(reference_um))
        .ok_or("glyph scale denominator is zero")?;
    u32::try_from(scaled).map_err(|_error| "glyph scale too large".into())
}

#[expect(
    clippy::redundant_pub_crate,
    reason = "Sibling flow module shares Unicode boundary authority"
)]
pub(super) fn segment_at(
    boundaries: UnicodeGraphemeSegmentation,
    text: &str,
    index: usize,
) -> Result<&str, String> {
    let start = boundaries.byte_offset(text, index)
        .ok_or("glyph start boundary unavailable")?;
    let end = boundaries.byte_offset(
        text, index.checked_add(1).ok_or("glyph index overflow")?,
    ).ok_or("glyph end boundary unavailable")?;
    text.get(start..end).ok_or_else(|| "invalid glyph UTF-8 boundary".into())
}
