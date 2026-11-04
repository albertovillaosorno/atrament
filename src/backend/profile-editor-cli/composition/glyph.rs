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

use std::f64::consts::PI;
use std::fmt::Write as _;
use std::mem::{replace, swap};
use std::ops::Range;
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
const MAX_TEXT_JOIN_PAIRS: usize = 32;
// An explicit short pen bridge must not silently cross a wide glyph gap.
const MAX_JOIN_GAP_UM: u64 = 1_200;
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
    /// Select exactly which adjacent Unicode letters may receive physical
    /// pen bridges on output; the canonical glyph TSV remains unchanged.
    pub(super) fn add_text_join_pair(
        &mut self, iter: &mut Iter<'_, String>,
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
        if self.text_joins.iter().any(|(first, second)| {
            first == left && second == right
        }) {
            return Err("duplicate text join pair refused".into());
        }
        if self.text_joins.len() == MAX_TEXT_JOIN_PAIRS {
            return Err("text joins limited to 32 selected pairs".into());
        }
        self.text_joins.push((left.clone(), right.clone()));
        Ok(())
    }

    /// Inspect the actual authored-shape inventory for one bounded text run.
    /// An English/Spanish statistics baseline alone never implies ink.
    pub(super) fn audit_text(&self, text: &str)
        -> Result<Vec<u8>, String> {
        let mut seen = Vec::new();
        let mut missing = 0usize;
        let mut details = String::new();
        let mut remaining = text;
        while !remaining.is_empty() {
            let (grapheme, rest) = first_grapheme(remaining)?;
            remaining = rest;
            // Only ASCII spaces are implicit 4-mm blank advances in the
            // actual compositor. Other Unicode whitespace may be authored
            // visible ink and must appear in this exact coverage audit.
            // Control-character line boundaries are source structure.
            if grapheme == " " || grapheme.chars().all(char::is_control)
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
        let boundaries = UnicodeGraphemeSegmentation;
        if !is_exact_visible_grapheme(source, boundaries)
            || !is_exact_visible_grapheme(target, boundaries)
            || source == target
        {
            return Err(
                "glyph copy needs two distinct exact visible graphemes".into()
            );
        }
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
        let mut expanded = manual.to_vec();
        let mut missing = Vec::new();
        let mut occurrence = 0u32;
        let mut joined_pairs = vec![false; self.text_joins.len()];
        for run in &self.runs {
            let mut offset_um = 0i64;
            // A pen lift is implicit at every run/row or authored space.
            let mut previous: Option<(&str, NotebookInkLine)> = None;
            let mut remaining = run.text.as_str();
            while !remaining.is_empty() {
                let (grapheme, rest) = first_grapheme(remaining)?;
                remaining = rest;
                if grapheme == " " {
                    previous = None;
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
                    previous = None;
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
                let rendered = selected.iter().map(|shape| {
                    placed_ink(shape, run, (glyph_x, glyph_y), metrics)
                }).collect::<Result<Vec<_>, _>>()?;
                append_text_ink(
                    &mut expanded, &rendered, previous, grapheme,
                    &self.text_joins, &mut joined_pairs,
                )?;
                previous = rendered.last().copied()
                    .map(|line| (grapheme, line));
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
        for ((left, right), seen) in self.text_joins.iter()
            .zip(joined_pairs)
        {
            if !seen {
                return Err(format!(
                    concat!(
                        "selected text join pair not adjacent in a run: ",
                        "\"{}\" \"{}\"",
                    ),
                    left, right,
                ));
            }
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
            || !self.text_joins.is_empty()
    }

    /// Inspect consecutive directed pen segments as contiguous candidate
    /// paths; never assert persisted pen-down groups or inferred cursive.
    pub(super) fn inspect_paths(
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

    /// Select one-based authored stroke positions without normalizing the
    /// grapheme or disturbing interleaved records for other glyphs.
    fn matching_stroke_positions(&self, grapheme: &str)
        -> Result<Vec<usize>, String> {
        if !is_exact_visible_grapheme(grapheme, UnicodeGraphemeSegmentation) {
            return Err(
                "stroke edit requires one exact visible grapheme".into()
            );
        }
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
        Ok(GlyphStroke {
            grapheme: grapheme.to_owned(),
            start_um: (x1, y1),
            end_um: (x2, y2),
            width_um,
            rgb: [channel(1)?, channel(3)?, channel(5)?],
        })
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
        if !self.text_joins.is_empty() && self.runs.is_empty() {
            return Err("--join-text-pair requires placed text".into());
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

// Respect the shared physical-ink budget even for newly added connectors.
fn append_text_ink(
    expanded: &mut Vec<NotebookInkLine>,
    rendered: &[NotebookInkLine],
    previous: Option<(&str, NotebookInkLine)>,
    grapheme: &str,
    text_joins: &[(String, String)],
    joined_pairs: &mut [bool],
) -> Result<(), String> {
    if let Some((left, prior)) = previous
        && let Some(pair_index) = text_joins.iter()
            .position(|(first, second)| {
                first == left && second == grapheme
            })
    {
        let first = rendered.first()
            .ok_or("text join needs an authored first stroke")?;
        if let Some(bridge) = text_join_bridge(prior, *first)? {
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
    if before.page != after.page {
        return Err("text join cannot cross physical pages".into());
    }
    if before.rgb != after.rgb || before.width_um != after.width_um {
        return Err("text join requires matching physical pen and RGB".into());
    }
    let (start, end) = (before.end_um, after.start_um);
    if start == end {
        return Ok(None);
    }
    let dx = i64::from(start.0).saturating_sub(i64::from(end.0));
    let dy = i64::from(start.1).saturating_sub(i64::from(end.1));
    if dx.unsigned_abs().pow(2).saturating_add(dy.unsigned_abs().pow(2))
        > MAX_JOIN_GAP_UM.pow(2)
    {
        return Err("text join gap exceeds 1200 um".into());
    }
    Ok(Some(NotebookInkLine {
        page: before.page, start_um: start, end_um: end,
        width_um: before.width_um, rgb: before.rgb,
    }))
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
