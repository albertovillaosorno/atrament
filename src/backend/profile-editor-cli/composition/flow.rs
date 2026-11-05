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

use std::borrow::Cow;
use std::iter::Peekable;
use std::mem::take;
use std::slice::Iter;

use atrament_handwriting_statistics::{Dimension, HandwritingStatisticsProfile};
use atrament_notebook_paper_cli::NotebookInkLine;
use atrament_notebook_profile::NotebookProfile;

use super::glyph::first_grapheme;
use super::{GlyphOverlay, HeadingInk, TextRun};

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

/// Optional absolute, page-local rectangle repeated on each physical sheet.
/// The native saved paper profile is still the authority for usable margins.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[expect(clippy::redundant_pub_crate, reason = "Sibling CLI module access")]
pub(super) struct FlowRegion {
    pub(super) height: u32,
    pub(super) left: u32,
    pub(super) top: u32,
    pub(super) width: u32,
}

impl FlowRegion {
    fn bounds_within(
        self, printable: (u32, u32, u32, u32),
    ) -> Result<(u32, u32, u32, u32), String> {
        let right = self.left.checked_add(self.width)
            .ok_or("flow region right edge overflows micrometres")?;
        let bottom = self.top.checked_add(self.height)
            .ok_or("flow region bottom edge overflows micrometres")?;
        if self.left < printable.0 || self.top < printable.1
            || right > printable.2 || bottom > printable.3 {
            return Err(
                "flow region must fit within printable paper margins".into()
            );
        }
        Ok((self.left, self.top, right, bottom))
    }

    pub(super) fn parse(iter: &mut Iter<'_, String>) -> Result<Self, String> {
        let mut values = [0; 4];
        for (value, name) in values.iter_mut()
            .zip(["left", "top", "width", "height"]) {
            let raw = iter.next().ok_or(
                "--flow-region requires LEFT TOP WIDTH HEIGHT in um"
            )?;
            let digits = !raw.is_empty()
                && raw.bytes().all(|byte| byte.is_ascii_digit());
            if !digits {
                return Err(format!("flow region {name} must be unsigned um"));
            }
            *value = raw.parse::<u32>().map_err(|_error| {
                format!("flow region {name} exceeds u32 micrometres")
            })?;
        }
        let [left, top, width, height] = values;
        if width == 0 || height == 0 {
            return Err("flow region width and height must be positive".into());
        }
        Ok(Self { height, left, top, width })
    }

    /// Layout measures nominal advances. Validate the actual expanded ink
    /// after slant, seeded variation and pen width transforms as well.
    pub(super) fn validate_ink(
        self, printable: (u32, u32, u32, u32),
        ink: &[NotebookInkLine],
    ) -> Result<(), String> {
        let (left, top, right, bottom) = self.bounds_within(printable)?;
        for line in ink {
            let clearance = line.width_um.div_ceil(2);
            let inside = |(x, y): (u32, u32)| {
                x.checked_sub(clearance).is_some_and(|edge| edge >= left)
                    && y.checked_sub(clearance)
                        .is_some_and(|edge| edge >= top)
                    && x.checked_add(clearance)
                        .is_some_and(|edge| edge <= right)
                    && y.checked_add(clearance)
                        .is_some_and(|edge| edge <= bottom)
            };
            if !inside(line.start_um) || !inside(line.end_um) {
                return Err(format!(
                    "flow region ink exceeds selected rectangle on page {}",
                    line.page,
                ));
            }
        }
        Ok(())
    }
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
        // A leading UTF-8 signature is transport metadata, not ink.
        // Keep the byte cap on the *original* input, including its BOM.
        let admitted = source.strip_prefix('\u{feff}').unwrap_or(source);
        if admitted.is_empty()
            || source.len() > MAX_FLOW_BYTES
            || !admitted.chars().any(|ch| !matches!(ch,
                ' ' | '\t' | '\n' | '\r' | '\u{2028}' | '\u{2029}'))
            || admitted
                .chars()
                .any(|ch| ch.is_control()
                    && !matches!(ch, '\n' | '\r' | '\t'))
            || admitted.replace("\r\n", "").contains('\r')
        {
            return Err(concat!(
                "flow text needs 1..4096 UTF-8 bytes, LF/CRLF lines ",
                "and visible text without other control characters",
            )
            .into());
        }
        // Only ASCII spaces and source line separators are implicit
        // spacing. Other Unicode whitespace may have exact authored pen
        // strokes, so it must survive admission and visible-block checks.
        // A pasted tab advances to the next four-grapheme column, never
        // inventing a visible glyph. Preserve NFC/NFD identities: one
        // decomposed grapheme occupies one column just like its NFC form.
        // Check the expanded bytes as well as the original source bound.
        // Unicode line/paragraph separators are layout boundaries in
        // pasted sources, not missing handwriting glyphs.
        // A Unicode line separator is an explicit hard break (two
        // trailing Markdown spaces). Plain flow ignores those spaces.
        // A paragraph separator reserves a blank row/paragraph gap.
        let normalized = admitted.replace("\r\n", "\n")
            .replace('\u{2028}', "  \n")
            .replace('\u{2029}', "\n\n");
        // Sources without tabs cannot grow after newline normalization.
        // The usual path needs no extra Unicode-boundary traversal.
        if !normalized.contains('\t') {
            return Ok(Self { format, text: normalized });
        }
        let mut remaining = normalized.as_str();
        let mut expanded = String::with_capacity(normalized.len());
        let mut column = 0usize;
        while !remaining.is_empty() {
            // Advance one grapheme at a time from the current suffix.
            // Querying ordinal indices from the full source would
            // repeatedly rescan its prefix (quadratic on long input).
            let (grapheme, rest) = first_grapheme(remaining)?;
            remaining = rest;
            match grapheme {
                "\n" => {
                    expanded.push('\n');
                    column = 0;
                }
                "\t" => {
                    let remainder = column.checked_rem(4)
                        .ok_or("tab column remainder refused")?;
                    let spaces = 4usize.checked_sub(remainder)
                        .ok_or("tab column width refused")?;
                    for _ in 0..spaces {
                        expanded.push(' ');
                    }
                    column = column.checked_add(spaces)
                        .ok_or("tab column overflow")?;
                }
                visible => {
                    expanded.push_str(visible);
                    column = column.checked_add(1)
                        .ok_or("tab column overflow")?;
                }
            }
            if expanded.len() > MAX_FLOW_BYTES {
                return Err("expanded flow text exceeds 4096 UTF-8 bytes"
                    .into());
            }
        }
        Ok(Self { format, text: expanded })
    }
}

#[cfg(test)]
#[path = "../../../../tests/backend/profile-editor-cli/composition/flow.rs"]
mod source_tests;

