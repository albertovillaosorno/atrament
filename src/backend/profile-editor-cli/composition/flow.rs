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
//   - Bounded physical word wrapping and pagination of supplied text.
// - Must-Not:
//   - Invent glyph shapes, persist notebook text, or infer font metrics.
// - Allows:
//   - Inputs: Validated paper profile, authored glyphs, optional mean metrics.
//   - Outputs: Ordered page-local text runs and explicit heading/list marks.
//   - Side effects: None.
// - Split-When:
//   - A persisted semantic notebook and general flow compiler are integrated.
// - Merge-When:
//   - The Rust semantic notebook compiler replaces CLI-only text flow.
// - Summary:
//   - Lays out caller-authored text on physical pages before native export.
// - Description:
//   - Explicit plain lines or a small Markdown headings/lists subset.
// - Usage:
//   - atrament-profiles notebook --glyph-file ink.tsv --flow-markdown '# Title'
// - Defaults:
//   - No synthetic ink, implicit fallbacks, or hidden file writes.
//

//! A bounded text-to-physical-page bridge for the native CLI.

use std::mem::take;
use std::slice::Iter;

use atrament_handwriting_statistics::{Dimension, HandwritingStatisticsProfile};
use atrament_notebook_paper_cli::NotebookInkLine;
use atrament_notebook_profile::NotebookProfile;
use atrament_unicode_grapheme_boundary_port::GraphemeBoundaryProvider as _;
use atrament_unicode_grapheme_segmentation::UnicodeGraphemeSegmentation;

use super::glyph::segment_at;
use super::{GlyphOverlay, TextRun};

const MAX_FLOW_BYTES: usize = 4_096;
const MAX_FLOW_LINES: usize = 128;
const MAX_LINE_GRAPHEMES: usize = 96;
const MAX_PAGES: usize = 32;
const INSET_UM: u32 = 2_500;
const EXTRA_RIGHT_UM: u32 = 1_500;
const PARAGRAPH_GAP_UM: u32 = 4_000;
const DEFAULT_PITCH_UM: u32 = 9_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[expect(clippy::redundant_pub_crate, reason = "Sibling CLI module access")]
pub(super) enum FlowFormat {
    Markdown,
    Plain,
}

#[derive(Debug)]
#[expect(clippy::redundant_pub_crate, reason = "Sibling CLI module access")]
pub(super) struct FlowSource {
    pub(super) format: FlowFormat,
    pub(super) text: String,
}

impl FlowSource {
    /// Read a caller-chosen regular text file using the existing bounded
    /// profile file reader, without creating any implicit saved notebook.
    pub(super) fn from_file(
        format: FlowFormat, path: &str,
    ) -> Result<Self, String> {
        let bytes = super::read_notebook_input(path, MAX_FLOW_BYTES)?;
        let source = String::from_utf8(bytes)
            .map_err(|_error| "flow file must be UTF-8")?;
        Self::new(format, &source)
    }

    pub(super) fn new(
        format: FlowFormat, source: &str,
    ) -> Result<Self, String> {
        if source.is_empty()
            || source.len() > MAX_FLOW_BYTES
            || !source.chars().any(|ch| !ch.is_whitespace())
            || source
                .chars()
                .any(|ch| ch.is_control() && ch != '\n' && ch != '\r')
            || source.replace("\r\n", "").contains('\r')
        {
            return Err(concat!(
                "flow text needs 1..4096 UTF-8 bytes, LF/CRLF lines ",
                "and visible text without other control characters",
            )
            .into());
        }
        Ok(Self {
            format,
            text: source.replace("\r\n", "\n"),
        })
    }
}

/// CLI-only title decoration; immutable canonical glyph templates are
/// painted after any marker ink, preserving original handwriting strokes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum HeadingStyle {
    Marker,
    None,
    Underline,
}

#[derive(Clone, Copy, Debug, Default)]
#[expect(clippy::redundant_pub_crate, reason = "Sibling CLI module access")]
pub(super) struct HeadingDecoration {
    color: Option<[u8; 3]>,
    style: Option<HeadingStyle>,
}

