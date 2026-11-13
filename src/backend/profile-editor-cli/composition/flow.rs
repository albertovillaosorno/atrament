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
use atrament_unicode_grapheme_segmentation::UnicodeGraphemeSegmentation;

use super::glyph::first_grapheme;
use super::{GlyphOverlay, HeadingInk, TextRun};

const MAX_FLOW_BYTES: usize = 4_096;
const MAX_FLOW_LINES: usize = 128;
const MAX_LINE_GRAPHEMES: usize = 96;
const MAX_PAGES: usize = 32;
const INSET_UM: u32 = 2_500;
const EXTRA_RIGHT_UM: u32 = 1_500;
const DEFAULT_PARAGRAPH_GAP_UM: u32 = 4_000;
const MAX_PARAGRAPH_GAP_UM: u32 = 12_000;
const MAX_BODY_INSET_UM: u32 = 12_000;
const DEFAULT_PITCH_UM: u32 = 9_000;
const MAX_SELECTED_PITCH_UM: u32 = 30_000;
const SEPARATOR_CENTER_UM: u32 = 4_500;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[expect(clippy::redundant_pub_crate, reason = "Sibling CLI module access")]
pub(super) enum FlowFormat {
    Markdown,
    Plain,
}

/// CLI-only layout selections; absent fields keep previous physical ink.
#[derive(Clone, Copy, Debug, Default)]
#[expect(clippy::redundant_pub_crate, reason = "Sibling CLI module access")]
pub(super) struct FlowTypography {
    pub(super) body_inset: Option<u32>,
    pub(super) body_right_inset: Option<u32>,
    pub(super) list_quote_inset: Option<u32>,
    pub(super) list_quote_right_inset: Option<u32>,
    pub(super) paragraph_gap: Option<u32>,
    pub(super) pitch: Option<u32>,
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
        if self.width == 0 || self.height == 0 {
            return Err(
                "flow region width and height must be positive".into()
            );
        }
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
            if !(1..=MAX_PAGES).contains(&line.page)
                || !(80..=1_500).contains(&line.width_um)
                || line.start_um == line.end_um {
                return Err(concat!(
                    "flow region ink needs valid page, ",
                    "width and endpoints",
                ).into());
            }
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
    fn admit(
        format: FlowFormat, source: &str, strip_bom: bool,
    ) -> Result<Self, String> {
        // Exactly one leading UTF-8 signature is transport metadata.
        // Once admitted, another leading BOM is literal authored ink.
        // Keep the original-input byte cap, including any signature.
        let admitted = if strip_bom {
            source.strip_prefix('\u{feff}').unwrap_or(source)
        } else {
            source
        };
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

    /// Read a chosen regular text file, or explicit `-` standard input,
    /// without creating any implicit saved notebook. The stream reads at
    /// most one byte beyond the admitted limit to detect oversize input.
    pub(super) fn from_file(
        format: FlowFormat, path: &str,
    ) -> Result<Self, String> {
        let bytes = if path == "-" {
            super::read_bounded_stdin(MAX_FLOW_BYTES, "flow")?
        } else {
            super::read_notebook_input(path, MAX_FLOW_BYTES)?
        };
        let source = String::from_utf8(bytes).map_err(|_error| {
            if path == "-" {
                "flow stdin must be UTF-8"
            } else {
                "flow file must be UTF-8"
            }
        })?;
        Self::new(format, &source)
    }

    pub(super) fn new(
        format: FlowFormat, source: &str,
    ) -> Result<Self, String> {
        Self::admit(format, source, true)
    }

    // Sibling typed callers can construct or mutate a FlowSource without
    // going through the CLI parser. Re-admit its already-normalized bytes
    // before physical layout; do not let that bypass the 4096-byte,
    // visible-text, control-character or source-normalization boundaries.
    fn validate_canonical(&self) -> Result<(), String> {
        let admitted = Self::admit(self.format, &self.text, false)?;
        if admitted.text != self.text {
            return Err("flow source needs canonical normalized text".into());
        }
        Ok(())
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
                if selected.len() != 7 || !selected.starts_with('#')
                    || !selected.bytes().skip(1)
                        .all(|byte| byte.is_ascii_hexdigit()) {
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
        if let Some(scale) = self.glyph_scale
            && (scale < 100 || scale > 250) {
            return Err("heading ink scale must be 100..250");
        }
        if let Some(scale) = self.marker_scale
            && (scale < 50 || scale > 105) {
            return Err("heading marker scale must be 50..105");
        }
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
// lines reserve full physical rows once followed by actual ink.
// Intermediate rows are headings or, when requested, thematic rules;
// the initial and final ink rows are measured separately.
#[derive(Clone, Copy)]
struct FollowingInkSpacing {
    blank_rows: usize,
    intermediate_rows: usize,
    paragraph_gaps: usize,
}

// An adjacent chain can own a complete page-fitting block only if its
// first title was not already attached to an earlier heading or rule.
#[derive(Clone, Copy)]
struct HeadingChain {
    attached_to_prior: bool,
    first_mark: usize,
    first_run: usize,
    // A free thematic rule may reserve this chain's first title. Keep
    // that measured, run-less row with the whole group if it fits.
    leading_rules: Option<RuleChain>,
}

// Thematic rules have physical rows but no text runs. A reserved chain
// keeps their marks together until its next heading or wrapped block.
#[derive(Clone, Copy)]
struct RuleChain {
    count: usize,
    first_mark: usize,
}

struct Layout<'model> {
    body_inset_um: u32,
    body_right_inset_um: u32,
    bottom: u32,
    heading: HeadingDecoration,
    // Whether the last heading was itself bound to an earlier title/rule.
    heading_attached_to_prior: bool,
    // Adjacent title chain for later whole-block pagination.
    heading_chain: Option<HeadingChain>,
    last_visible_heading: bool,
    // True only if the preceding rule actually reserved its next ink row.
    last_visible_rule: bool,
    left: u32,
    lines: Vec<TextRun>,
    list_quote_inset_um: u32,
    list_quote_right_inset_um: u32,
    ornaments: Vec<NotebookInkLine>,
    page: usize,
    paragraph_gap_um: u32,
    pending_blank_rows: usize,
    pitch: u32,
    right: u32,
    row_count: usize,
    rule_chain: Option<RuleChain>,
    statistics: Option<&'model HandwritingStatisticsProfile>,
    top: u32,
    y: u32,
}

impl Layout<'_> {
    fn add_gap(&mut self) -> Result<(), String> {
        self.flush_blank_rows()?;
        self.y = self
            .y
            .checked_add(self.paragraph_gap_um)
            .ok_or("paragraph gap overflow")?;
        Ok(())
    }

    fn arrange_wrapped_block(
        &mut self, run_start: usize, mark_start: usize,
        heading_before_block: bool, preceding_reservation: bool,
        style: LineStyle, protect_body: bool,
    ) -> Result<(), String> {
        self.keep_heading_with_fitting_block(
            run_start, mark_start, heading_before_block,
            style, protect_body,
        )?;
        self.keep_rule_with_fitting_block(
            run_start, heading_before_block,
            preceding_reservation, style, protect_body,
        )?;
        if !preceding_reservation {
            self.keep_fitting_block_together(
                run_start, mark_start, style, protect_body, &[], 0,
            )?;
        }
        if Self::balance_tail_eligible(style, protect_body) {
            self.balance_wrapped_tail(run_start, mark_start, style)?;
        }
        if !matches!(style, LineStyle::Heading(_)) {
            self.heading_chain = None;
            self.rule_chain = None;
        }
        Ok(())
    }

    const fn balance_tail_eligible(
        style: LineStyle, protect_body: bool,
    ) -> bool {
        (protect_body && matches!(style, LineStyle::Body))
            || matches!(style, LineStyle::Bullet | LineStyle::Ordered
                | LineStyle::Quote | LineStyle::Task(_))
    }

    // A long wrapped Markdown block should not leave a single terminal
    // physical row on a new page if the previous page has three rows of
    // that same block. The first-two-row and heading/rule reservations
    // take priority. List markers belong to the unmoved first row; quote
    // rules must follow both relocated physical continuation rows.
    fn balance_wrapped_tail(
        &mut self, first_run: usize, first_ornament: usize,
        style: LineStyle,
    ) -> Result<(), String> {
        let runs = self.lines.get_mut(first_run..)
            .ok_or("flow block-tail origin out of bounds")?;
        let run_count = runs.len();
        if run_count < 4 {
            return Ok(());
        }
        let (last, before_last) = runs.split_last_mut()
            .ok_or("flow block-tail missing final row")?;
        let (penultimate, earlier) = before_last.split_last_mut()
            .ok_or("flow block-tail missing penultimate row")?;
        let previous_page = penultimate.page;
        let page = last.page;
        let top = self.top.checked_add(INSET_UM)
            .ok_or("flow block-tail top inset overflow")?;
        let bottom = self.bottom.checked_sub(INSET_UM)
            .ok_or("flow block-tail bottom inset overflow")?;
        let second_y = top.checked_add(self.pitch)
            .ok_or("flow block-tail row overflow")?;
        if top.checked_add(self.pitch.checked_mul(2)
                .ok_or("flow block-tail height overflow")?)
                .is_none_or(|end| end > bottom)
            || previous_page.checked_add(1) != Some(page)
            || earlier.len() < 2
            || !earlier.iter().rev().take(2)
                .all(|row| row.page == previous_page)
            || last.origin_um.1 != top
        {
            return Ok(());
        }
        if matches!(style, LineStyle::Quote) {
            let bars = self.ornaments.get_mut(first_ornament..)
                .ok_or("flow quote-bar origin out of bounds")?;
            if bars.len() != run_count {
                return Err("flow quote bars do not match physical rows".into());
            }
            let (last_bar, earlier_bars) = bars.split_last_mut()
                .ok_or("flow quote missing final bar")?;
            let (before_last_bar, _) = earlier_bars.split_last_mut()
                .ok_or("flow quote missing penultimate bar")?;
            let relocate = |bar: &mut NotebookInkLine, row_y: u32|
                -> Result<(), String> {
                let rule_top = row_y.checked_add(500)
                    .ok_or("flow quote-tail top overflow")?;
                let rule_bottom = row_y.checked_add(self.pitch)
                    .and_then(|end| end.checked_sub(600))
                    .ok_or("flow quote-tail bottom overflow")?;
                bar.page = page;
                bar.start_um.1 = rule_top;
                bar.end_um.1 = rule_bottom;
                Ok(())
            };
            relocate(before_last_bar, top)?;
            relocate(last_bar, second_y)?;
        }
        penultimate.page = page;
        penultimate.origin_um.1 = top;
        last.origin_um.1 = second_y;
        self.y = self.y.checked_add(self.pitch)
            .ok_or("flow block-tail cursor overflow")?;
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

    // Only title rows may join this candidate. Account for every rule's
    // virtual row and each collapsed gap; physical fenced rows, unrelated
    // ornaments and page turns must not be pulled along.
    fn fitting_heading_gaps(
        &self, heading_index: usize, first_run: usize,
        last_heading_index: usize, leading_rules: Option<RuleChain>,
        body_gap: u32,
    ) -> Result<Option<Vec<(usize, u32)>>, String> {
        let heading = self.lines.get(heading_index)
            .ok_or("flow heading chain missing first title")?;
        let prefix_rows = leading_rules.map_or(0, |chain| chain.count);
        let mut gaps = if let Some(chain) = leading_rules {
            let Some(gaps) = self.fitting_rule_gaps(
                chain, heading.page, heading.origin_um.1,
            )? else {
                return Ok(None);
            };
            gaps
        } else {
            Vec::new()
        };
        let titles = self.lines.get(heading_index..first_run)
            .ok_or("flow heading chain out of bounds")?;
        for (index, pair) in titles.windows(2).enumerate() {
            let [earlier, later] = pair else {
                return Err("flow heading chain missing title".into());
            };
            if earlier.page != heading.page || later.page != heading.page {
                return Ok(None);
            }
            let after = earlier.origin_um.1.checked_add(self.pitch)
                .ok_or("flow heading chain spacing overflow")?;
            let Some(interior_gap) = later.origin_um.1.checked_sub(after)
            else {
                return Ok(None);
            };
            if interior_gap != 0 && interior_gap != self.paragraph_gap_um {
                return Ok(None);
            }
            if interior_gap != 0 {
                let gap_index = index.checked_add(prefix_rows)
                    .ok_or("flow heading gap index overflow")?;
                gaps.push((gap_index, interior_gap));
            }
        }
        if body_gap != 0 {
            let last = last_heading_index.checked_sub(heading_index)
                .and_then(|index| index.checked_add(prefix_rows))
                .ok_or("flow heading chain index overflow")?;
            gaps.push((last, body_gap));
        }
        Ok(Some(gaps))
    }

    // A relocated heading/block group retains each selected Markdown
    // paragraph gap.
    // between titles and before the block. All ink, row and page cursor
    // offsets share these checked measurements; page turns omit the gaps.
    fn fitting_row_offset(
        pitch: u32, index: usize, gaps_after: &[(usize, u32)],
    ) -> Result<u32, String> {
        let rows = u32::try_from(index)
            .map_err(|_error| "flow short-block row index overflow")?;
        let mut offset = pitch.checked_mul(rows)
            .ok_or("flow short-block row offset overflow")?;
        for &(after, gap) in gaps_after {
            if index > after {
                offset = offset.checked_add(gap)
                    .ok_or("flow short-block row offset overflow")?;
            }
        }
        Ok(offset)
    }

    // Consecutive separator marks are run-less measured rows. Admit only
    // horizontal 0.4-mm rules on one page with selected paragraph gaps;
    // fenced physical blanks and earlier page turns break the group.
    fn fitting_rule_gaps(
        &self, chain: RuleChain, page: usize, following_y: u32,
    ) -> Result<Option<Vec<(usize, u32)>>, String> {
        let end = chain.first_mark.checked_add(chain.count)
            .ok_or("flow rule-chain length overflow")?;
        let rules = self.ornaments.get(chain.first_mark..end)
            .ok_or("flow rule-chain marks out of bounds")?;
        let mut gaps = Vec::new();
        let mut previous_y: Option<u32> = None;
        for (index, rule) in rules.iter().enumerate() {
            let Some(rule_y) = rule.start_um.1
                .checked_sub(SEPARATOR_CENTER_UM) else {
                return Ok(None);
            };
            if rule.page != page || rule.width_um != 400
                || rule.start_um.1 != rule.end_um.1 {
                return Ok(None);
            }
            if let Some(prior_y) = previous_y {
                let after = prior_y.checked_add(self.pitch)
                    .ok_or("flow rule-chain spacing overflow")?;
                let Some(gap) = rule_y.checked_sub(after) else {
                    return Ok(None);
                };
                if gap != 0 && gap != self.paragraph_gap_um {
                    return Ok(None);
                }
                if gap != 0 {
                    let before = index.checked_sub(1)
                        .ok_or("flow rule-chain gap index underflow")?;
                    gaps.push((before, gap));
                }
            }
            previous_y = Some(rule_y);
        }
        let Some(last_rule_y) = previous_y else {
            return Ok(None);
        };
        let after = last_rule_y.checked_add(self.pitch)
            .ok_or("flow rule-chain final spacing overflow")?;
        let Some(gap) = following_y.checked_sub(after) else {
            return Ok(None);
        };
        if gap != 0 && gap != self.paragraph_gap_um {
            return Ok(None);
        }
        if gap != 0 {
            let last = chain.count.checked_sub(1)
                .ok_or("flow rule-chain final gap index underflow")?;
            gaps.push((last, gap));
        }
        Ok(Some(gaps))
    }

    // Only flowing Markdown prose receives optional left/right insets.
    // Literal code, indented source, headings, quotes and list markers
    // retain their original physical anchors and measured row widths.
    fn flow_row_bounds(
        &self, style: LineStyle, protect_body: bool,
    ) -> Result<(u32, u32), String> {
        let (mut start_x, mut cap) = self.line_measurements(style)?;
        if protect_body && matches!(style, LineStyle::Body) {
            start_x = start_x.checked_add(self.body_inset_um)
                .ok_or("flow body inset position overflow")?;
            cap = cap.checked_sub(self.body_inset_um)
                .filter(|width| *width >= 4_000)
                .ok_or("flow body inset leaves no writing row")?;
            cap = cap.checked_sub(self.body_right_inset_um)
                .filter(|width| *width >= 4_000)
                .ok_or("flow body right inset leaves no writing row")?;
        } else if matches!(style, LineStyle::Bullet | LineStyle::Ordered
            | LineStyle::Task(_) | LineStyle::Quote) {
            cap = cap.checked_sub(self.list_quote_right_inset_um)
                .filter(|width| *width >= 4_000)
                .ok_or("flow list/quote right inset leaves no writing row")?;
        }
        Ok((start_x, cap))
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

    fn keep_first_row_with_next(
        &mut self, following: FollowingInkSpacing,
    ) -> Result<bool, String> {
        self.flush_blank_rows()?;
        let top = self.top.checked_add(INSET_UM)
            .ok_or("flow keep-row top inset overflow")?;
        let bottom = self.bottom.checked_sub(INSET_UM)
            .ok_or("flow keep-row bottom inset overflow")?;
        let rows = u32::try_from(following.blank_rows)
            .map_err(|_error| "flow blank-row budget overflow")?
            .checked_add(u32::try_from(following.intermediate_rows)
                .map_err(|_error| "flow chain-row budget overflow")?)
            .and_then(|rows| rows.checked_add(2))
            .ok_or("flow following-row budget overflow")?;
        let gaps = u32::try_from(following.paragraph_gaps)
            .map_err(|_error| "flow paragraph-gap budget overflow")?
            .checked_mul(self.paragraph_gap_um)
            .ok_or("flow paragraph-gap budget overflow")?;
        let needed = self.pitch.checked_mul(rows)
            .and_then(|value| value.checked_add(gaps))
            .ok_or("flow following-row budget overflow")?;
        // If one sheet cannot hold this full span, fall back to normal
        // pagination instead of refusing valid short paper outright.
        if top.checked_add(needed).is_none_or(|end| end > bottom) {
            return Ok(false);
        }
        if self.y.checked_add(needed).is_none_or(|end| end > bottom) {
            self.page = self.page.checked_add(1)
                .ok_or("flow page count overflow")?;
            if self.page > MAX_PAGES {
                return Err("flow exceeds 32 physical pages".into());
            }
            self.y = top;
        }
        Ok(true)
    }

    // A Markdown block that fits on one fresh sheet should not be split
    // across two sheets when independent ink occupies its starting page.
    // The global 128-row budget bounds this check; long blocks that cannot
    // fit retain ordinary pagination and terminal-row balancing. A heading
    // or rule's reserved next row takes priority.
    fn keep_fitting_block_together(
        &mut self, first_run: usize, first_ornament: usize,
        style: LineStyle, protect_body: bool,
        gaps_after: &[(usize, u32)], leading_rules: usize,
    ) -> Result<(), String> {
        if !Self::balance_tail_eligible(style, protect_body) {
            return Ok(());
        }
        let runs = self.lines.get(first_run..)
            .ok_or("flow short-block origin out of bounds")?;
        let prefix_rows = leading_rules;
        let count = runs.len().checked_add(prefix_rows)
            .ok_or("flow short-block size overflow")?;
        if count < 3 {
            return Ok(());
        }
        let first = runs.first()
            .ok_or("flow short-block missing first row")?;
        let (old_page, first_y) = if leading_rules > 0 {
            let rule = self.ornaments.get(first_ornament)
                .ok_or("flow short-block missing leading rule")?;
            (rule.page, rule.start_um.1.checked_sub(SEPARATOR_CENTER_UM)
                .ok_or("flow leading-rule origin underflow")?)
        } else {
            (first.page, first.origin_um.1)
        };
        let new_page = old_page.checked_add(1)
            .ok_or("flow short-block page overflow")?;
        let old_rows = runs.iter().take_while(|row| row.page == old_page)
            .count().checked_add(prefix_rows)
            .ok_or("flow short-block old-row count overflow")?;
        let top = self.top.checked_add(INSET_UM)
            .ok_or("flow short-block top inset overflow")?;
        let bottom = self.bottom.checked_sub(INSET_UM)
            .ok_or("flow short-block bottom inset overflow")?;
        let total_gap = gaps_after.iter().try_fold(0u32, |sum, &(_, gap)| {
            sum.checked_add(gap).ok_or("flow short-block gap overflow")
        })?;
        let block_height = self.pitch.checked_mul(
            u32::try_from(count)
                .map_err(|_error| "flow short-block size overflow")?
        ).and_then(|height| height.checked_add(total_gap))
            .ok_or("flow short-block height overflow")?;
        let earlier_rows = self.lines.get(..first_run)
            .ok_or("flow short-block earlier rows out of bounds")?;
        let earlier_marks = self.ornaments.get(..first_ornament)
            .ok_or("flow short-block earlier marks out of bounds")?;
        if old_rows < 2 || old_rows >= count
            || top.checked_add(block_height).is_none_or(|end| end > bottom)
            || (!earlier_rows.iter().any(|run| run.page == old_page)
                && !earlier_marks.iter().any(|mark| mark.page == old_page)) {
            return Ok(());
        }
        // The block must be a contiguous two-page split. Never pull an
        // older row, or a later page's ink, into this replacement sheet.
        for (index, run) in runs.iter().enumerate() {
            let grouped_index = index.checked_add(prefix_rows)
                .ok_or("flow short-block row index overflow")?;
            let (page, origin) = if grouped_index < old_rows {
                (old_page, first_y)
            } else {
                (new_page, top)
            };
            let row_index = if grouped_index < old_rows {
                grouped_index
            } else {
                grouped_index.saturating_sub(old_rows)
            };
            let row_offset = Self::fitting_row_offset(
                self.pitch, row_index,
                if grouped_index < old_rows { gaps_after } else { &[] },
            )?;
            if run.page != page
                || origin.checked_add(row_offset) != Some(run.origin_um.1) {
                return Ok(());
            }
        }
        self.move_fitting_block_marks(
            first_run, first_ornament, leading_rules, (new_page, top),
            gaps_after,
        )?;
        let rows = self.lines.get_mut(first_run..)
            .ok_or("flow short-block rows disappeared")?;
        for (index, row) in rows.iter_mut().enumerate() {
            let grouped_index = index.checked_add(prefix_rows)
                .ok_or("flow short-block row index overflow")?;
            let offset = Self::fitting_row_offset(
                self.pitch, grouped_index, gaps_after,
            )?;
            row.page = new_page;
            row.origin_um.1 = top.checked_add(offset)
                .ok_or("flow short-block row position overflow")?;
        }
        self.y = top.checked_add(block_height)
            .ok_or("flow short-block cursor overflow")?;
        Ok(())
    }

    // When a complete body/list/quote and its preceding heading chain
    // fit a fresh sheet, move the full group instead of splitting the
    // title and continuation rows. A free thematic rule may lead the
    // group, but never detach an earlier reserved title or separator.
    // Reuse the bounded mover so native ornaments follow their text rows.
    fn keep_heading_with_fitting_block(
        &mut self,
        first_run: usize,
        first_mark: usize,
        heading_before_block: bool,
        style: LineStyle,
        protect_body: bool,
    ) -> Result<(), String> {
        if !heading_before_block
            || self.lines.len().saturating_sub(first_run) < 2
            || !Self::balance_tail_eligible(style, protect_body)
        {
            return Ok(());
        }
        let Some(last_heading_index) = first_run.checked_sub(1) else {
            return Ok(());
        };
        // Move the full chain, including a preceding free thematic rule
        // when that rule reserved its first title. A chain already owned
        // by another heading or a non-free rule must remain in place.
        let (heading_index, chain_mark, leading_rules) =
            if self.heading_attached_to_prior {
                let Some(chain) = self.heading_chain else {
                    return Ok(());
                };
                if (chain.attached_to_prior
                        && chain.leading_rules.is_none())
                    || chain.first_run > last_heading_index
                {
                    return Ok(());
                }
                (chain.first_run, Some(chain.first_mark),
                    chain.leading_rules)
            } else {
                (last_heading_index, None, None)
            };
        let heading = self
            .lines
            .get(heading_index)
            .ok_or("flow heading block origin out of bounds")?;
        let last_heading = self
            .lines
            .get(last_heading_index)
            .ok_or("flow heading block has no final title")?;
        let first = self
            .lines
            .get(first_run)
            .ok_or("flow heading block has no first row")?;
        // Only immediate continuation or a collapsed Markdown paragraph
        // gap can join the heading. Fenced blank rows, earlier structural
        // reservations and a page boundary remain separate.
        let Some(after_heading) =
            last_heading.origin_um.1.checked_add(self.pitch)
        else {
            return Err("flow heading block spacing overflow".into());
        };
        let Some(gap) = first.origin_um.1.checked_sub(after_heading) else {
            return Ok(());
        };
        if heading.page != first.page
            || (gap != 0 && gap != self.paragraph_gap_um) {
            return Ok(());
        }
        let heading_y = heading.origin_um.1;
        let heading_end = heading_y
            .checked_add(self.pitch)
            .ok_or("flow heading ornament boundary overflow")?;
        let first_heading_mark = if let Some(chain) = leading_rules {
            chain.first_mark
        } else if let Some(mark) = chain_mark {
            mark
        } else {
            self.ornaments
                .get(..first_mark)
                .ok_or("flow heading marks out of bounds")?
                .iter()
                .position(|mark| {
                    mark.page == heading.page
                        && (heading_y..heading_end).contains(&mark.start_um.1)
                        && (heading_y..heading_end).contains(&mark.end_um.1)
                })
                .unwrap_or(first_mark)
        };
        let Some(gaps) = self.fitting_heading_gaps(
            heading_index, first_run, last_heading_index,
            leading_rules, gap,
        )? else {
            return Ok(());
        };
        let prefix_rows = leading_rules.map_or(0, |chain| chain.count);
        self.keep_fitting_block_together(
            heading_index, first_heading_mark, style, protect_body,
            &gaps, prefix_rows,
        )
    }

    // Keep a thematic rule and its complete wrapped block together when
    // that span fits a fresh sheet and earlier independent ink remains.
    // The rule owns a measured row but no TextRun; reuse the physical
    // mover with one virtual leading row, preserving its centerline.
    fn keep_rule_with_fitting_block(
        &mut self,
        first_run: usize,
        heading_before_block: bool,
        preceding_reservation: bool,
        style: LineStyle,
        protect_body: bool,
    ) -> Result<(), String> {
        if !preceding_reservation
            || heading_before_block
            || !Self::balance_tail_eligible(style, protect_body)
        {
            return Ok(());
        }
        let Some(chain) = self.rule_chain else {
            return Ok(());
        };
        let Some(first) = self.lines.get(first_run) else {
            return Ok(());
        };
        let Some(gaps) = self.fitting_rule_gaps(
            chain, first.page, first.origin_um.1,
        )? else {
            return Ok(());
        };
        self.keep_fitting_block_together(
            first_run, chain.first_mark, style, protect_body,
            &gaps, chain.count,
        )
    }

    // A wrapped Markdown paragraph, list or quote should keep its first
    // two rows together when possible. Literal code and plain lines retain
    // their exact row boundaries. Never detach a preceding title or rule.
    fn keep_wrapped_block_start(
        &mut self, style: LineStyle, first: bool, protect_body: bool,
    ) -> Result<(), String> {
        // The preceding title or rule already reserved this first row.
        // Its guarantee takes priority over the continuation guard.
        if first && !self.last_visible_heading && !self.last_visible_rule
            && (protect_body && matches!(style, LineStyle::Body)
                || matches!(style, LineStyle::Bullet | LineStyle::Ordered
                    | LineStyle::Quote | LineStyle::Task(_))) {
            let _: bool = self.keep_first_row_with_next(FollowingInkSpacing {
                blank_rows: 0,
                intermediate_rows: 0,
                paragraph_gaps: 0,
            })?;
        }
        Ok(())
    }

    fn layout_line(
        &mut self, content: &str, style: LineStyle, protect_body: bool,
    ) -> Result<(), String> {
        let (run_start, mark_start) = (self.lines.len(), self.ornaments.len());
        let heading_before_block = self.last_visible_heading;
        let preceding_reservation =
            heading_before_block || self.last_visible_rule;
        let (mut start_x, mut cap) = self.flow_row_bounds(style, protect_body)?;
        let (hang, ordered_marker) = ordered_hang_and_label(
            style, content, cap, self.statistics,
        )?;
        let mut row = String::new();
        let mut width = 0u32;
        // Pen contour can outlive a compressed glyph's nominal advance.
        // Reserve that occupied ink before admitting the next word.
        let mut row_ink_extent = 0u32;
        let mut count = 0usize;
        let mut first = true;
        let mut pending_spaces = 0usize;
        for (index, word) in grapheme_word_slices(content)?.iter().enumerate() {
            if index > 0 {
                pending_spaces = pending_spaces.checked_add(1)
                    .ok_or("flow spaces overflow")?;
            }
            if word.is_empty() {
                continue;
            }
            let (glyphs, _, word_span) = measured_word(word, self.statistics)?;
            let spaces_um = u32::try_from(pending_spaces)
                .map_err(|_error| "flow spaces overflow")?
                .checked_mul(4_000).ok_or("flow spaces overflow")?;
            let outside = word_crosses_row(
                style, width, row_ink_extent, spaces_um, word_span, cap,
            )?;
            if !row.is_empty()
                && (outside
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
                self.keep_wrapped_block_start(style, first, protect_body)?;
                self.write_row(take(&mut row), start_x, width, style, first)?;
                start_continuation(first, &mut start_x, &mut cap, hang)?;
                first = false;
                width = 0;
                row_ink_extent = 0;
                count = 0;
                pending_spaces = 0;
            }
            append_spaces(
                &mut row, &mut width, &mut count, pending_spaces, cap,
            )?;
            pending_spaces = 0;
            for (grapheme, advance, slot) in glyphs {
                if !row.is_empty()
                    && (width.checked_add(slot).is_none_or(|end| end > cap)
                        || count == MAX_LINE_GRAPHEMES
                        || row
                            .len()
                            .checked_add(grapheme.len())
                            .is_none_or(|length| length > 384))
                {
                    if matches!(style, LineStyle::Heading(_)) {
                        return Err("heading exceeds one-line fixed region"
                            .into());
                    }
                    self.keep_wrapped_block_start(style, first, protect_body)?;
                    self.write_row(
                        take(&mut row), start_x, width, style, first,
                    )?;
                    start_continuation(first, &mut start_x, &mut cap, hang)?;
                    first = false;
                    width = 0;
                    row_ink_extent = 0;
                    count = 0;
                }
                // A numbered item narrows its continuation to the
                // measured hanging indent. Recheck against that new cap
                // after a wrap, before accepting the next authored shape.
                if slot > cap {
                    return Err(
                        "glyph exceeds horizontal printable region".into()
                    );
                }
                if grapheme.len() > 384 {
                    return Err("flow grapheme exceeds line byte limit".into());
                }
                row_ink_extent = row_ink_extent.max(width.checked_add(slot)
                    .ok_or("flow glyph contour overflow")?);
                row.push_str(grapheme);
                width = width
                    .checked_add(advance)
                    .ok_or("flow glyph advance overflow")?;
                count = count.checked_add(1).ok_or("flow line too long")?;
            }
        }
        self.write_last_row(row, start_x, width, style, first)?;
        self.arrange_wrapped_block(
            run_start, mark_start, heading_before_block,
            preceding_reservation, style, protect_body,
        )
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
                    6_000u32.checked_add(self.list_quote_inset_um)?
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

    fn move_fitting_block_marks(
        &mut self, first_run: usize, first_ornament: usize,
        leading_rules: usize, destination: (usize, u32),
        gaps_after: &[(usize, u32)],
    ) -> Result<(), String> {
        let (new_page, top) = destination;
        let prefix_rows = leading_rules;
        let runs = self.lines.get(first_run..)
            .ok_or("flow short-block origin out of bounds")?;
        for (mark_index, mark) in self.ornaments.get_mut(first_ornament..)
            .ok_or("flow short-block mark origin out of bounds")?
            .iter_mut().enumerate() {
            // Every leading thematic rule has its own physical row but
            // no TextRun. Recreate its centerline at the measured offset.
            if mark_index < leading_rules {
                let offset = Self::fitting_row_offset(
                    self.pitch, mark_index, gaps_after,
                )?;
                let center = top.checked_add(offset)
                    .and_then(|row| row.checked_add(SEPARATOR_CENTER_UM))
                    .ok_or("flow leading-rule position overflow")?;
                mark.start_um.1 = center;
                mark.end_um.1 = center;
                mark.page = new_page;
                continue;
            }
            // Bullets, checkboxes and quote bars lie within their owning
            // text row. Preserve their exact local offsets when moving it.
            let index = runs.iter().position(|row| {
                let within = |y: u32| {
                    row.origin_um.1 <= y
                        && row.origin_um.1.checked_add(self.pitch)
                            .is_some_and(|end| y < end)
                };
                row.page == mark.page
                    && within(mark.start_um.1) && within(mark.end_um.1)
            }).ok_or("flow short-block mark row mismatch")?;
            let old_y = runs.get(index)
                .ok_or("flow short-block mark owner disappeared")?
                .origin_um.1;
            let new_y = top.checked_add(Self::fitting_row_offset(
                self.pitch, index.checked_add(prefix_rows)
                    .ok_or("flow short-block mark index overflow")?,
                gaps_after,
            )?).ok_or("flow short-block mark position overflow")?;
            let translate = |value: u32| -> Result<u32, String> {
                value.checked_sub(old_y)
                    .and_then(|offset| new_y.checked_add(offset))
                    .ok_or_else(|| {
                        "flow short-block mark position overflow".into()
                    })
            };
            mark.start_um.1 = translate(mark.start_um.1)?;
            mark.end_um.1 = translate(mark.end_um.1)?;
            mark.page = new_page;
        }
        Ok(())
    }

    // Record a title's first run and ornament at the start of a chain.
    // Adjacent rows or one selected Markdown paragraph gap may connect
    // titles. Fenced blank rows and physical page breaks end the chain;
    // an earlier title/rule still keeps its reservation.
    fn remember_heading_chain(&mut self, style: LineStyle, y: u32) {
        if !matches!(style, LineStyle::Heading(_)) {
            return;
        }
        let continues = self.last_visible_heading
            && self.lines.last().is_some_and(|prior| {
                prior.page == self.page
                    && prior.origin_um.1.checked_add(self.pitch)
                        .and_then(|after| y.checked_sub(after))
                        .is_some_and(|gap| gap == 0
                            || gap == self.paragraph_gap_um)
            });
        if !continues {
            self.heading_chain = Some(HeadingChain {
                attached_to_prior: self.last_visible_rule
                    || self.last_visible_heading,
                first_mark: self.ornaments.len(),
                first_run: self.lines.len(),
                leading_rules: if self.last_visible_rule {
                    self.rule_chain
                } else {
                    None
                },
            });
        }
    }

    fn write_last_row(
        &mut self, row: String, start_x: u32, width: u32,
        style: LineStyle, first: bool,
    ) -> Result<(), String> {
        if !row.is_empty() {
            if matches!(style, LineStyle::Heading(_)) && !first {
                return Err("heading exceeds one-line fixed region".into());
            }
            self.write_row(row, start_x, width, style, first)?;
        }
        Ok(())
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
        self.remember_heading_chain(style, y);
        if let LineStyle::Heading(level) = style {
            // A compressed letter advance may end well before its visible
            // glyph contour. Decorations must cover actual authored ink,
            // not only the cursor position after the final grapheme.
            let ink_extent = heading_contour_extent(
                &text, extent, self.statistics,
            )?;
            let mark_end = start_x
                .checked_add(ink_extent)
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
        if matches!(style, LineStyle::Heading(_)) {
            self.heading_attached_to_prior =
                self.last_visible_heading || self.last_visible_rule;
        }
        self.last_visible_heading = matches!(style, LineStyle::Heading(_));
        self.last_visible_rule = false;
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

    fn write_separator(&mut self, reserved_next: bool) -> Result<(), String> {
        let (start_x, cap) = self.line_measurements(LineStyle::Body)?;
        let y = self.begin_row()?;
        let rule_y = y.checked_add(SEPARATOR_CENTER_UM)
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
        self.last_visible_heading = false;
        self.heading_chain = None;
        let continuing = self.last_visible_rule;
        self.rule_chain = if continuing {
            let chain = self.rule_chain
                .ok_or("flow reserved rule chain missing")?;
            Some(RuleChain {
                count: chain.count.checked_add(1)
                    .ok_or("flow rule-chain count overflow")?,
                ..chain
            })
        } else if reserved_next {
            Some(RuleChain {
                count: 1,
                first_mark: self.ornaments.len().checked_sub(1)
                    .ok_or("flow rule-chain mark missing")?,
            })
        } else {
            None
        };
        self.last_visible_rule = reserved_next || continuing;
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

// A non-numbered block has no authored marker or hanging indent.
// Numbered labels retain their measured width and exact glyph source.
fn ordered_hang_and_label<'text>(
    style: LineStyle, content: &'text str, cap: u32,
    statistics: Option<&HandwritingStatisticsProfile>,
) -> Result<(u32, Option<&'text str>), String> {
    if matches!(style, LineStyle::Ordered) {
        let (indent, marker) = measured_ordered_label(
            content, cap, statistics,
        )?;
        Ok((indent, Some(marker)))
    } else {
        Ok((0, None))
    }
}

// Keep the authored marker and at least the first body glyph together.
// Continuations start after the actual measured numeral and separator spaces.
fn measured_ordered_label<'text>(
    content: &'text str,
    cap: u32,
    statistics: Option<&HandwritingStatisticsProfile>,
) -> Result<(u32, &'text str), String> {
    let (marker, _) = content.split_once(' ')
        .ok_or("ordered list requires a numeral and text")?;
    // The separating ASCII space can itself be part of an authored
    // grapheme (space + combining mark). Its space advance still counts
    // toward the hanging indent; its attached ink must be measured whole.
    let mut item = content.get(marker.len()..)
        .ok_or("ordered label item boundary refused")?;
    let mut spaces = 0usize;
    while let Ok((" ", rest)) = first_grapheme(item) {
        item = rest;
        spaces = spaces.checked_add(1)
            .ok_or("ordered label spacing overflow")?;
    }
    let first_item_spaces = spaces;
    if item.starts_with(' ') {
        spaces = spaces.checked_add(1)
            .ok_or("ordered label spacing overflow")?;
    }
    let (marker_glyphs, marker_width, _) =
        measured_word(marker, statistics)?;
    // A compressed advance can be shorter than the actual pen contour.
    // Reserve the whole numeral and delimiter before laying out any row;
    // they must never wrap away from the first item glyph.
    let mut marker_offset = 0u32;
    let mut marker_extent = 0u32;
    for (_, advance, slot) in marker_glyphs {
        let end = marker_offset.checked_add(slot)
            .ok_or("ordered label contour overflow")?;
        if end > cap {
            return Err(
                "ordered list label and first glyph exceed horizontal region"
                    .into()
            );
        }
        marker_extent = marker_extent.max(end);
        marker_offset = marker_offset.checked_add(advance)
            .ok_or("ordered label advance overflow")?;
    }
    let indent = u32::try_from(spaces)
        .map_err(|_error| "ordered label spacing overflow")?
        .checked_mul(4_000)
        .and_then(|space_width| marker_width.checked_add(space_width))
        .ok_or("ordered label advance overflow")?;
    // A negative letter-spacing mean can compress nominal advances far
    // below the pen contour. Respect the authored label's exact spaces,
    // but refuse when the first item glyph would overlap the numeral ink.
    if indent < marker_extent {
        return Err("ordered list marker ink overlaps first item glyph".into());
    }
    let (first_glyph, _) = first_grapheme(item)
        .map_err(|_error| "ordered list requires visible item text")?;
    let (glyphs, _, _) = measured_word(first_glyph, statistics)?;
    let first_slot = glyphs.first()
        .ok_or("ordered list requires visible item text")?.2;
    // A marked space advances the continuation indent, but its ink
    // begins before its own advance on the first physical row.
    let first_item_origin = u32::try_from(first_item_spaces)
        .map_err(|_error| "ordered label spacing overflow")?
        .checked_mul(4_000)
        .and_then(|padding| marker_width.checked_add(padding))
        .ok_or("ordered label first glyph position overflow")?;
    if first_item_origin.checked_add(first_slot)
        .is_none_or(|end| end > cap) {
        return Err(
            "ordered list label and first glyph exceed horizontal region".into()
        );
    }
    Ok((indent, marker))
}

// Flowing prose reserves a preceding compressed glyph's nominal ink contour.
// Decorative titles are indivisible lines with intentionally adjustable
// letter overlap; they still obey the physical row width and native ink gates.
fn word_crosses_row(
    style: LineStyle, advance: u32, ink_extent: u32, spaces: u32,
    word_span: u32, cap: u32,
) -> Result<bool, String> {
    let start = advance.checked_add(spaces)
        .ok_or("flow word position overflow")?;
    Ok((!matches!(style, LineStyle::Heading(_)) && ink_extent > start)
        || start.checked_add(word_span).is_none_or(|end| end > cap))
}

// Break only on a standalone ASCII-space grapheme. Splitting on the
// scalar character would detach a following combining mark from its base,
// changing the exact authored glyph identity when a physical row wraps.
fn grapheme_word_slices(content: &str) -> Result<Vec<&str>, String> {
    let mut words = Vec::new();
    let mut start = 0usize;
    let mut offset = 0usize;
    while offset < content.len() {
        let suffix = content.get(offset..)
            .ok_or("flow word suffix refused")?;
        let (grapheme, _) = first_grapheme(suffix)?;
        if grapheme == " " {
            words.push(content.get(start..offset)
                .ok_or("flow word boundary refused")?);
            start = offset.checked_add(grapheme.len())
                .ok_or("flow word offset overflow")?;
        }
        offset = offset.checked_add(grapheme.len())
            .ok_or("flow word offset overflow")?;
    }
    words.push(content.get(start..)
        .ok_or("flow word suffix refused")?);
    Ok(words)
}

fn measured_word<'text>(
    word: &'text str,
    statistics: Option<&HandwritingStatisticsProfile>,
) -> Result<(Vec<MeasuredGrapheme<'text>>, u32, u32), String> {
    let mut glyphs = Vec::new();
    let mut width = 0u32;
    let mut occupied = 0u32;
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
        // Word admission must include the farthest physical glyph slot,
        // including compressed advances. Otherwise a separator is added
        // before discovering that the next word needs a new row.
        occupied = occupied.max(width.checked_add(slot)
            .ok_or("flow word contour overflow")?);
        glyphs.push((grapheme, advance, slot));
        width = width
            .checked_add(advance)
            .ok_or("flow word width overflow")?;
    }
    Ok((glyphs, width, occupied))
}

// Use the farthest visible glyph edge for heading ornaments. A glyph
// with negative letter spacing can extend beyond later glyphs and beyond
// the summed advances. Ordinary rows still use their original positions.
fn heading_contour_extent(
    text: &str, advances: u32,
    statistics: Option<&HandwritingStatisticsProfile>,
) -> Result<u32, String> {
    let mut cursor = 0u32;
    let mut extent = advances;
    let mut remaining = text;
    while !remaining.is_empty() {
        let (grapheme, rest) = first_grapheme(remaining)?;
        let (advance, contour) = GlyphOverlay::flow_advance_um(
            grapheme, statistics,
        )?;
        extent = extent.max(cursor.checked_add(contour)
            .ok_or("heading glyph contour overflow")?);
        cursor = cursor.checked_add(advance)
            .ok_or("heading glyph advance overflow")?;
        remaining = rest;
    }
    Ok(extent)
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

// A space with attached combining marks is a distinct authored grapheme,
// not a Markdown delimiter or disposable indentation. Match exact Unicode
// grapheme boundaries before removing any marker-padding spaces.
fn strip_standalone_spaces(mut text: &str, limit: usize) -> &str {
    for _ in 0..limit {
        match first_grapheme(text) {
            Ok((" ", rest)) => text = rest,
            _ => break,
        }
    }
    text
}

// Keep Unicode-aware marker-padding handling shared by bullets and tasks.
fn markdown_bullet_line(block: &str) -> Option<(LineStyle, Cow<'_, str>)> {
    // Empty list markers cannot produce a visible handwritten block.
    if matches!(block, "-" | "*" | "+") {
        return Some((LineStyle::Bullet, Cow::Borrowed("")));
    }
    let bullet = block.strip_prefix('-')
        .or_else(|| block.strip_prefix('*'))
        .or_else(|| block.strip_prefix('+'))?;
    if !bullet.starts_with(' ') {
        return None;
    }
    // Only the first four standalone spaces delimit a list item.
    // A space with attached ink is an authored grapheme, not padding.
    let content = strip_standalone_spaces(bullet, 4);
    for (marker, checked) in [("[ ]", false), ("[x]", true),
        ("[X]", true)] {
        if let Some(after) = content.strip_prefix(marker)
            && (after.is_empty() || after.starts_with(' ')) {
            return Some((LineStyle::Task(checked), Cow::Borrowed(
                strip_standalone_spaces(after, after.len())
                    .trim_end_matches(' '),
            )));
        }
    }
    Some((LineStyle::Bullet, Cow::Borrowed(
        content.trim_end_matches(' '),
    )))
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
        strip_standalone_spaces(line, indent)
    } else {
        line
    };
    // A single leading backslash escapes a syntactic first-line marker.
    // Keep the marker as actual authored ink, but not the escape itself.
    if indent <= 3
        && let Some(escaped) = line.trim_start_matches(' ')
            .strip_prefix('\\')
        && escaped.bytes().next().is_some_and(|mark| {
            matches!(mark, b'#' | b'>' | b'-' | b'*' | b'+' | b'_' | b'=')
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
                    heading_body(strip_standalone_spaces(body, body.len()))));
            }
        }
    }
    if let Some(list) = markdown_bullet_line(block) {
        return list;
    }
    if let Some(quote) = block.strip_prefix('>') {
        // One level of Markdown quote syntax, optionally followed by one
        // separator space. Nested > markers remain literal authored text.
        return (LineStyle::Quote, Cow::Borrowed(
            strip_standalone_spaces(quote, 1).trim_end_matches(' '),
        ));
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

// Only a final backslash directly at the line boundary is hard-break
// syntax. A backslash preceding two trailing spaces remains authored ink:
// those spaces already supply the break. Pairs and longer runs stay literal;
// this parser does not perform general inline Markdown unescaping.
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
    if raw.ends_with('\\')
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

// Reserve the first following visible row for headings, or a free
// thematic rule and its consecutive rules. A rule chain must account
// for each rule's physical row and collapsed gap before its first body
// row, rather than reserving just the next separator. No printable
// content after terminal rules means there is no *following* ink, but
// the rules themselves remain physical rows that can stay together.
fn following_gap<'text>(
    lines: impl Iterator<Item = &'text str>, include_rules: bool,
    include_terminal_rules: bool,
) -> Option<FollowingInkSpacing> {
    let mut following = FollowingInkSpacing {
        blank_rows: 0,
        intermediate_rows: 0,
        paragraph_gaps: 0,
    };
    let mut fence = None;
    let mut pending_gap = false;
    let mut pending_blank_rows = 0usize;
    let mut saw_non_rule = false;
    let mut lookahead = lines.peekable();
    while let Some(line) = lookahead.next() {
        if markdown_fence_transition(&mut fence, line) {
            // Fence delimiters themselves have no physical row.
            continue;
        }
        if fence.is_some() && line.trim_matches(' ').is_empty() {
            pending_blank_rows = pending_blank_rows.saturating_add(1);
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
        // Include fenced blank rows only when visible ink follows them.
        // Interior rows must count when grouping terminal headings, while
        // a final empty code fence must not force an ink-free page.
        following.blank_rows = following.blank_rows
            .saturating_add(pending_blank_rows);
        pending_blank_rows = 0;
        if fence.is_some() {
            return Some(following);
        }
        let (style, body) = markdown_line(line);
        if include_rules && matches!(style, LineStyle::Separator) {
            following.intermediate_rows =
                following.intermediate_rows.saturating_add(1);
        } else if matches!(style, LineStyle::Heading(_)) {
            saw_non_rule = true;
            following.intermediate_rows =
                following.intermediate_rows.saturating_add(1);
        } else if matches!(style, LineStyle::Body)
            && take_following_setext(line, body.as_ref(), &mut lookahead)
        {
            // A soft-joined Setext title contributes one physical row.
            saw_non_rule = true;
            following.intermediate_rows =
                following.intermediate_rows.saturating_add(1);
        } else {
            return Some(following);
        }
    }
    // Terminal titles and rule-only chains are printable ink, even
    // when no later body row exists. Ignore trailing source blanks.
    (following.intermediate_rows > 0
        && (!include_rules || saw_non_rule || include_terminal_rules))
        .then_some(FollowingInkSpacing {
            blank_rows: following.blank_rows,
            intermediate_rows: following.intermediate_rows.saturating_sub(1),
            paragraph_gaps: following.paragraph_gaps,
        })
}

// A completed paragraph can be promoted into a Setext heading only
// before a closing delimiter, never inside a fence or quote block.
fn handle_markdown_setext<'text>(
    layout: &mut Layout<'_>, paragraph: &mut String,
    style: LineStyle, input_line: &str,
    following: impl Iterator<Item = &'text str> + Clone,
) -> Result<bool, String> {
    if paragraph.is_empty() || !matches!(style, LineStyle::Body) {
        return Ok(false);
    }
    let Some(level) = markdown_setext_level(input_line) else {
        return Ok(false);
    };
    reserve_heading_first_row(
        layout, LineStyle::Heading(level), following,
    )?;
    layout.layout_line(&take(paragraph), LineStyle::Heading(level), false)?;
    Ok(true)
}

// End a source paragraph at its structural boundary or explicit hard break.
// Keep the authored text buffer separate from physical rows and decorations.
fn flush_paragraph(
    layout: &mut Layout<'_>, paragraph: &mut String, style: LineStyle,
) -> Result<(), String> {
    if !paragraph.is_empty() {
        layout.layout_line(paragraph, style, true)?;
        paragraph.clear();
    }
    Ok(())
}

// A thematic rule begins a new visual block. Keep the rule with its
// next printed row if that fits on a fresh sheet, but never detach it
// from a preceding title or rule that reserved this first row.
fn layout_thematic_separator<'text>(
    layout: &mut Layout<'_>, paragraph: &mut String,
    previous: &mut LineStyle,
    following: impl Iterator<Item = &'text str> + Clone,
) -> Result<(), String> {
    flush_paragraph(layout, paragraph, *previous)?;
    let reserved_next = if !layout.last_visible_heading
        && !layout.last_visible_rule {
        // Prefer the complete rule chain plus its first visible ink,
        // or all terminal rules when no later text follows. When that
        // cannot fit a fresh sheet, retain the older first-rule/next-
        // row pairing rather than detaching later rules.
        let full = match following_gap(following.clone(), true, true) {
            Some(gap) => layout.keep_first_row_with_next(gap)?,
            None => false,
        };
        if full {
            true
        } else {
            match following_gap(following, false, false) {
                Some(gap) => layout.keep_first_row_with_next(gap)?,
                None => false,
            }
        }
    } else {
        false
    };
    layout.write_separator(reserved_next)?;
    *previous = LineStyle::Separator;
    Ok(())
}

// Terminal headings may alternate with thematic rules. Reserve the full
// fitting title/rule group, never a fenced row or an intervening
// paragraph/list/quote, and do not count Setext ink as a separate rule.
fn terminal_rule_chain<'text>(
    following: impl Iterator<Item = &'text str>,
) -> bool {
    let mut rules = 0usize;
    let mut lookahead = following.peekable();
    while let Some(line) = lookahead.next() {
        if line.trim_end_matches(' ').is_empty() {
            continue;
        }
        let (style, body) = markdown_line(line);
        match style {
            LineStyle::Separator => rules = rules.saturating_add(1),
            LineStyle::Heading(_) => {},
            LineStyle::Body if take_following_setext(
                line, body.as_ref(), &mut lookahead,
            ) => {},
            LineStyle::Body | LineStyle::Bullet | LineStyle::Ordered
            | LineStyle::Quote | LineStyle::Task(_) => {
                return false;
            }
        }
    }
    rules > 0
}

// A free heading and its trailing rules remain together if the group
// fits one page. Otherwise keep the older first-following-ink priority.
// Do not extend a title already owned by a preceding title or rule.
fn reserve_heading_first_row<'text>(
    layout: &mut Layout<'_>, style: LineStyle,
    following: impl Iterator<Item = &'text str> + Clone,
) -> Result<(), String> {
    if matches!(style, LineStyle::Heading(_))
        && !layout.last_visible_rule {
        if !layout.last_visible_heading
            && terminal_rule_chain(following.clone())
            && let Some(group) = following_gap(following.clone(), true, true)
            && layout.keep_first_row_with_next(group)? {
            return Ok(());
        }
        if let Some(gap) = following_gap(following, false, false) {
            let _: bool = layout.keep_first_row_with_next(gap)?;
        }
    }
    Ok(())
}