/// CLI-only title decoration; immutable canonical glyph templates are
/// painted after any marker ink, preserving original handwriting strokes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum HeadingStyle {
    DoubleUnderline,
    Marker,
    None,
    Underline,
}

#[derive(Clone, Copy, Debug, Default)]
#[expect(clippy::redundant_pub_crate, reason = "Sibling CLI module access")]
pub(super) struct HeadingDecoration {
    color: Option<[u8; 3]>,
    glyph_color: Option<[u8; 3]>,
    glyph_scale: Option<u32>,
    marker_scale: Option<u32>,
    style: Option<HeadingStyle>,
}

impl HeadingDecoration {
    const fn glyph_ink(self) -> Option<HeadingInk> {
        if self.glyph_color.is_some() || self.glyph_scale.is_some() {
            Some(HeadingInk {
                color: self.glyph_color,
                scale_percent: match self.glyph_scale {
                    Some(scale) => scale,
                    None => 100,
                },
            })
        } else {
            None
        }
    }

    pub(super) const fn is_selected(self) -> bool {
        self.color.is_some() || self.style.is_some()
            || self.glyph_color.is_some() || self.glyph_scale.is_some()
            || self.marker_scale.is_some()
    }

    pub(super) fn parse(
        &mut self, flag: &str, iter: &mut Iter<'_, String>,
    ) -> Result<(), String> {
        let selected = iter.next().ok_or_else(|| {
            format!("{flag} requires a heading setting")
        })?;
        match flag {
            "--heading-color" | "--heading-ink-color" => {
                let destination = if flag == "--heading-color" {
                    &mut self.color
                } else {
                    &mut self.glyph_color
                };
                if destination.is_some() {
                    return Err(format!("{flag} may appear only once"));
                }
                if selected.len() != 7 || !selected.is_ascii()
                    || !selected.starts_with('#') {
                    return Err(format!("{flag} requires #RRGGBB"));
                }
                let byte = |start: usize| -> Result<u8, String> {
                    let digits = selected.get(start..start.saturating_add(2))
                        .ok_or_else(|| format!("{flag} requires #RRGGBB"))?;
                    u8::from_str_radix(digits, 16)
                        .map_err(|_error| format!("{flag} requires #RRGGBB"))
                };
                *destination = Some([byte(1)?, byte(3)?, byte(5)?]);
            }
            "--heading-ink-scale" => {
                if self.glyph_scale.is_some() {
                    return Err("--heading-ink-scale repeated".into());
                }
                if selected.is_empty()
                    || !selected.bytes().all(|byte| byte.is_ascii_digit()) {
                    return Err("heading ink scale must be 100..250".into());
                }
                let scale = selected.parse::<u32>().map_err(|_error| {
                    "heading ink scale must be 100..250 percent"
                })?;
                if !(100..=250).contains(&scale) {
                    return Err("heading ink scale must be 100..250".into());
                }
                self.glyph_scale = Some(scale);
            }
            "--heading-marker-scale" => {
                if self.marker_scale.is_some() {
                    return Err("--heading-marker-scale repeated".into());
                }
                if selected.is_empty()
                    || !selected.bytes().all(|byte| byte.is_ascii_digit()) {
                    return Err("heading marker scale must be 50..105".into());
                }
                let scale = selected.parse::<u32>().map_err(|_error| {
                    "heading marker scale must be 50..105"
                })?;
                if !(50..=105).contains(&scale) {
                    return Err("heading marker scale must be 50..105".into());
                }
                self.marker_scale = Some(scale);
            }
            "--heading-style" => {
                if self.style.is_some() {
                    return Err("--heading-style may appear only once".into());
                }
                self.style = Some(match selected.as_str() {
                    "double-underline" => HeadingStyle::DoubleUnderline,
                    "marker" => HeadingStyle::Marker,
                    "none" => HeadingStyle::None,
                    "underline" => HeadingStyle::Underline,
                    _ => {
                        let message =
                            concat!(
                                "heading style must be double-underline, ",
                                "marker, underline or none",
                            );
                        return Err(message.into());
                    }
                });
            }
            _ => return Err("unknown heading setting".into()),
        }
        Ok(())
    }

    pub(super) const fn validate(self) -> Result<(), &'static str> {
        if self.marker_scale.is_some()
            && !matches!(self.style, Some(HeadingStyle::Marker)) {
            return Err(
                "--heading-marker-scale requires --heading-style marker",
            );
        }
        if matches!(self.style, Some(HeadingStyle::None))
            && self.color.is_some() {
            return Err("--heading-color cannot accompany --heading-style none");
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
enum HeadingLevel {
    Primary,
    Quaternary,
    Quinary,
    Secondary,
    Senary,
    Tertiary,
}

impl HeadingLevel {
    const fn marker_lifts_um(self) -> &'static [u32] {
        match self {
            Self::Primary => &[900, 1_900, 2_900, 3_900],
            Self::Secondary => &[1_000, 2_000, 3_000],
            Self::Tertiary => &[1_400, 2_400],
            Self::Quaternary => &[1_600, 2_500],
            Self::Quinary => &[2_000],
            Self::Senary => &[2_800],
        }
    }

    const fn marker_width_um(self) -> u32 {
        match self {
            Self::Primary => 1_400,
            Self::Secondary => 1_150,
            Self::Tertiary => 950,
            Self::Quaternary => 760,
            Self::Quinary => 620,
            Self::Senary => 500,
        }
    }

    const fn underline_width_um(self) -> u32 {
        match self {
            Self::Primary => 550,
            Self::Secondary => 420,
            Self::Tertiary => 300,
            Self::Quaternary => 240,
            Self::Quinary => 180,
            Self::Senary => 120,
        }
    }
}

#[derive(Clone, Copy)]
enum LineStyle {
    Body,
    Bullet,
    Heading(HeadingLevel),
    Ordered,
    Quote,
    Separator,
    Task(bool),
}

// Paragraph whitespace collapses to one 4-mm gap, but blank fenced
// lines reserve full physical rows once followed by actual ink. Retain
// both measurements in the heading orphan guard rather than treating a
// blank code row as if it were paragraph spacing.
#[derive(Clone, Copy)]
struct FollowingInkSpacing {
    blank_rows: usize,
    heading_rows: usize,
    paragraph_gaps: usize,
}

struct Layout<'model> {
    bottom: u32,
    heading: HeadingDecoration,
    left: u32,
    lines: Vec<TextRun>,
    ornaments: Vec<NotebookInkLine>,
    page: usize,
    pending_blank_rows: usize,
    pitch: u32,
    right: u32,
    row_count: usize,
    statistics: Option<&'model HandwritingStatisticsProfile>,
    top: u32,
    y: u32,
}