impl HeadingDecoration {
    pub(super) const fn is_selected(self) -> bool {
        self.color.is_some() || self.style.is_some()
    }

    pub(super) fn parse(
        &mut self, flag: &str, iter: &mut Iter<'_, String>,
    ) -> Result<(), String> {
        let selected = iter.next().ok_or_else(|| {
            format!("{flag} requires a heading setting")
        })?;
        match flag {
            "--heading-color" => {
                if self.color.is_some() {
                    return Err("--heading-color may appear only once".into());
                }
                if selected.len() != 7 || !selected.is_ascii()
                    || !selected.starts_with('#') {
                    return Err("heading color must be #RRGGBB".into());
                }
                let byte = |start: usize| -> Result<u8, String> {
                    let digits = selected.get(start..start.saturating_add(2))
                        .ok_or("heading color must be #RRGGBB")?;
                    u8::from_str_radix(digits, 16).map_err(|_error| {
                        "heading color must be #RRGGBB".into()
                    })
                };
                self.color = Some([byte(1)?, byte(3)?, byte(5)?]);
            }
            "--heading-style" => {
                if self.style.is_some() {
                    return Err("--heading-style may appear only once".into());
                }
                self.style = Some(match selected.as_str() {
                    "marker" => HeadingStyle::Marker,
                    "none" => HeadingStyle::None,
                    "underline" => HeadingStyle::Underline,
                    _ => {
                        let message =
                            "heading style must be marker, underline or none";
                        return Err(message.into());
                    }
                });
            }
            _ => return Err("unknown heading setting".into()),
        }
        Ok(())
    }

    pub(super) const fn validate(self) -> Result<(), &'static str> {
        if matches!(self.style, Some(HeadingStyle::None))
            && self.color.is_some() {
            return Err("--heading-color cannot accompany --heading-style none");
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
enum LineStyle {
    Body,
    Bullet,
    Heading,
    Ordered,
}

struct Layout<'model> {
    bottom: u32,
    heading: HeadingDecoration,
    left: u32,
    lines: Vec<TextRun>,
    ornaments: Vec<NotebookInkLine>,
    page: usize,
    pitch: u32,
    right: u32,
    statistics: Option<&'model HandwritingStatisticsProfile>,
    top: u32,
    y: u32,
}

impl Layout<'_> {
    fn add_gap(&mut self) -> Result<(), String> {
        self.y = self
            .y
            .checked_add(PARAGRAPH_GAP_UM)
            .ok_or("paragraph gap overflow")?;
        Ok(())
    }

    fn begin_row(&mut self) -> Result<u32, String> {
        let usable_bottom = self
            .bottom
            .checked_sub(INSET_UM)
            .ok_or("flow has no bottom inset")?;
        if self
            .y
            .checked_add(self.pitch)
            .is_none_or(|end| end > usable_bottom)
        {
            self.page = self.page.checked_add(1)
                .ok_or("flow page count overflow")?;
            if self.page > MAX_PAGES {
                return Err("flow exceeds 32 physical pages".into());
            }
            self.y = self
                .top
                .checked_add(INSET_UM)
                .ok_or("flow top inset overflow")?;
        }
        if self
            .y
            .checked_add(self.pitch)
            .is_none_or(|end| end > usable_bottom)
        {
            return Err("flow line cannot fit printable vertical region".into());
        }
        Ok(self.y)
    }