// Plain blank lines reserve physical rows; Markdown blanks instead
// flush the current paragraph and defer one collapsed paragraph gap.
fn handle_source_blank(
    layout: &mut Layout<'_>, paragraph: &mut String,
    style: LineStyle, format: FlowFormat, pending_gap: &mut bool,
) -> Result<(), String> {
    if format == FlowFormat::Plain {
        layout.blank_row()
    } else {
        flush_paragraph(layout, paragraph, style)?;
        *pending_gap = layout.row_count > 0;
        Ok(())
    }
}

// Consecutive syntactic blank quotes collapse into one paragraph gap.
// They need quoted ink on both sides; no empty marker reserves a row or
// silently consumes an authored plain-text >.
fn admit_empty_quote_gap<'text>(
    layout: &mut Layout<'_>, paragraph: &mut String,
    previous: LineStyle, format: FlowFormat, style: LineStyle,
    original_body: &str, following: impl Iterator<Item = &'text str>,
) -> Result<(), String> {
    // A line whose sole backslash became a hard-break marker is not an
    // empty quote delimiter. Do not silently turn its syntax into a gap.
    if !original_body.trim_matches(' ').is_empty() {
        return Err("empty Markdown block refused".into());
    }
    // The source byte cap bounds this lookahead even for a long series
    // of virtual blank markers. Stop before any unrelated block or row.
    // Ordinary blank lines and empty quote markers are the same
    // collapsed paragraph boundary. Permit them to interleave, but stop
    // at the first different visible or structural block.
    let next = following.into_iter()
        .filter(|line| !line.trim_end_matches(' ').is_empty())
        .map(markdown_line)
        .find(|(kind, body)| !matches!(kind, LineStyle::Quote)
            || !body.trim_matches(' ').is_empty());
    if format != FlowFormat::Markdown
        || !matches!(style, LineStyle::Quote)
        || !matches!(previous, LineStyle::Quote)
        || !next.is_some_and(|(kind, body)| {
            matches!(kind, LineStyle::Quote)
                && !body.trim_matches(' ').is_empty()
        })
    {
        return Err("empty Markdown block refused".into());
    }
    flush_paragraph(layout, paragraph, previous)
}