impl Layout<'_> {
    fn add_gap(&mut self) -> Result<(), String> {
        self.flush_blank_rows()?;
        self.y = self
            .y
            .checked_add(PARAGRAPH_GAP_UM)
            .ok_or("paragraph gap overflow")?;
        Ok(())
    }

    fn begin_row(&mut self) -> Result<u32, String> {
        self.flush_blank_rows()?;
        self.consume_row()
    }

    fn blank_row(&mut self) -> Result<(), String> {
        self.pending_blank_rows = self.pending_blank_rows
            .checked_add(1).ok_or("flow blank-row count overflow")?;
        Ok(())
    }

    fn consume_row(&mut self) -> Result<u32, String> {
        if self.row_count == MAX_FLOW_LINES {
            return Err("flow exceeds 128 measured lines".into());
        }
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
        self.row_count = self.row_count.checked_add(1)
            .ok_or("flow row count overflow")?;
        Ok(self.y)
    }

    // Delay empty physical rows until real ink follows them. This keeps
    // deliberate interior spacing without exporting blank trailing pages
    // or exhausting line/page budgets on invisible end-of-source rows.
    fn flush_blank_rows(&mut self) -> Result<(), String> {
        let pending = take(&mut self.pending_blank_rows);
        for _ in 0..pending {
            let y = self.consume_row()?;
            self.y = y.checked_add(self.pitch)
                .ok_or("flow blank-row advance overflow")?;
        }
        Ok(())
    }

    fn keep_heading_with_next(
        &mut self, following: FollowingInkSpacing,
    ) -> Result<(), String> {
        self.flush_blank_rows()?;
        let top = self.top.checked_add(INSET_UM)
            .ok_or("heading top inset overflow")?;
        let bottom = self.bottom.checked_sub(INSET_UM)
            .ok_or("heading bottom inset overflow")?;
        let rows = u32::try_from(following.blank_rows)
            .map_err(|_error| "heading blank-row budget overflow")?
            .checked_add(u32::try_from(following.heading_rows)
                .map_err(|_error| "heading chain-row budget overflow")?)
            .and_then(|rows| rows.checked_add(2))
            .ok_or("heading following-row budget overflow")?;
        let gaps = u32::try_from(following.paragraph_gaps)
            .map_err(|_error| "heading paragraph-gap budget overflow")?
            .checked_mul(PARAGRAPH_GAP_UM)
            .ok_or("heading paragraph-gap budget overflow")?;
        let needed = self.pitch.checked_mul(rows)
            .and_then(|value| value.checked_add(gaps))
            .ok_or("heading following-row budget overflow")?;
        // If one sheet cannot hold this full span, fall back to normal
        // pagination instead of refusing valid short paper outright.
        if top.checked_add(needed).is_none_or(|end| end > bottom) {
            return Ok(());
        }
        if self.y.checked_add(needed).is_none_or(|end| end > bottom) {
            self.page = self.page.checked_add(1)
                .ok_or("flow page count overflow")?;
            if self.page > MAX_PAGES {
                return Err("flow exceeds 32 physical pages".into());
            }
            self.y = top;
        }
        Ok(())
    }

    fn layout_line(
        &mut self, content: &str, style: LineStyle,
    ) -> Result<(), String> {
        let (mut start_x, mut cap) = self.line_measurements(style)?;
        let (hang, ordered_marker) = if matches!(style, LineStyle::Ordered) {
            let (indent, marker) = measured_ordered_label(
                content, cap, self.statistics,
            )?;
            (indent, Some(marker))
        } else {
            (0, None)
        };
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
                // Keep the first item glyph attached to its authored label
                // even when the first word itself needs grapheme wrapping.
                && !(first && ordered_marker == Some(row.as_str()))
            {
                self.write_row(take(&mut row), start_x, width, style, first)?;
                start_continuation(first, &mut start_x, &mut cap, hang)?;
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
                    if matches!(style, LineStyle::Heading(_)) {
                        return Err(
                            "heading exceeds one-line fixed region".into()
                        );
                    }
                    self.write_row(
                        take(&mut row), start_x, width, style, first,
                    )?;
                    start_continuation(first, &mut start_x, &mut cap, hang)?;
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
            if matches!(style, LineStyle::Heading(_)) && !first {
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
                    style,
                    LineStyle::Bullet | LineStyle::Ordered | LineStyle::Quote
                        | LineStyle::Task(_)
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

    fn write_list_marker(
        &mut self, style: LineStyle, first: bool,
        start_x: u32, y: u32,
    ) -> Result<(), String> {
        if !first {
            return Ok(());
        }
        if let LineStyle::Task(checked) = style {
            return self.write_task_box(start_x, y, checked);
        }
        if matches!(style, LineStyle::Bullet) {
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
        Ok(())
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
        // Literal leading indentation may fill an entire narrow row.
        // Never consume a page row with only spaces before the first
        // visible glyph; reject that unrenderable placement explicitly.
        if text.bytes().all(|byte| byte == b' ') {
            return Err(
                "flow indentation cannot fit first visible glyph".into()
            );
        }
        let y = self.begin_row()?;
        if let LineStyle::Heading(level) = style {
            let mark_end = start_x
                .checked_add(extent)
                .ok_or("heading ornament exceeds page")?;
            match self.heading.style.unwrap_or(HeadingStyle::Underline) {
                HeadingStyle::DoubleUnderline | HeadingStyle::Underline => {
                    let mark_y = y.checked_add(self.pitch)
                        .and_then(|end| end.checked_sub(2_300))
                        .ok_or("heading underline position overflow")?;
                    if self.heading.style
                        == Some(HeadingStyle::DoubleUnderline) {
                        let upper = mark_y.checked_sub(900)
                            .ok_or("double underline position overflow")?;
                        self.ornaments.push(NotebookInkLine {
                            page: self.page,
                            start_um: (start_x, upper),
                            end_um: (mark_end, upper),
                            width_um: level.underline_width_um(),
                            rgb: self.heading.color.unwrap_or([30, 48, 74]),
                        });
                    }
                    self.ornaments.push(NotebookInkLine {
                        page: self.page,
                        start_um: (start_x, mark_y),
                        end_um: (mark_end, mark_y),
                        width_um: level.underline_width_um(),
                        rgb: self.heading.color.unwrap_or([30, 48, 74]),
                    });
                }
                HeadingStyle::Marker => {
                    // Successively finer strokes distinguish all six ATX
                    // levels without altering the authored glyph shapes.
                    // Existing compositor width/clearance bounds still apply.
                    let width = level.marker_width_um()
                        .checked_mul(self.heading.marker_scale.unwrap_or(100))
                        .and_then(|scaled| scaled.checked_div(100))
                        .ok_or("heading marker width overflow")?;
                    for &lift in level.marker_lifts_um() {
                        let mark_y = y.checked_add(lift)
                            .ok_or("heading marker position overflow")?;
                        self.ornaments.push(NotebookInkLine {
                            page: self.page,
                            start_um: (start_x, mark_y),
                            end_um: (mark_end, mark_y),
                            width_um: width,
                            rgb: self.heading.color.unwrap_or([255, 237, 159]),
                        });
                    }
                }
                HeadingStyle::None => {}
            }
        }
        if matches!(style, LineStyle::Quote) {
            // Every physical continuation row retains a quiet quote bar.
            // The literal Markdown > is syntax, never an invented glyph.
            let rule_x = start_x.checked_sub(3_000)
                .ok_or("quote bar left inset overflow")?;
            let rule_top = y.checked_add(500)
                .ok_or("quote bar top overflow")?;
            let rule_bottom = y.checked_add(self.pitch)
                .and_then(|end| end.checked_sub(600))
                .ok_or("quote bar bottom overflow")?;
            self.ornaments.push(NotebookInkLine {
                page: self.page,
                start_um: (rule_x, rule_top),
                end_um: (rule_x, rule_bottom),
                width_um: 400,
                rgb: [30, 48, 74],
            });
        }
        self.write_list_marker(style, first, start_x, y)?;
        self.lines.push(TextRun {
            heading_ink: if matches!(style, LineStyle::Heading(_)) {
                self.heading.glyph_ink()
            } else {
                None
            },
            page: self.page,
            origin_um: (start_x, y),
            text,
        });
        self.y = y
            .checked_add(self.pitch)
            .ok_or("flow row advance overflow")?;
        Ok(())
    }

    fn write_separator(&mut self) -> Result<(), String> {
        let (start_x, cap) = self.line_measurements(LineStyle::Body)?;
        let y = self.begin_row()?;
        let rule_y = y.checked_add(4_500)
            .ok_or("separator center overflow")?;
        let end_x = start_x.checked_add(cap.min(60_000))
            .ok_or("separator horizontal overflow")?;
        self.ornaments.push(NotebookInkLine {
            page: self.page,
            start_um: (start_x, rule_y),
            end_um: (end_x, rule_y),
            width_um: 400,
            rgb: [30, 48, 74],
        });
        self.y = y.checked_add(self.pitch)
            .ok_or("separator row advance overflow")?;
        Ok(())
    }

    fn write_task_box(
        &mut self, start_x: u32, y: u32, checked: bool,
    ) -> Result<(), String> {
        let left = start_x.checked_sub(4_600)
            .ok_or("task box exceeds left margin")?;
        let right = left.checked_add(2_500)
            .ok_or("task box exceeds right margin")?;
        let top = y.checked_add(1_200)
            .ok_or("task box top overflow")?;
        let bottom = top.checked_add(2_500)
            .ok_or("task box bottom overflow")?;
        for (start, end) in [
            ((left, top), (right, top)),
            ((right, top), (right, bottom)),
            ((right, bottom), (left, bottom)),
            ((left, bottom), (left, top)),
        ] {
            self.ornaments.push(NotebookInkLine {
                page: self.page,
                start_um: start,
                end_um: end,
                width_um: 350,
                rgb: [30, 48, 74],
            });
        }
        if checked {
            let middle = (
                left.checked_add(1_100).ok_or("task check X overflow")?,
                bottom.checked_sub(350).ok_or("task check Y underflow")?,
            );
            let start = (
                left.checked_add(450).ok_or("task check X overflow")?,
                top.checked_add(1_250).ok_or("task check Y overflow")?,
            );
            let end = (
                right.checked_sub(250).ok_or("task check X underflow")?,
                top.checked_add(400).ok_or("task check Y overflow")?,
            );
            for (start_um, end_um) in [(start, middle), (middle, end)] {
                self.ornaments.push(NotebookInkLine {
                    page: self.page,
                    start_um,
                    end_um,
                    width_um: 450,
                    rgb: [30, 48, 74],
                });
            }
        }
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

fn start_continuation(
    first: bool, start_x: &mut u32, cap: &mut u32, indent: u32,
) -> Result<(), String> {
    if first {
        *cap = cap.checked_sub(indent)
            .ok_or("ordered continuation region overflow")?;
        *start_x = start_x.checked_add(indent)
            .ok_or("ordered continuation position overflow")?;
    }
    Ok(())
}

// Keep the authored marker and at least the first body glyph together.
// Continuations start after the actual measured numeral and separator spaces.
fn measured_ordered_label<'text>(
    content: &'text str,
    cap: u32,
    statistics: Option<&HandwritingStatisticsProfile>,
) -> Result<(u32, &'text str), String> {
    let (marker, after) = content.split_once(' ')
        .ok_or("ordered list requires a numeral and text")?;
    let spaces = after.bytes().take_while(|byte| *byte == b' ')
        .count().checked_add(1)
        .ok_or("ordered label spacing overflow")?;
    let (_, marker_width) = measured_word(marker, statistics)?;
    let indent = u32::try_from(spaces)
        .map_err(|_error| "ordered label spacing overflow")?
        .checked_mul(4_000)
        .and_then(|space_width| marker_width.checked_add(space_width))
        .ok_or("ordered label advance overflow")?;
    let first_word = after.trim_start_matches(' ')
        .split(' ').next().unwrap_or("");
    let (glyphs, _) = measured_word(first_word, statistics)?;
    let first_slot = glyphs.first()
        .ok_or("ordered list requires visible item text")?.2;
    if indent.checked_add(first_slot).is_none_or(|end| end > cap) {
        return Err(
            "ordered list label and first glyph exceed horizontal region".into()
        );
    }
    Ok((indent, marker))
}

fn measured_word<'text>(
    word: &'text str,
    statistics: Option<&HandwritingStatisticsProfile>,
) -> Result<(Vec<MeasuredGrapheme<'text>>, u32), String> {
    let mut glyphs = Vec::new();
    let mut width = 0u32;
    let mut remaining = word;
    while !remaining.is_empty() {
        let (grapheme, rest) = first_grapheme(remaining)?;
        remaining = rest;
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

// Closing ATX markers are syntax only after a separating space. A hash
// directly touching the title remains a literal authored handwriting glyph.
fn heading_body(body: &str) -> &str {
    let trimmed = body.trim_end_matches(' ');
    let without_hashes = trimmed.trim_end_matches('#');
    if without_hashes.len() < trimmed.len()
        && (without_hashes.is_empty() || without_hashes.ends_with(' ')) {
        return without_hashes.trim_end_matches(' ');
    }
    trimmed
}

// Fences are syntactic only with 0..3 initial ASCII spaces. A single
// optional short ASCII language tag is metadata, never authored ink.
fn markdown_fence_marker(line: &str) -> Option<(u8, usize, &str)> {
    let indent = line.bytes().take_while(|&byte| byte == b' ').count();
    if indent > 3 {
        return None;
    }
    let block = line.get(indent..)?;
    let marker = *block.as_bytes().first()?;
    if !matches!(marker, b'`' | b'~') {
        return None;
    }
    let count = block.bytes().take_while(|&byte| byte == marker).count();
    if count < 3 {
        return None;
    }
    Some((marker, count, block.get(count..)?))
}

// Only an all-marker closing line can close an active fence. A shorter or
// differently marked line remains literal code and needs authored glyphs.
fn markdown_fence_transition(
    active: &mut Option<(u8, usize)>, line: &str,
) -> bool {
    let Some((marker, count, tail)) = markdown_fence_marker(line) else {
        return false;
    };
    let suffix = tail.trim_matches(' ');
    match *active {
        Some((open, minimum)) if marker == open
            && count >= minimum && suffix.is_empty() => {
            *active = None;
            true
        }
        None if suffix.is_empty()
            || (suffix.len() <= 32 && !suffix.is_empty()
                && suffix.bytes().all(|byte| {
                    byte.is_ascii_alphanumeric()
                        || matches!(byte, b'_' | b'-' | b'+' | b'.' | b'#')
                })) => {
            *active = Some((marker, count));
            true
        }
        _ => false,
    }
}

// A Setext underline is structural only directly after a nonempty
// unquoted Markdown paragraph. With no paragraph, "==" stays authored
// text and "---" retains the existing thematic-separator meaning.
fn markdown_setext_level(line: &str) -> Option<HeadingLevel> {
    let indent = line.bytes().take_while(|byte| *byte == b' ').count();
    if indent > 3 {
        return None;
    }
    let marker = line.get(indent..)?.trim_end_matches(' ');
    if !marker.is_empty() && marker.bytes().all(|byte| byte == b'=') {
        Some(HeadingLevel::Primary)
    } else if !marker.is_empty()
        && marker.bytes().all(|byte| byte == b'-') {
        Some(HeadingLevel::Secondary)
    } else {
        None
    }
}

// A standalone numeral and delimiter is still an ordered-list marker,
// but it cannot render without an item. The regular path below handles
// labels with following authored content and spacing.
fn empty_ordered_marker(block: &str) -> bool {
    block.strip_suffix('.').or_else(|| block.strip_suffix(')'))
        .is_some_and(|digits| (1..=9).contains(&digits.len())
            && digits.bytes().all(|digit| digit.is_ascii_digit()))
}

// Restrict Markdown ordered markers to at most nine ASCII digits followed
// by ". " or ") ". The number and delimiter remain authored text ink;
// only unordered bullets receive a synthetic short line ornament.
fn markdown_line(line: &str) -> (LineStyle, Cow<'_, str>) {
    let raw = line.trim_end_matches(' ');
    let indent = line.bytes().take_while(|byte| *byte == b' ').count();
    // Indented block syntax does not consume a visible handwriting space.
    // At four spaces, keep the entire source as literal authored text.
    let block = if indent <= 3 {
        line.get(indent..).unwrap_or(line)
    } else {
        line
    };
    // A single leading backslash escapes a syntactic first-line marker.
    // Keep the marker as actual authored ink, but not the escape itself.
    if indent <= 3
        && let Some(escaped) = line.trim_start_matches(' ')
            .strip_prefix('\\')
        && escaped.bytes().next().is_some_and(|mark| {
            matches!(mark, b'#' | b'>' | b'-' | b'*' | b'+' | b'_')
                || matches!(mark, b'`' | b'~')
        })
    {
        return (LineStyle::Body, Cow::Borrowed(
            escaped.trim_end_matches(' ')));
    }
    // CommonMark thematic rules admit spaces between at least three
    // identical marks. Four-space lines stay literal (not code blocks).
    let rule = raw.get(indent..).unwrap_or(raw);
    if indent <= 3
        && let Some(&marker) = rule.as_bytes().first()
        && matches!(marker, b'-' | b'*' | b'_')
        && rule.bytes().filter(|&mark| mark == marker).count() >= 3
        && rule.bytes().all(|mark| mark == marker || mark == b' ') {
        return (LineStyle::Separator, Cow::Borrowed(""));
    }
    // Standard ATX headings admit up to three indentation spaces. Four
    // spaces remain literal text (not an inferred fenced/code block).
    if indent <= 3 {
        for (prefix, level) in [
            ("######", HeadingLevel::Senary),
            ("#####", HeadingLevel::Quinary),
            ("####", HeadingLevel::Quaternary),
            ("###", HeadingLevel::Tertiary),
            ("##", HeadingLevel::Secondary),
            ("#", HeadingLevel::Primary),
        ] {
            if let Some(body) = block.strip_prefix(prefix)
                && (body.is_empty() || body.starts_with(' ')) {
                // A bare ATX marker is an empty heading, not a literal
                // printed hash. It will refuse with the other empty blocks.
                // Tabs are already expanded to physical four-column stops.
                return (LineStyle::Heading(level), Cow::Borrowed(
                    heading_body(body.trim_start_matches(' '))));
            }
        }
    }
    // Empty list markers have no body ink. Do not silently print an
    // otherwise caller-authored punctuation glyph in their place.
    if matches!(block, "-" | "*" | "+") {
        return (LineStyle::Bullet, Cow::Borrowed(""));
    }
    if let Some(bullet) = block.strip_prefix("- ")
        .or_else(|| block.strip_prefix("* "))
        .or_else(|| block.strip_prefix("+ "))
    {
        // The first through fourth spaces after a list marker delimit
        // its body. Extra spaces retain physical authored indentation.
        // Tab stops are already expanded at Unicode source admission.
        let padding = bullet.bytes().take(3)
            .take_while(|byte| *byte == b' ').count();
        let content = bullet.get(padding..).unwrap_or(bullet);
        for (marker, checked) in [("[ ]", false), ("[x]", true),
            ("[X]", true)] {
            if let Some(after) = content.strip_prefix(marker)
                && (after.is_empty() || after.starts_with(' ')) {
                return (LineStyle::Task(checked),
                    Cow::Borrowed(after.trim_matches(' ')));
            }
        }
        return (LineStyle::Bullet,
            Cow::Borrowed(content.trim_end_matches(' ')));
    }
    if let Some(quote) = block.strip_prefix('>') {
        // One level of Markdown quote syntax, optionally followed by one
        // separator space. Nested > markers remain literal authored text.
        return (LineStyle::Quote, Cow::Borrowed(quote.strip_prefix(' ')
            .unwrap_or(quote).trim_end_matches(' ')));
    }
    if empty_ordered_marker(block) {
        return (LineStyle::Ordered, Cow::Borrowed(""));
    }
    if let Some((marker, item)) = block.split_once(' ')
        && let Some(digits) = marker.strip_suffix('.')
            .or_else(|| marker.strip_suffix(')'))
        && (1..=9).contains(&digits.len())
        && digits.bytes().all(|digit| digit.is_ascii_digit())
    {
        return (LineStyle::Ordered, Cow::Borrowed(
            if item.trim_matches(' ').is_empty() { "" }
            else { block.trim_end_matches(' ') }));
    }
    // A backslash before an otherwise syntactic ordered-list delimiter
    // escapes only that delimiter: 12\. text is ordinary authored text,
    // not a numbered list. Preserve every digit, punctuation glyph and
    // following source space; no general inline Markdown unescape is implied.
    let (escaped_label, escaped_item) =
        block.split_once(' ').unwrap_or((block, ""));
    if indent <= 3
        && let Some((digits, escaped_delimiter)) =
            escaped_label.split_once('\\')
        && (1..=9).contains(&digits.len())
        && digits.bytes().all(|digit| digit.is_ascii_digit())
        && matches!(escaped_delimiter, "." | ")")
    {
        let separator = if escaped_item.is_empty() { "" } else { " " };
        let literal = format!(
            "{digits}{escaped_delimiter}{separator}{escaped_item}"
        );
        return (LineStyle::Body, Cow::Owned(
            literal.trim_end_matches(' ').to_owned()));
    }
    // Leading 0..3 spaces on a Markdown paragraph are syntax, not ink.
    // Four or more remain literal rather than silently becoming code.
    (LineStyle::Body, Cow::Borrowed(if indent <= 3 {
        block.trim_end_matches(' ')
    } else { raw }))
}

// A single final backslash is Markdown hard-break syntax, including before
// two trailing hard-break spaces inserted by a Unicode line separator.
// Pairs and longer runs remain literal authored glyphs: this parser does
// not perform inline Markdown unescaping.
fn markdown_paragraph_break<'text>(
    raw: &str, style: LineStyle, body: &'text str,
) -> (&'text str, bool) {
    if !matches!(style, LineStyle::Body | LineStyle::Quote
        | LineStyle::Bullet | LineStyle::Ordered | LineStyle::Task(_))
        // Four-space rows are literal physical ink, not Markdown paragraph
        // syntax. Do not drop a final authored backslash as a hard break.
        || raw.starts_with("    ") {
        return (body, false);
    }
    if (raw.ends_with('\\') || raw.ends_with("  "))
        && !body.ends_with("\\\\")
        && let Some(visible) = body.strip_suffix('\\') {
        return (visible, true);
    }
    (body, raw.ends_with("  "))
}

// Detect a Setext title from one or more soft-joined Markdown source lines.
// The underline contributes no physical row; hard breaks, block boundaries
// and literal four-space indents prevent promotion to a heading.
fn take_following_setext<'text>(
    raw: &str, body: &str,
    lookahead: &mut Peekable<impl Iterator<Item = &'text str>>,
) -> bool {
    if raw.starts_with("    ")
        || markdown_paragraph_break(raw, LineStyle::Body, body).1 {
        return false;
    }
    while let Some(&next) = lookahead.peek() {
        if markdown_setext_level(next).is_some() {
            let _: Option<&str> = lookahead.next();
            return true;
        }
        // An opening code fence terminates the pending paragraph; a
        // later === inside the fence is literal ink, not a title mark.
        let mut opening = None;
        if markdown_fence_transition(&mut opening, next) {
            return false;
        }
        let (style, next_body) = markdown_line(next);
        if next.trim_end_matches(' ').is_empty()
            || next.starts_with("    ")
            || !matches!(style, LineStyle::Body)
            || markdown_paragraph_break(next, style, next_body.as_ref()).1
        {
            return false;
        }
        let _: Option<&str> = lookahead.next();
    }
    false
}

// A heading needs its first following visible line. Adjacent ATX and
// Setext headings, including collapsed blank paragraph gaps, stay with
// their first body row when that span fits on one physical sheet.
// Do not reserve blank fenced rows or gaps after the final visible heading.
fn following_heading_gap<'text>(
    lines: impl Iterator<Item = &'text str>,
) -> Option<FollowingInkSpacing> {
    let mut following = FollowingInkSpacing {
        blank_rows: 0,
        heading_rows: 0,
        paragraph_gaps: 0,
    };
    let mut fence = None;
    let mut pending_gap = false;
    let mut lookahead = lines.peekable();
    while let Some(line) = lookahead.next() {
        if markdown_fence_transition(&mut fence, line) {
            // Fence delimiters themselves have no physical row.
            continue;
        }
        if fence.is_some() && line.trim_matches(' ').is_empty() {
            following.blank_rows = following.blank_rows.saturating_add(1);
            continue;
        }
        if fence.is_none() && line.trim_end_matches(' ').is_empty() {
            // Multiple empty Markdown lines form just one 4-mm gap.
            pending_gap = true;
            continue;
        }
        if pending_gap {
            following.paragraph_gaps =
                following.paragraph_gaps.saturating_add(1);
            pending_gap = false;
        }
        // Blank physical code rows do not end a following title chain.
        // Continue counting headings after the closing fence so they and
        // their first body row can move together when the next page fits.
        if fence.is_some() {
            return Some(following);
        }
        let (style, body) = markdown_line(line);
        if matches!(style, LineStyle::Heading(_)) {
            following.heading_rows =
                following.heading_rows.saturating_add(1);
        } else if matches!(style, LineStyle::Body)
            && take_following_setext(line, body.as_ref(), &mut lookahead)
        {
            // A soft-joined Setext title contributes one physical row.
            following.heading_rows =
                following.heading_rows.saturating_add(1);
        } else {
            return Some(following);
        }
    }
    // The last heading is printable ink. Trailing empty Markdown or
    // fenced code rows must not reserve nonexistent following ink.
    (following.heading_rows > 0).then_some(FollowingInkSpacing {
        blank_rows: 0,
        heading_rows: following.heading_rows.saturating_sub(1),
        paragraph_gaps: following.paragraph_gaps,
    })
}

// A completed paragraph can be promoted into a Setext heading only
// before a closing delimiter, never inside a fence or quote block.
fn handle_markdown_setext<'text>(
    layout: &mut Layout<'_>, paragraph: &mut String,
    style: LineStyle, input_line: &str,
    following: impl Iterator<Item = &'text str>,
) -> Result<bool, String> {
    if paragraph.is_empty() || !matches!(style, LineStyle::Body) {
        return Ok(false);
    }
    let Some(level) = markdown_setext_level(input_line) else {
        return Ok(false);
    };
    if let Some(gap) = following_heading_gap(following) {
        layout.keep_heading_with_next(gap)?;
    }
    layout.layout_line(&take(paragraph), LineStyle::Heading(level))?;
    Ok(true)
}

// End a source paragraph at its structural boundary or explicit hard break.
// Keep the authored text buffer separate from physical rows and decorations.
fn flush_paragraph(
    layout: &mut Layout<'_>, paragraph: &mut String, style: LineStyle,
) -> Result<(), String> {
    if !paragraph.is_empty() {
        layout.layout_line(paragraph, style)?;
        paragraph.clear();
    }
    Ok(())
}

// Four or more leading ASCII spaces are literal physical placement, not
// Markdown soft continuation. They neither absorb a pending paragraph nor
// gain a synthetic joining space before their first authored glyph.
fn layout_literal_indented_row(
    layout: &mut Layout<'_>, paragraph: &mut String,
    previous: LineStyle, raw: &str, style: LineStyle, body: &str,
) -> Result<bool, String> {
    if !matches!(style, LineStyle::Body) || !raw.starts_with("    ") {
        return Ok(false);
    }
    flush_paragraph(layout, paragraph, previous)?;
    layout.layout_line(body, style)?;
    Ok(true)
}

// Join an ordinary 2..3-space-indented line to the prior one-level list
// item, preserving the native hanging indent and a single initial marker.
// A new block or four-space literal line flushes the previous item instead.
fn continue_list_item(
    layout: &mut Layout<'_>, paragraph: &mut String, previous: LineStyle,
    line: &str, style: LineStyle, body: &str, hard_break: bool,
) -> Result<bool, String> {
    if paragraph.is_empty() || !matches!(previous,
        LineStyle::Bullet | LineStyle::Ordered | LineStyle::Task(_)) {
        return Ok(false);
    }
    let indent = line.bytes().take_while(|byte| *byte == b' ').count();
    if (2..=3).contains(&indent) && matches!(style, LineStyle::Body) {
        paragraph.push(' ');
        paragraph.push_str(body);
        if hard_break {
            flush_paragraph(layout, paragraph, previous)?;
        }
        return Ok(true);
    }
    flush_paragraph(layout, paragraph, previous)?;
    Ok(false)
}

// Defer a list's physical row until its possible soft continuations.
// Trailing two spaces also represent pasted Unicode hard line separators,
// so they end the current list item before the next source row.
fn queue_list_item(
    layout: &mut Layout<'_>, paragraph: &mut String,
    paragraph_style: &mut LineStyle, style: LineStyle,
    body: &str, hard_break: bool,
) -> Result<(), String> {
    *paragraph_style = style;
    paragraph.push_str(body);
    if hard_break {
        flush_paragraph(layout, paragraph, style)?;
    }
    Ok(())
}

// A fenced row is literal handwriting with physical hard breaks. Only
// the fence delimiter is syntax: no invented code font, glyph, or style.
fn handle_markdown_code_line(
    layout: &mut Layout<'_>,
    paragraph: &mut String,
    paragraph_style: LineStyle,
    pending_gap: &mut bool,
    fence: &mut Option<(u8, usize)>,
    input_line: &str,
) -> Result<bool, String> {
    if markdown_fence_transition(fence, input_line) {
        flush_paragraph(layout, paragraph, paragraph_style)?;
        return Ok(true);
    }
    if fence.is_none() {
        return Ok(false);
    }
    if *pending_gap {
        layout.add_gap()?;
        *pending_gap = false;
    }
    if input_line.trim_matches(' ').is_empty() {
        layout.blank_row()?;
    } else {
        layout.layout_line(input_line, LineStyle::Body)?;
    }
    Ok(true)
}

// Derive row height from visible authored text, not Markdown syntax. A
// per-glyph height override for #, -, +, *, or > must not alter pitch
// when that marker becomes a heading/list/quote ornament instead of ink.
fn measured_pitch(
    source: &FlowSource,
    statistics: Option<&HandwritingStatisticsProfile>,
) -> Result<u32, String> {
    let mut pitch = DEFAULT_PITCH_UM;
    let mut fence = None;
    let mut pending_setext = false;
    let mut pending_list = false;
    for raw in source.text.lines() {
        let visible = if source.format == FlowFormat::Markdown {
            if markdown_fence_transition(&mut fence, raw) {
                pending_setext = false;
                pending_list = false;
                continue;
            }
            if fence.is_some() {
                pending_setext = false;
                pending_list = false;
                Cow::Borrowed(raw)
            } else if pending_setext && markdown_setext_level(raw).is_some() {
                pending_setext = false;
                continue;
            } else {
                let (style, body) = markdown_line(raw);
                let (visible, hard_break) = markdown_paragraph_break(
                    raw, style, body.as_ref());
                // List continuations belong to their pending list item,
                // not to an ordinary paragraph eligible for Setext.
                let indent = raw.bytes()
                    .take_while(|byte| *byte == b' ').count();
                let list_continuation = pending_list
                    && (2..=3).contains(&indent)
                    && matches!(style, LineStyle::Body);
                pending_list = !hard_break && (list_continuation
                    || matches!(style, LineStyle::Bullet | LineStyle::Ordered
                        | LineStyle::Task(_)));
                // Four-space literal rows are also laid out immediately,
                // leaving no ordinary paragraph for Setext promotion.
                pending_setext = !list_continuation
                    && matches!(style, LineStyle::Body)
                    && indent < 4
                    && !visible.trim_matches(' ').is_empty()
                    && !hard_break;
                Cow::Owned(visible.to_owned())
            }
        } else {
            Cow::Borrowed(raw)
        };
        let mut remaining = visible.as_ref();
        while !remaining.is_empty() {
            let (grapheme, rest) = first_grapheme(remaining)?;
            remaining = rest;
            // Unlike ordinary spaces, Unicode whitespace can have exact
            // authored glyph ink and its own height/baseline statistics.
            if grapheme == " " {
                continue;
            }
            if let Some(stats) = statistics {
                let height = stats
                    .statistics_for(grapheme, Dimension::Height)
                    .ok_or("flow glyph height statistics missing")?.mean;
                let baseline = stats
                    .statistics_for(grapheme, Dimension::BaselineOffset)
                    .ok_or("flow baseline statistics missing")?.mean;
                let needed = height.checked_add(baseline.abs())
                    .and_then(|combined| combined.checked_add(4_000))
                    .ok_or("flow row height overflow")?;
                pitch = pitch.max(u32::try_from(needed)
                    .map_err(|_error| "flow row height refused")?);
            }
        }
    }
    Ok(pitch)
}

// Build one physical page layout from the caller's profile and optional
// rectangle. The source parser never changes the saved paper geometry.
fn initialize_layout<'model>(
    profile: &NotebookProfile,
    statistics: Option<&'model HandwritingStatisticsProfile>,
    heading: HeadingDecoration, region: Option<FlowRegion>, pitch: u32,
) -> Result<Layout<'model>, String> {
    let printable = profile.printable_rect_um()?;
    let (left, top, right, bottom) = if let Some(fixed) = region {
        fixed.bounds_within(printable)?
    } else {
        printable
    };
    Ok(Layout {
        heading,
        statistics,
        lines: Vec::new(),
        ornaments: Vec::new(),
        page: 1,
        pending_blank_rows: 0,
        y: top.checked_add(INSET_UM)
            .ok_or("flow top margin overflow")?,
        top,
        bottom,
        left,
        right,
        row_count: 0,
        pitch,
    })
}