    fn layout_line(
        &mut self, content: &str, style: LineStyle,
    ) -> Result<(), String> {
        let (start_x, cap) = self.line_measurements(style)?;
        let mut row = String::new();
        let mut width = 0u32;
        let mut count = 0usize;
        let mut first = true;
        let mut pending_spaces = 0usize;
        for (index, word) in content.split(' ').enumerate() {
            if index > 0 {
                pending_spaces = pending_spaces
                    .checked_add(1)
                    .ok_or("flow spaces overflow")?;
            }
            if word.is_empty() {
                continue;
            }
            let (glyphs, word_width) = measured_word(word, self.statistics)?;
            let spaces_um = u32::try_from(pending_spaces)
                .map_err(|_error| "flow spaces overflow")?
                .checked_mul(4_000)
                .ok_or("flow spaces overflow")?;
            if !row.is_empty()
                && (width
                    .checked_add(spaces_um)
                    .and_then(|value| value.checked_add(word_width))
                    .is_none_or(|value| value > cap)
                    || count
                        .checked_add(pending_spaces)
                        .and_then(|value| value.checked_add(glyphs.len()))
                        .is_none_or(|value| value > MAX_LINE_GRAPHEMES)
                    || row
                        .len()
                        .checked_add(pending_spaces)
                        .and_then(|value| value.checked_add(word.len()))
                        .is_none_or(|value| value > 384))
            {
                self.write_row(take(&mut row), start_x, width, style, first)?;
                first = false;
                width = 0;
                count = 0;
                pending_spaces = 0;
            }
            append_spaces(
                &mut row, &mut width, &mut count, pending_spaces, cap,
            )?;
            pending_spaces = 0;
            for (grapheme, advance, slot) in glyphs {
                if slot > cap {
                    return Err(
                        "glyph exceeds horizontal printable region".into()
                    );
                }
                if !row.is_empty()
                    && (width.checked_add(slot).is_none_or(|end| end > cap)
                        || count == MAX_LINE_GRAPHEMES
                        || row
                            .len()
                            .checked_add(grapheme.len())
                            .is_none_or(|length| length > 384))
                {
                    if matches!(style, LineStyle::Heading) {
                        return Err(
                            "heading exceeds one-line fixed region".into()
                        );
                    }
                    self.write_row(
                        take(&mut row), start_x, width, style, first,
                    )?;
                    first = false;
                    width = 0;
                    count = 0;
                }
                if grapheme.len() > 384 {
                    return Err("flow grapheme exceeds line byte limit".into());
                }
                row.push_str(grapheme);
                width = width
                    .checked_add(advance)
                    .ok_or("flow glyph advance overflow")?;
                count = count.checked_add(1).ok_or("flow line too long")?;
            }
        }
        if !row.is_empty() {
            if matches!(style, LineStyle::Heading) && !first {
                return Err("heading exceeds one-line fixed region".into());
            }
            self.write_row(row, start_x, width, style, first)?;
        }
        Ok(())
    }

    fn line_measurements(
        &self, style: LineStyle,
    ) -> Result<(u32, u32), String> {
        let start_x = self
            .left
            .checked_add(INSET_UM)
            .and_then(|x| {
                x.checked_add(if matches!(
                    style, LineStyle::Bullet | LineStyle::Ordered
                ) {
                    6_000
                } else {
                    0
                })
            })
            .ok_or("flow left inset overflow")?;
        let cap = self
            .right
            .checked_sub(EXTRA_RIGHT_UM)
            .and_then(|right| right.checked_sub(start_x))
            .ok_or("flow horizontal printable region too narrow")?;
        if cap < 4_000 {
            return Err("flow horizontal printable region too narrow".into());
        }
        Ok((start_x, cap))
    }