// Four or more leading ASCII spaces are literal physical placement, not
// Markdown soft continuation. They neither absorb a pending paragraph nor
// gain a synthetic joining space before their first authored glyph.
fn layout_literal_indented_row(
    layout: &mut Layout<'_>, paragraph: &mut String,
    previous: &mut LineStyle, raw: &str, style: LineStyle, body: &str,
) -> Result<bool, String> {
    if !matches!(style, LineStyle::Body) || !raw.starts_with("    ") {
        return Ok(false);
    }
    flush_paragraph(layout, paragraph, *previous)?;
    layout.layout_line(body, style, false)?;
    *previous = LineStyle::Body;
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

// Two/three-space-indented body rows can explicitly continue an open
// blockquote paragraph under its physical rule. Unindented ordinary rows
// still end that quote, preserving the CLI's established separation.
// Explicit breaks, literal four-space rows and new blocks also end it.
fn continue_quote_paragraph(
    layout: &mut Layout<'_>, paragraph: &mut String, previous: LineStyle,
    line: &str, style: LineStyle, body: &str, hard_break: bool,
) -> Result<bool, String> {
    if paragraph.is_empty() || !matches!(previous, LineStyle::Quote)
        || !matches!(style, LineStyle::Body)
        || !(2..=3).contains(&line.bytes()
            .take_while(|&byte| byte == b' ').count())
        || markdown_setext_level(line).is_some() {
        return Ok(false);
    }
    paragraph.push(' ');
    paragraph.push_str(body);
    if hard_break {
        flush_paragraph(layout, paragraph, previous)?;
    }
    Ok(true)
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
    paragraph_style: &mut LineStyle,
    pending_gap: &mut bool,
    fence: &mut Option<(u8, usize)>,
    input_line: &str,
) -> Result<bool, String> {
    if markdown_fence_transition(fence, input_line) {
        flush_paragraph(layout, paragraph, *paragraph_style)?;
        *paragraph_style = LineStyle::Body;
        return Ok(true);
    }
    if fence.is_none() {
        return Ok(false);
    }
    // Fenced lines are literal body ink, never a quoted paragraph.
    *paragraph_style = LineStyle::Body;
    if *pending_gap {
        layout.add_gap()?;
        *pending_gap = false;
    }
    if input_line.trim_matches(' ').is_empty() {
        layout.blank_row()?;
    } else {
        layout.layout_line(input_line, LineStyle::Body, false)?;
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
    source.validate_canonical()?;
    validate_flow_statistics(statistics)?;
    let mut pitch = DEFAULT_PITCH_UM;
    let mut fence = None;
    let mut pending_setext = false;
    let mut pending_list = false;
    let mut pending_quote = false;
    for raw in source.text.lines() {
        let visible = if source.format == FlowFormat::Markdown {
            if markdown_fence_transition(&mut fence, raw) {
                pending_setext = false;
                pending_list = false;
                pending_quote = false;
                continue;
            }
            if fence.is_some() {
                pending_setext = false;
                pending_list = false;
                pending_quote = false;
                Cow::Borrowed(raw)
            } else if pending_setext && markdown_setext_level(raw).is_some() {
                pending_setext = false;
                pending_quote = false;
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
                let quote_continuation = pending_quote
                    && (2..=3).contains(&indent)
                    && matches!(style, LineStyle::Body)
                    && markdown_setext_level(raw).is_none();
                // The physical paragraph compiler inserts a separating
                // space on soft joins. A leading combining mark then
                // belongs to that *new* space grapheme, not the standalone
                // source-line grapheme. Measure that exact resulting ink.
                let joins_paragraph = list_continuation || quote_continuation
                    || (pending_setext && indent < 4
                        && matches!(style, LineStyle::Body))
                    || (pending_quote && matches!(style, LineStyle::Quote));
                pending_list = !hard_break && (list_continuation
                    || matches!(style, LineStyle::Bullet | LineStyle::Ordered
                        | LineStyle::Task(_)));
                pending_quote = !hard_break && (quote_continuation
                    || matches!(style, LineStyle::Quote));
                // List and quote continuations remain structural blocks,
                // not body paragraphs eligible for Setext promotion.
                // Four-space literal rows were also laid out immediately.
                pending_setext = !list_continuation && !quote_continuation
                    && matches!(style, LineStyle::Body)
                    && indent < 4
                    && !visible.trim_matches(' ').is_empty()
                    && !hard_break;
                Cow::Owned(if joins_paragraph {
                    format!(" {visible}")
                } else {
                    visible.to_owned()
                })
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
    heading: HeadingDecoration, format: FlowFormat,
    region: Option<FlowRegion>, pitch: u32, spacing: FlowTypography,
) -> Result<Layout<'model>, String> {
    heading.validate().map_err(str::to_owned)?;
    if heading.is_selected() && format != FlowFormat::Markdown {
        return Err("heading customization requires --flow-markdown".into());
    }
    let printable = profile.printable_rect_um()?;
    let (left, top, right, bottom) = if let Some(fixed) = region {
        fixed.bounds_within(printable)?
    } else {
        printable
    };
    Ok(Layout {
        body_inset_um: spacing.body_inset.unwrap_or(0),
        body_right_inset_um: spacing.body_right_inset.unwrap_or(0),
        list_quote_inset_um: spacing.list_quote_inset.unwrap_or(0),
        list_quote_right_inset_um: spacing.list_quote_right_inset.unwrap_or(0),
        heading,
        heading_attached_to_prior: false,
        heading_chain: None,
        rule_chain: None,
        last_visible_heading: false,
        last_visible_rule: false,
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
        paragraph_gap_um: spacing.paragraph_gap
            .unwrap_or(DEFAULT_PARAGRAPH_GAP_UM),
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

// Typed callers can mutate an otherwise admitted statistics profile.
// Recheck its bounded dimension and grapheme contract before layout.
fn validate_flow_statistics(
    statistics: Option<&HandwritingStatisticsProfile>,
) -> Result<(), String> {
    if let Some(profile) = statistics {
        profile.validate(&UnicodeGraphemeSegmentation).map_err(|error| {
            format!("invalid flow handwriting statistics: {error:?}")
        })?;
    }
    Ok(())
}

/// An optional fixed rectangle constrains every page's text and decoration.
/// The whole printable rectangle remains the default when none is selected.
#[expect(clippy::redundant_pub_crate, reason = "Sibling CLI module access")]
#[cfg(test)]
pub(super) fn layout_flow_with_region(
    source: &FlowSource,
    profile: &NotebookProfile,
    statistics: Option<&HandwritingStatisticsProfile>,
    decoration: HeadingDecoration,
    region: Option<FlowRegion>,
) -> Result<(Vec<TextRun>, Vec<NotebookInkLine>, usize), String> {
    layout_flow_with_min_pitch(
        source, profile, statistics, decoration, region, None,
    )
}

// Validate typed callers too; CLI preflight performs the same bound check.
fn measured_pitch_with_minimum(
    source: &FlowSource,
    statistics: Option<&HandwritingStatisticsProfile>,
    minimum: Option<u32>,
) -> Result<u32, String> {
    if minimum.is_some_and(|pitch| {
        !(DEFAULT_PITCH_UM..=MAX_SELECTED_PITCH_UM).contains(&pitch)
    }) {
        return Err("flow pitch must be within 9000..30000 um".into());
    }
    Ok(measured_pitch(source, statistics)?
        .max(minimum.unwrap_or(DEFAULT_PITCH_UM)))
}

/// This explicit spacing option changes only collapsed Markdown paragraph
/// gaps, not physical row pitch, plain-text blank rows, or saved profiles.
/// Existing callers continue to receive the default 4-mm gap.
#[expect(clippy::redundant_pub_crate, reason = "Sibling CLI module access")]
#[cfg(test)]
pub(super) fn layout_flow_with_min_pitch(
    source: &FlowSource,
    profile: &NotebookProfile,
    statistics: Option<&HandwritingStatisticsProfile>,
    decoration: HeadingDecoration,
    region: Option<FlowRegion>,
    min_pitch_um: Option<u32>,
) -> Result<(Vec<TextRun>, Vec<NotebookInkLine>, usize), String> {
    layout_flow_with_spacing(
        source, profile, statistics, decoration, region, min_pitch_um, None,
    )
}

/// The selected minimum row pitch cannot compress measured glyph clearance.
/// The paragraph gap is a bounded export-only spacing policy.
#[expect(clippy::redundant_pub_crate, reason = "Sibling CLI module access")]
#[cfg(test)]
pub(super) fn layout_flow_with_spacing(
    source: &FlowSource,
    profile: &NotebookProfile,
    statistics: Option<&HandwritingStatisticsProfile>,
    decoration: HeadingDecoration,
    region: Option<FlowRegion>,
    min_pitch_um: Option<u32>,
    paragraph_gap_um: Option<u32>,
) -> Result<(Vec<TextRun>, Vec<NotebookInkLine>, usize), String> {
    layout_flow_with_typography(
        source, profile, statistics, decoration, region, FlowTypography {
            pitch: min_pitch_um, paragraph_gap: paragraph_gap_um,
            ..FlowTypography::default()
        },
    )
}

/// An optional physical inset applies to every row of a Markdown prose
/// paragraph. It does not shift structural markers or literal code rows.
#[expect(clippy::redundant_pub_crate, reason = "Sibling CLI module access")]
pub(super) fn layout_flow_with_typography(
    source: &FlowSource,
    profile: &NotebookProfile,
    statistics: Option<&HandwritingStatisticsProfile>,
    decoration: HeadingDecoration,
    region: Option<FlowRegion>,
    spacing: FlowTypography,
) -> Result<(Vec<TextRun>, Vec<NotebookInkLine>, usize), String> {
    if spacing.paragraph_gap
        .is_some_and(|gap| gap > MAX_PARAGRAPH_GAP_UM) {
        return Err("flow paragraph gap must be within 0..12000 um".into());
    }
    if spacing.paragraph_gap.is_some()
        && source.format != FlowFormat::Markdown {
        return Err("flow paragraph gap requires Markdown flow".into());
    }
    if spacing.body_inset
        .is_some_and(|inset| inset > MAX_BODY_INSET_UM) {
        return Err("flow body inset must be within 0..12000 um".into());
    }
    if spacing.body_inset.is_some()
        && source.format != FlowFormat::Markdown {
        return Err("flow body inset requires Markdown flow".into());
    }
    if spacing.body_right_inset
        .is_some_and(|inset| inset > MAX_BODY_INSET_UM) {
        return Err("flow body right inset must be within 0..12000 um".into());
    }
    if spacing.body_right_inset.is_some()
        && source.format != FlowFormat::Markdown {
        return Err("flow body right inset requires Markdown flow".into());
    }
    if spacing.list_quote_inset
        .is_some_and(|inset| inset > MAX_BODY_INSET_UM) {
        return Err("flow list/quote inset must be within 0..12000 um".into());
    }
    if spacing.list_quote_inset.is_some()
        && source.format != FlowFormat::Markdown {
        return Err("flow list/quote inset requires Markdown flow".into());
    }
    if spacing.list_quote_right_inset
        .is_some_and(|inset| inset > MAX_BODY_INSET_UM) {
        return Err(
            "flow list/quote right inset must be within 0..12000 um".into()
        );
    }
    if spacing.list_quote_right_inset.is_some()
        && source.format != FlowFormat::Markdown {
        return Err("flow list/quote right inset requires Markdown flow".into());
    }
    let pitch = measured_pitch_with_minimum(
        source, statistics, spacing.pitch,
    )?;
    let layout = initialize_layout(
        profile, statistics, decoration, source.format, region,
        pitch, spacing,
    )?;
    compose_flow_rows(source, layout)
}

// Parse physical rows only after both selected spacing policies are admitted.
fn compose_flow_rows(
    source: &FlowSource, mut layout: Layout<'_>,
) -> Result<(Vec<TextRun>, Vec<NotebookInkLine>, usize), String> {
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
                &mut paragraph_style, &mut pending_gap, &mut fence,
                input_line)? {
            continue;
        }
        if source.format == FlowFormat::Markdown
            && handle_markdown_setext(&mut layout, &mut paragraph,
                paragraph_style, input_line, source_lines.clone())? {
            continue;
        }
        let raw = input_line.trim_end_matches(' ');
        if raw.is_empty() {
            handle_source_blank(&mut layout, &mut paragraph,
                paragraph_style, source.format, &mut pending_gap)?;
            continue;
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
        // A syntactic empty quote extends an already pending paragraph
        // gap; it does not consume one before creating another. A thematic
        // separator, by contrast, is real ink and receives the pending gap.
        if body.trim_matches(' ').is_empty()
            && !matches!(style, LineStyle::Separator) {
            admit_empty_quote_gap(&mut layout, &mut paragraph,
                paragraph_style, source.format, style,
                parsed_body.as_ref(), source_lines.clone())?;
            pending_gap = layout.row_count > 0;
            continue;
        }
        if take(&mut pending_gap) {
            layout.add_gap()?;
        }
        if matches!(style, LineStyle::Separator) {
            layout_thematic_separator(&mut layout, &mut paragraph,
                &mut paragraph_style, source_lines.clone())?;
            continue;
        }
        if source.format == FlowFormat::Markdown
            && layout_literal_indented_row(&mut layout, &mut paragraph,
                &mut paragraph_style, input_line, style, body)? {
            continue;
        }
        if source.format == FlowFormat::Markdown
            && (continue_list_item(&mut layout, &mut paragraph,
                paragraph_style, input_line, style, body, hard_break)?
            || continue_quote_paragraph(&mut layout, &mut paragraph,
                paragraph_style, input_line, style, body, hard_break)?) {
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
            reserve_heading_first_row(
                &mut layout, style, source_lines.clone(),
            )?;
            if source.format == FlowFormat::Markdown
                && matches!(style,
                    LineStyle::Bullet | LineStyle::Ordered | LineStyle::Task(_))
            {
                queue_list_item(&mut layout, &mut paragraph,
                    &mut paragraph_style, style, body, hard_break)?;
            } else {
                layout.layout_line(body, style, false)?;
                paragraph_style = style;
            }
        }
    }
    finish_layout(layout, paragraph, paragraph_style, fence)
}