// Finalize only after checking fence closure and remaining authored ink.
// No output bytes are written until the entire layout has been admitted.
fn finish_layout(
    mut layout: Layout<'_>, mut paragraph: String,
    paragraph_style: LineStyle, fence: Option<(u8, usize)>,
) -> Result<(Vec<TextRun>, Vec<NotebookInkLine>, usize), String> {
    if fence.is_some() {
        return Err("unclosed Markdown code fence refused".into());
    }
    flush_paragraph(&mut layout, &mut paragraph, paragraph_style)?;
    if layout.lines.is_empty() {
        return Err("flow requires at least one visible text block".into());
    }
    // Explicit blank rows can reserve space before later ink, but trailing
    // rows must not introduce a physical page with no printable content.
    // Include terminal rules as they are real ink without a text run.
    let last_ink_page = layout.lines.iter().map(|run| run.page)
        .chain(layout.ornaments.iter().map(|mark| mark.page))
        .max().ok_or("flow has no printable page")?;
    Ok((layout.lines, layout.ornaments, last_ink_page))
}

/// Compile bounded visible text on the caller-selected native page profile.
/// This does not create persistent notebook semantic state.
#[expect(clippy::redundant_pub_crate, reason = "Sibling CLI module access")]
#[cfg(test)]
pub(super) fn layout_flow(
    source: &FlowSource,
    profile: &NotebookProfile,
    statistics: Option<&HandwritingStatisticsProfile>,
    decoration: HeadingDecoration,
) -> Result<(Vec<TextRun>, Vec<NotebookInkLine>, usize), String> {
    layout_flow_with_region(source, profile, statistics, decoration, None)
}