    fn write_row(
        &mut self,
        text: String,
        start_x: u32,
        extent: u32,
        style: LineStyle,
        first: bool,
    ) -> Result<(), String> {
        if text.is_empty() {
            return Ok(());
        }
        if self.lines.len() == MAX_FLOW_LINES {
            return Err("flow exceeds 128 measured lines".into());
        }
        let y = self.begin_row()?;
        if matches!(style, LineStyle::Heading) {
            let mark_end = start_x
                .checked_add(extent)
                .ok_or("heading ornament exceeds page")?;
            match self.heading.style.unwrap_or(HeadingStyle::Underline) {
                HeadingStyle::Underline => {
                    let mark_y = y.checked_add(self.pitch)
                        .and_then(|end| end.checked_sub(2_300))
                        .ok_or("heading underline position overflow")?;
                    self.ornaments.push(NotebookInkLine {
                        page: self.page,
                        start_um: (start_x, mark_y),
                        end_um: (mark_end, mark_y),
                        width_um: 550,
                        rgb: self.heading.color.unwrap_or([30, 48, 74]),
                    });
                }
                HeadingStyle::Marker => {
                    // Four overlapping 1.4-mm strokes create a visible
                    // pastel marker band behind the subsequent glyph ink.
                    // Existing Rust compositor width/clearance bounds apply.
                    for lift in [900u32, 1_900, 2_900, 3_900] {
                        let mark_y = y.checked_add(lift)
                            .ok_or("heading marker position overflow")?;
                        self.ornaments.push(NotebookInkLine {
                            page: self.page,
                            start_um: (start_x, mark_y),
                            end_um: (mark_end, mark_y),
                            width_um: 1_400,
                            rgb: self.heading.color.unwrap_or([255, 237, 159]),
                        });
                    }
                }
                HeadingStyle::None => {}
            }
        }
        if first && matches!(style, LineStyle::Bullet) {
            let x1 = start_x
                .checked_sub(4_400)
                .ok_or("bullet left inset overflow")?;
            let x2 = x1.checked_add(1_700).ok_or("bullet width overflow")?;
            let marker_y = y.checked_add(2_200)
                .ok_or("bullet height overflow")?;
            self.ornaments.push(NotebookInkLine {
                page: self.page,
                start_um: (x1, marker_y),
                end_um: (x2, marker_y),
                width_um: 450,
                rgb: [30, 48, 74],
            });
        }
        self.lines.push(TextRun {
            page: self.page,
            origin_um: (start_x, y),
            text,
        });
        self.y = y
            .checked_add(self.pitch)
            .ok_or("flow row advance overflow")?;
        Ok(())
    }
}

type MeasuredGrapheme<'text> = (&'text str, u32, u32);

fn append_spaces(
    row: &mut String,
    width: &mut u32,
    count: &mut usize,
    spaces: usize,
    cap: u32,
) -> Result<(), String> {
    if spaces == 0 {
        return Ok(());
    }
    let space_um = u32::try_from(spaces)
        .map_err(|_error| "flow indentation exceeds line region")?
        .checked_mul(4_000)
        .ok_or("flow spaces overflow")?;
    if width.checked_add(space_um).is_none_or(|value| value > cap)
        || count
            .checked_add(spaces)
            .is_none_or(|value| value > MAX_LINE_GRAPHEMES)
        || row
            .len()
            .checked_add(spaces)
            .is_none_or(|value| value > 384)
    {
        return Err("flow indentation exceeds line region".into());
    }
    row.push_str(&" ".repeat(spaces));
    *width = width.checked_add(space_um).ok_or("flow space overflow")?;
    *count = count.checked_add(spaces).ok_or("flow line too long")?;
    Ok(())
}

fn measured_word<'text>(
    word: &'text str,
    statistics: Option<&HandwritingStatisticsProfile>,
) -> Result<(Vec<MeasuredGrapheme<'text>>, u32), String> {
    let boundaries = UnicodeGraphemeSegmentation;
    let mut glyphs = Vec::with_capacity(boundaries.grapheme_count(word));
    let mut width = 0u32;
    for index in 0..boundaries.grapheme_count(word) {
        let grapheme = segment_at(boundaries, word, index)?;
        let (advance, outline) = GlyphOverlay::flow_advance_um(
            grapheme, statistics,
        )?;
        let slot = advance.max(
            outline
                .checked_add(800)
                .ok_or("glyph contour extent overflow")?,
        );
        glyphs.push((grapheme, advance, slot));
        width = width
            .checked_add(advance)
            .ok_or("flow word width overflow")?;
    }
    Ok((glyphs, width))
}

