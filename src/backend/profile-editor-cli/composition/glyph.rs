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

use atrament_notebook_paper_cli::{
    MAX_NOTEBOOK_INK_LINES, NotebookInkLine,
};
use atrament_unicode_grapheme_boundary_port::GraphemeBoundaryProvider as _;
use atrament_unicode_grapheme_segmentation::UnicodeGraphemeSegmentation;

use super::{export_integer, GlyphOverlay, GlyphStroke, TextRun};

const ADVANCE_UM: u32 = 4_000;
const MAX_STROKES: usize = 256;
const MAX_RUNS: usize = 16;
const MAX_RUN_GRAPHEMES: usize = 96;

impl GlyphOverlay {
    pub(super) fn expand(
        &self, manual: &[NotebookInkLine],
    ) -> Result<Vec<NotebookInkLine>, String> {
        self.validate_pairing()?;
        let boundaries = UnicodeGraphemeSegmentation;
        let mut expanded = manual.to_vec();
        let mut missing = Vec::new();
        for run in &self.runs {
            for index in 0..boundaries.grapheme_count(&run.text) {
                let grapheme = segment_at(boundaries, &run.text, index)?;
                if grapheme == " " {
                    continue;
                }
                let matches = self.strokes.iter()
                    .filter(|shape| shape.grapheme == grapheme)
                    .collect::<Vec<_>>();
                if matches.is_empty() {
                    if !missing.contains(&grapheme) {
                        missing.push(grapheme);
                    }
                    continue;
                }
                let shift = u32::try_from(index)
                    .map_err(|error| format!("text length refused: {error}"))?
                    .checked_mul(ADVANCE_UM)
                    .ok_or("text horizontal offset overflow")?;
                let x = run.origin_um.0.checked_add(shift)
                    .ok_or("text x coordinate overflow")?;
                for shape in matches {
                    if expanded.len() == MAX_NOTEBOOK_INK_LINES {
                        return Err("text exceeds 512 total pen strokes".into());
                    }
                    let translate = |point: (u32, u32)| {
                        Some((x.checked_add(point.0)?,
                            run.origin_um.1.checked_add(point.1)?))
                    };
                    expanded.push(NotebookInkLine {
                        page: run.page,
                        start_um: translate(shape.start_um)
                            .ok_or("glyph start coordinate overflow")?,
                        end_um: translate(shape.end_um)
                            .ok_or("glyph end coordinate overflow")?,
                        width_um: shape.width_um,
                        rgb: shape.rgb,
                    });
                }
            }
        }
        if !missing.is_empty() {
            let mut message = String::from("missing authored glyph shapes:");
            for grapheme in missing {
                let codepoints = grapheme.chars()
                    .map(|ch| format!("U+{:04X}", u32::from(ch)))
                    .collect::<Vec<_>>().join(" ");
                let _written = write!(
                    message, " \"{}\" [{codepoints}]", grapheme.escape_debug(),
                );
            }
            return Err(message);
        }
        Ok(expanded)
    }

    pub(super) const fn has_any(&self) -> bool {
        !self.runs.is_empty() || !self.strokes.is_empty()
    }

    pub(super) fn parse(
        &mut self, flag: &str, iter: &mut Iter<'_, String>,
    ) -> Result<(), String> {
        match flag {
            "--glyph-stroke" => {
                if self.strokes.len() == MAX_STROKES {
                    return Err("more than 256 glyph strokes refused".into());
                }
                let grapheme = iter.next()
                    .ok_or("--glyph-stroke requires one grapheme")?;
                let boundaries = UnicodeGraphemeSegmentation;
                if grapheme.is_empty() || grapheme.len() > 64
                    || grapheme.chars().any(char::is_control)
                    || boundaries.grapheme_count(grapheme) != 1
                    || grapheme == " "
                {
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
                    page: usize::try_from(page).map_err(|error| {
                        format!("invalid text page: {error}")
                    })?,
                    origin_um: (x, y),
                    text: text.clone(),
                });
            }
            _ => return Err(format!("unknown glyph export option: {flag}")),
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
        Ok(())
    }
}

fn next_integer(
    iter: &mut Iter<'_, String>, label: &str, minimum: u32, maximum: u32,
) -> Result<u32, String> {
    export_integer(
        iter.next().map(String::as_str), label, minimum, maximum,
    )
}

fn segment_at(
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