/// An optional fixed rectangle constrains every page's text and decoration.
/// The whole printable rectangle remains the default when none is selected.
#[expect(clippy::redundant_pub_crate, reason = "Sibling CLI module access")]
pub(super) fn layout_flow_with_region(
    source: &FlowSource,
    profile: &NotebookProfile,
    statistics: Option<&HandwritingStatisticsProfile>,
    decoration: HeadingDecoration,
    region: Option<FlowRegion>,
) -> Result<(Vec<TextRun>, Vec<NotebookInkLine>, usize), String> {
    let pitch = measured_pitch(source, statistics)?;
    let mut layout = initialize_layout(
        profile, statistics, decoration, region, pitch,
    )?;
    // Markdown blank-line runs separate paragraphs once, even after multiple
    // empty source lines; leading and trailing blanks add no physical space.
    // Plain-text blank lines instead consume a full physical writing row.
    let mut paragraph = String::new();
    let mut paragraph_style = LineStyle::Body;
    let mut pending_gap = false;
    let mut fence = None;
    let mut source_lines = source.text.lines();
    while let Some(input_line) = source_lines.next() {
        if source.format == FlowFormat::Markdown
            && handle_markdown_code_line(&mut layout, &mut paragraph,
                paragraph_style, &mut pending_gap, &mut fence, input_line)? {
            continue;
        }
        if source.format == FlowFormat::Markdown
            && handle_markdown_setext(&mut layout, &mut paragraph,
                paragraph_style, input_line, source_lines.clone())? {
            continue;
        }
        let raw = input_line.trim_end_matches(' ');
        if raw.is_empty() {
            if source.format == FlowFormat::Plain {
                layout.blank_row()?;
            } else {
                flush_paragraph(&mut layout, &mut paragraph, paragraph_style)?;
                pending_gap = layout.row_count > 0;
            }
            continue;
        }
        if pending_gap {
            layout.add_gap()?;
            pending_gap = false;
        }
        let (style, parsed_body) = if source.format == FlowFormat::Markdown {
            markdown_line(input_line)
        } else {
            (LineStyle::Body, Cow::Borrowed(raw))
        };
        let (body, hard_break) = if source.format == FlowFormat::Markdown {
            markdown_paragraph_break(input_line, style, parsed_body.as_ref())
        } else {
            (parsed_body.as_ref(), false)
        };
        if matches!(style, LineStyle::Separator) {
            flush_paragraph(&mut layout, &mut paragraph, paragraph_style)?;
            layout.write_separator()?;
            continue;
        }
        if body.trim_matches(' ').is_empty() {
            return Err("empty Markdown block refused".into());
        }
        if source.format == FlowFormat::Markdown
            && layout_literal_indented_row(&mut layout, &mut paragraph,
                paragraph_style, input_line, style, body)? {
            continue;
        }
        if source.format == FlowFormat::Markdown
            && continue_list_item(&mut layout, &mut paragraph,
                paragraph_style, input_line, style, body, hard_break)? {
            continue;
        }
        if source.format == FlowFormat::Markdown
            && matches!(style, LineStyle::Body | LineStyle::Quote)
        {
            // A quoted paragraph merges adjacent quoted soft lines, but
            // never joins an unquoted paragraph or another block kind.
            if !paragraph.is_empty()
                && matches!(style, LineStyle::Quote)
                    != matches!(paragraph_style, LineStyle::Quote)
            {
                flush_paragraph(&mut layout, &mut paragraph, paragraph_style)?;
            }
            if !paragraph.is_empty() {
                paragraph.push(' ');
            }
            paragraph_style = style;
            paragraph.push_str(body);
            if hard_break {
                flush_paragraph(&mut layout, &mut paragraph, paragraph_style)?;
            }
        } else {
            flush_paragraph(&mut layout, &mut paragraph, paragraph_style)?;
            if matches!(style, LineStyle::Heading(_))
                && let Some(gap) = following_heading_gap(source_lines.clone()) {
                layout.keep_heading_with_next(gap)?;
            }
            if source.format == FlowFormat::Markdown
                && matches!(style,
                    LineStyle::Bullet | LineStyle::Ordered | LineStyle::Task(_))
            {
                queue_list_item(&mut layout, &mut paragraph,
                    &mut paragraph_style, style, body, hard_break)?;
            } else {
                layout.layout_line(body, style)?;
            }
        }
    }
    finish_layout(layout, paragraph, paragraph_style, fence)
}