// Restrict Markdown ordered markers to at most nine ASCII digits followed
// by ". " or ") ". The number and delimiter remain authored text ink;
// only unordered bullets receive a synthetic short line ornament.
fn markdown_line(line: &str) -> (LineStyle, &str) {
    let raw = line.trim_end_matches(' ');
    for heading in ["### ", "## ", "# "] {
        if let Some(body) = line.strip_prefix(heading) {
            return (LineStyle::Heading, body.trim_end_matches(' '));
        }
    }
    if let Some(bullet) = line.strip_prefix("- ")
        .or_else(|| line.strip_prefix("* "))
    {
        return (LineStyle::Bullet, bullet.trim_end_matches(' '));
    }
    if let Some((marker, item)) = line.split_once(' ')
        && let Some(digits) = marker.strip_suffix('.')
            .or_else(|| marker.strip_suffix(')'))
        && (1..=9).contains(&digits.len())
        && digits.bytes().all(|digit| digit.is_ascii_digit())
    {
        return (LineStyle::Ordered,
            if item.trim().is_empty() { "" } else { raw });
    }
    (LineStyle::Body, raw)
}

/// Compile bounded visible text on the caller-selected native page profile.
/// This does not create persistent notebook semantic state.
#[expect(clippy::redundant_pub_crate, reason = "Sibling CLI module access")]
pub(super) fn layout_flow(
    source: &FlowSource,
    profile: &NotebookProfile,
    statistics: Option<&HandwritingStatisticsProfile>,
    decoration: HeadingDecoration,
) -> Result<(Vec<TextRun>, Vec<NotebookInkLine>, usize), String> {
    let (left, top, right, bottom) = profile.printable_rect_um()?;
    let boundaries = UnicodeGraphemeSegmentation;
    let mut pitch = DEFAULT_PITCH_UM;
    for index in 0..boundaries.grapheme_count(&source.text) {
        let grapheme = segment_at(boundaries, &source.text, index)?;
        if grapheme.chars().all(char::is_whitespace) {
            continue;
        }
        if let Some(stats) = statistics {
            let height = stats
                .statistics_for(grapheme, Dimension::Height)
                .ok_or("flow glyph height statistics missing")?
                .mean;
            let baseline = stats
                .statistics_for(grapheme, Dimension::BaselineOffset)
                .ok_or("flow baseline statistics missing")?
                .mean;
            let needed = height
                .checked_add(baseline.abs())
                .and_then(|combined| combined.checked_add(4_000))
                .ok_or("flow row height overflow")?;
            pitch = pitch.max(u32::try_from(needed)
                .map_err(|_error| "flow row height refused")?);
        }
    }
    let mut layout = Layout {
        heading: decoration,
        statistics,
        lines: Vec::new(),
        ornaments: Vec::new(),
        page: 1,
        y: top
            .checked_add(INSET_UM)
            .ok_or("flow top margin overflow")?,
        top,
        bottom,
        left,
        right,
        pitch,
    };
    // A Markdown newline is a soft paragraph wrap, unlike an explicit
    // line in plain-text mode. Two trailing spaces request a hard line
    // break. Structural blocks terminate a pending paragraph first.
    let mut paragraph = String::new();
    for input_line in source.text.lines() {
        let raw = input_line.trim_end_matches(' ');
        if raw.is_empty() {
            if !paragraph.is_empty() {
                layout.layout_line(&paragraph, LineStyle::Body)?;
                paragraph.clear();
            }
            layout.add_gap()?;
            continue;
        }
        let (style, body) = if source.format == FlowFormat::Markdown {
            markdown_line(input_line)
        } else {
            (LineStyle::Body, raw)
        };
        if body.trim().is_empty() {
            return Err("empty Markdown block refused".into());
        }
        if source.format == FlowFormat::Markdown
            && matches!(style, LineStyle::Body)
        {
            if !paragraph.is_empty() {
                paragraph.push(' ');
            }
            paragraph.push_str(body);
            if input_line.ends_with("  ") {
                layout.layout_line(&paragraph, LineStyle::Body)?;
                paragraph.clear();
            }
        } else {
            if !paragraph.is_empty() {
                layout.layout_line(&paragraph, LineStyle::Body)?;
                paragraph.clear();
            }
            layout.layout_line(body, style)?;
        }
    }
    if !paragraph.is_empty() {
        layout.layout_line(&paragraph, LineStyle::Body)?;
    }
    if layout.lines.is_empty() {
        return Err("flow has no renderable text".into());
    }
    Ok((layout.lines, layout.ornaments, layout.page))
}
