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
//   - Rust CLI editing and inventory inspection of handwriting profiles.
// - Must-Not:
//   - Invent glyph outlines, infer calibration, or silently sample ink.
// - Allows:
//   - Inputs: Explicit CLI arguments and bounded caller-selected JSON files.
//   - Outputs: Admitted JSON, exact text-inventory audit, or refusal.
//   - Side effects: Explicit atomic output file commit, or stdout write.
// - Split-When:
//   - Interactive profile mutation becomes an application service.
// - Merge-When:
//   - A full notebook CLI subsumes both profile families.
// - Summary:
//   - Provides one executable path to edit/save Rust paper and ink controls.
// - Description:
//   - Delegates paper to the existing CLI and statistics to shared Rust codecs.
// - Usage:
//   - atrament-profiles handwriting --seed 1 --output ink.json
//   - atrament-profiles paper --emit-profile --output paper.json
// - Defaults:
//   - Handwriting baseline is explicitly uncalibrated; no random samples.
//

//! Single local CLI boundary for admitted paper and handwriting profiles.

mod flow;
mod glyph;

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::error::Error;
use std::fmt::Write as _;
use std::io::{self, Read as _, Write as _};
use std::mem::replace;
use std::path::Path;
use std::str::from_utf8;
use std::slice::{Iter, from_ref};

#[cfg(test)]
use flow::{
    layout_flow, layout_flow_with_min_pitch, layout_flow_with_region,
    layout_flow_with_spacing,
};
use flow::{
    BodyAlign, BulletStyle, FlowFormat, FlowRegion, FlowSource, FlowTypography,
    TaskStyle,
    HeadingDecoration,
    layout_flow_with_typography,
};
use glyph::{GlyphVariation, first_grapheme};

use atrament_english_spanish_grapheme_inventory::REQUIRED_TEXT_GRAPHEMES;
use atrament_handwriting_statistics::{
    Dimension, DimensionStatistics, HandwritingStatisticsProfile,
    ScalarStatistics, Unit, preview_value,
};
use atrament_handwriting_statistics_json::{
    MAXIMUM_STATISTICS_JSON_BYTES, decode_statistics_json,
    encode_statistics_json,
};
use atrament_notebook_paper_cli::{
    MAX_NOTEBOOK_INK_LINES, MAX_PAPER_OUTPUT_BYTES, NotebookInkLine,
    RulerVariation,
    canonical_paper_profile_json, validate_printed_date_text,
};
use atrament_notebook_profile::{
    DatePlacement, DateStyle, MAX_NOTEBOOK_JSON_BYTES, NotebookProfile,
};
use atrament_profile_file_commit::{
    ProfileFileCommitDisposition, commit_profile_file,
};
use atrament_profile_file_reader::read_bounded_profile_file;
use atrament_profile_selected_file_open::open_selected_profile_file;
#[cfg(test)]
use atrament_unicode_grapheme_boundary_port::GraphemeBoundaryProvider as _;
use atrament_unicode_grapheme_segmentation::UnicodeGraphemeSegmentation;

const MAX_INVENTORY_TEXT_BYTES: usize = 4096;
const MAX_GLYPH_LIBRARY_BYTES: usize = 524_288;
const MAX_PAGE_INK_FILE_BYTES: usize = 262_144;
const PAGE_INK_FILE_HEADER: &str = "atrament.page-ink.v1";

const USAGE: &str = "Usage: atrament-profiles notebook [--help] \
    | paper [atrament-paper options] \
    | glyphs [--profile strokes.tsv|-] \
    [--glyph-stroke GRAPHEME X1 Y1 X2 Y2 WIDTH_UM '#RRGGBB'] \
    [--copy-glyph SOURCE TARGET] \
    [--copy-path SOURCE PATH_INDEX TARGET] \
    [--copy-path-shifted SOURCE PATH_INDEX TARGET DX_UM DY_UM] \
    [--remove-glyph GRAPHEME] \
    [--reverse-stroke GRAPHEME INDEX] \
    [--split-stroke GRAPHEME INDEX] \
    [--split-stroke-exact GRAPHEME INDEX] \
    [--merge-strokes GRAPHEME FIRST_INDEX] \
    [--move-stroke GRAPHEME FROM TO] \
    [--reverse-path GRAPHEME PATH_INDEX] \
    [--close-path GRAPHEME PATH_INDEX] \
    [--join-next-path GRAPHEME PATH_INDEX] \
    [--join-next-path-curved GRAPHEME PATH_INDEX BEND_X_UM BEND_Y_UM] \
    [--close-path-curved GRAPHEME PATH_INDEX BEND_X_UM BEND_Y_UM] \
    [--rotate-path GRAPHEME PATH_INDEX START_STROKE] \
    [--move-path GRAPHEME FROM_PATH TO_PATH] \
    [--translate-path GRAPHEME PATH_INDEX DX_UM DY_UM] \
    [--scale-path GRAPHEME PATH_INDEX X_PERCENT Y_PERCENT] \
    [--shear-path GRAPHEME PATH_INDEX DEG BASELINE_Y_UM] \
    [--reflect-path GRAPHEME PATH_INDEX x|y PIVOT_UM] \
    [--rotate-path-ink GRAPHEME PATH_INDEX DEG PIVOT_X_UM PIVOT_Y_UM] \
    [--style-path GRAPHEME PATH_INDEX WIDTH_UM #RRGGBB] \
    [--remove-path GRAPHEME PATH_INDEX] \
    [--shear-glyph GRAPHEME ANGLE_DEG BASELINE_Y_UM] \
    [--scale-glyph-height GRAPHEME PERCENT BASELINE_Y_UM] \
    [--scale-glyph-width GRAPHEME PERCENT ANCHOR_X_UM] \
    [--scale-glyph-ink GRAPHEME PERCENT] \
    [--translate-glyph GRAPHEME DX_UM DY_UM] \
    [--rotate-glyph GRAPHEME ANGLE_DEG PIVOT_X_UM PIVOT_Y_UM] \
    [--reflect-glyph GRAPHEME x|y PIVOT_UM] \
    [--join-strokes GRAPHEME FIRST_INDEX] \
    [--set-stroke GRAPHEME INDEX X1 Y1 X2 Y2 WIDTH_UM '#RRGGBB'] \
    [--remove-stroke GRAPHEME INDEX] \
    [--audit-text UTF8_TEXT|--audit-baseline|--inspect-glyph GRAPHEME \
    |--inspect-paths GRAPHEME] \
    [--output strokes.tsv [--replace]] \
    | handwriting [--profile ink.json] [--seed U64] \
    [--set-global DIM MIN MAX MEAN MEDIAN DEVIATION] \
    [--set-glyph GRAPHEME DIM MIN MAX MEAN MEDIAN DEVIATION] \
    [--remove-glyph GRAPHEME] [--remove-glyph-dimension GRAPHEME DIM] \
    [--inspect | --inventory-text TEXT | --preview-glyph GRAPHEME OCCURRENCE] \
    [--output FILE [--replace]]\n\
    Paper is the existing Rust physical-page profile (not a full notebook).\n\
    Handwriting units: micrometres, millidegrees, or parts per thousand.\n\
    --inspect-glyph prints exact authored stroke order and direction;\n\
    output is stdout-only and never an inferred cursive pen path.\n\
    --inspect-paths groups exactly touching, same-pen directed segments\n\
    in stroke order; it does not persist new curve or pen-lift metadata.\n\
    Glyph stroke indices are one-based within one exact Unicode glyph;\n\
    --reverse-stroke flips pen direction and --move-stroke shifts order.\n\
    --split-stroke adds two continuous half-segments at the rounded midpoint.\n\
    --split-stroke-exact uses an integer point for lossless remerging.\n\
    --merge-strokes coalesces touching co-directed same-pen straight ink.\n\
    --set-stroke updates ink geometry; --remove-stroke deletes one segment.\n\
    --reverse-path flips a multi-stroke path; --move-path swaps adjacent\n\
    paths using the current pen-continuity inspection order.\n\
    --rotate-path changes the first stroke of a closed loop (one-based).\n\
    --close-path inserts a short closing segment on a selected open path.\n\
    --join-next-path bridges adjacent same-pen glyph paths (up to 1200 um).\n\
    --join-next-path-curved adds four quadratic segments (bend +/-600 um).\n\
    --close-path-curved closes an open path with four curved segments.\n\
    --translate-path moves one path using bounded signed micrometres.\n\
    --scale-path resizes around the selected path's first endpoint.\n\
    --shear-path slants one selected pen path -20..20 degrees.\n\
    --reflect-path mirrors a selected path, preserving directed joints.\n\
    --rotate-path-ink turns one path clockwise around X/Y coordinates.\n\
    --shear-glyph slants all exact-glyph pen segments 0..20 degrees.\n\
    --scale-glyph-height rescales exact-glyph Y about the baseline.\n\
    --scale-glyph-width rescales exact-glyph X about the anchor.\n\
    --scale-glyph-ink rescales exact-glyph pen thickness 50..150%.\n\
    --translate-glyph moves all exact-glyph strokes within the pen cell.\n\
    --rotate-glyph rotates ordered ink clockwise around an X/Y pivot.\n\
    --reflect-glyph mirrors exact glyph ink about an X or Y coordinate.\n\
    --join-strokes inserts a short same-pen connector to the next segment.\n\
    --copy-glyph clones explicit source ink under an unoccupied target.\n\
    --copy-path appends one source pen path to a distinct exact target.\n\
    --copy-path-shifted appends translated ink, including self-copy.\n\
    --inventory-text checks exact text-baseline membership and overrides,\n\
    not glyph outlines, available ink shapes, or renderability.\n\
    --preview-glyph reports bounded seeded numerical estimates only; \
    not calibrated moments, median reproduction, or generated ink.\n\
    Run paper --help for page sizes, styles, margins and PDF options.";

const NOTEBOOK_USAGE: &str = "Usage: atrament-profiles notebook \
    [--profile notebook.json] [--paper-profile paper.json] \
    [--date-style none|line|box] \
    [--date-placement none|top-left|top-right] \
    [--date-text YYYY-MM-DD|NN/NN/YYYY|NN.NN.YYYY] \
    [--border-radius-um INTEGER] [--page-numbers true|false] \
    [--printed-sheet true|false] [--inspect | --inspect-flow] \
    [--inspect-flow-inventory (requires --inspect-flow)] \
    [--inspect-flow-page-inventory (requires --inspect-flow-inventory)] \
    [--inspect-flow-shape-coverage (requires --inspect-flow)] \
    [--inspect-flow-stroke-demand (requires --inspect-flow-shape-coverage)] \
    [--inspect-flow-stroke-breakdown (requires --inspect-flow-stroke-demand)] \
    [--inspect-flow-ink-validation (requires --inspect-flow-shape-coverage)] \
    [--inspect-flow-ink-bounds (requires --inspect-flow-ink-validation)] \
    [--inspect-flow-ink-palette (requires --inspect-flow-ink-validation)] \
    [--inspect-flow-ink-widths (requires --inspect-flow-ink-validation)] \
    [--inspect-flow-ink-length (requires --inspect-flow-ink-validation)] \
    [--inspect-flow-ink-length-by-color (requires --inspect-flow-ink-length)] \
    [--inspect-flow-ink-gaps (requires --inspect-flow-ink-validation)] \
    [--inspect-flow-ink-gap-details (requires --inspect-flow-ink-gaps)] \
    [--export-paper pdf|png] [--pages 1..32 (PDF only)] \
    [--ruler-seed UINT32 --ruler-error-mm 0.001..0.250] \
    [--dpi 72..300 (PNG only)] [--png-page 1..32 (PNG only)] \
    [--quality preview|final (PNG only, exclusive with --dpi)] \
    [--ink-line PAGE X1_UM Y1_UM X2_UM Y2_UM WIDTH_UM '#RRGGBB'] \
    [--ink-file FILE|-] [--emit-ink-file] \
    [--glyph-file strokes.tsv|-] \
    [--glyph-stroke GRAPHEME X1 Y1 X2 Y2 WIDTH_UM '#RRGGBB'] \
    [--fallback-glyph MISSING AUTHORED] \
    [--place-text PAGE X_UM Y_UM UTF8_TEXT] \
    [--join-text-pair LEFT RIGHT (repeatable, max 32)] \
    [--join-text-anchors LEFT RIGHT EXIT_STROKE ENTRY_STROKE] \
    [--glyph-variation SEED WIDTH HEIGHT THICKNESS SLANT SPACING REGULARITY] \
    [--glyph-line-drift MAX_UM (requires --glyph-variation)] \
    [--glyph-word-drift MAX_UM (requires --glyph-variation)] \
    [--glyph-word-rise MAX_UM (requires --glyph-variation)] \
    [--glyph-word-pressure MAX_PERCENT (requires --glyph-variation)] \
    [--glyph-word-pigment MAX_PERCENT (requires --glyph-variation)] \
    [--glyph-word-slant MAX_MDEG (requires --glyph-variation)] \
    [--glyph-word-width MAX_PERCENT (requires --glyph-variation)] \
    [--glyph-word-height MAX_PERCENT (requires --glyph-variation)] \
    [--glyph-word-tracking MAX_UM (requires --glyph-variation)] \
    [--glyph-word-baseline-step MAX_UM (requires --glyph-variation)] \
    [--heading-style underline|double-underline|marker|none] \
    [--heading-align left|center|right (Markdown only)] \
    [--heading-color '#RRGGBB'] [--heading-marker-scale 50..105] \
    [--heading-ink-color '#RRGGBB'] [--heading-ink-scale 100..250] \
    [--flow-text TEXT | --flow-markdown MARKDOWN | \
     --flow-text-file FILE|- | --flow-markdown-file FILE|-] \
    [--flow-region LEFT_UM TOP_UM WIDTH_UM HEIGHT_UM] \
    [--flow-pitch-um MIN_UM (9000..30000)] \
    [--flow-paragraph-gap-um GAP_UM (0..12000, Markdown only)] \
    [--flow-body-inset-um INSET_UM (0..12000, Markdown only)] \
    [--flow-body-align left|center|right (Markdown only)] \
    [--flow-quote-align left|center|right (Markdown only)] \
    [--flow-quote-color '#RRGGBB' (Markdown only)] \
    [--flow-bullet-align left|center|right (Markdown only)] \
    [--flow-bullet-style dash|dot|ring (Markdown only)] \
    [--flow-bullet-color '#RRGGBB' (Markdown only)] \
    [--flow-task-style square|diamond (Markdown only)] \
    [--flow-task-color '#RRGGBB' (Markdown only)] \
    [--flow-ordered-align left|center|right (Markdown only)] \
    [--flow-task-align left|center|right (Markdown only)] \
    [--flow-body-first-line-indent-um INSET_UM (0..12000, Markdown only)] \
    [--flow-body-continuation-indent-um INSET_UM (0..12000, Markdown only)] \
    [--flow-body-right-inset-um INSET_UM (0..12000, Markdown only)] \
    [--flow-list-quote-inset-um INSET_UM (0..12000, Markdown only)] \
    [--flow-list-quote-right-inset-um INSET_UM (0..12000, Markdown only)] \
    [--handwriting-profile ink.json (requires glyph shapes)] \
    [--output FILE [--replace]]\n\
    Flow requires --export-paper pdf|png or --emit-ink-file; text is not \
    stored in notebook profiles. \
    Paper-layer export uses nested Rust paper settings and skips ruling \
    on preprinted sheets. The optional date prints in its selected box \
    or line; rounded borders render. Repeated --ink-line (max 4096) draws \
    explicit physical pen segments; --ink-file reads a bounded saved \
    atrament.page-ink.v1 TSV (page, endpoints, width, color); \
    select - for bounded stdin. \
    --emit-ink-file compiles physical segments (including placed glyphs \
    and flow ornaments) into reusable versioned TSV instead of a PDF/PNG. \
    It retains no paper marks, date or saved semantic content. \
    These inputs remain export-only, not saved notebook v1 fields. \
    --glyph-stroke supplies exact \
    author-drawn segments in a 4 x 5 mm cell and --place-text repeats them; \
    missing shapes refuse; --glyph-file loads a reusable library; \
    select - for bounded glyph TSV stdin (one stdin source per command). \
    --join-text-pair opt-in bridges selected adjacent glyph pen endpoints \
    within one row; matching physical ink and <=1200 um gap required. \
    --join-text-anchors chooses explicit 1-based source exit and target \
    entry strokes for an adjacent pair, without modifying glyph ink. \
    --fallback-glyph borrows a caller-selected existing authored shape \
    for one missing exact grapheme; no implicit or chained mappings. \
    --flow-text keeps explicit lines; --flow-markdown joins plain lines \
    into paragraphs. Two trailing spaces or one trailing backslash force \
    a hard row break. A standalone <!-- atrament:page-break --> starts \
    the next nonempty Markdown block on a new physical page; adjacent \
    directives collapse and terminal directives add no blank sheet. \
    Markdown supports # through ###### headings, \
    -/*/+ bullets, \
    [ ]/[x]/[X] checkbox tasks after bullets, > quotes, 1./1) numbered \
    lists (authored digits/punctuation), and standalone ---/***/___ \
    horizontal ink rules. -file reads bounded UTF-8 from a regular file; \
    select - to read at most 4096 UTF-8 bytes from standard input. \
    An initial UTF-8 BOM is ignored but counts toward the 4096-byte cap. \
    Both use physical printable regions and exact authored ink; \
    optional --flow-region repeats a fixed rectangle within margins on \
    every sheet, refusing invalid regions or unfit glyphs/rows. \
    --flow-pitch-um selects minimum 9..30 mm physical row spacing; \
    taller measured glyphs retain their necessary clearance. \
    --flow-paragraph-gap-um selects Markdown blank-line spacing \
    from 0..12 mm; default is 4 mm. Plain rows are unchanged. \
    --flow-body-inset-um shifts Markdown prose right 0..12 mm; \
    --flow-body-first-line-indent-um indents only the first physical \
    row of each Markdown prose block by 0..12 mm. \
    --flow-body-continuation-indent-um indents later wrapped \
    rows of each prose block by 0..12 mm. \
    --flow-body-right-inset-um reserves 0..12 mm to its right. \
    Titles and literal code stay unchanged by those prose insets; \
    --flow-body-align centers or right-aligns each measured prose row \
    within the remaining inset writing width; defaults to left. \
    --flow-quote-align centers or right-aligns quoted text only, \
    preserving each vertical quote rule in its original gutter. \
    --flow-quote-color changes only the quote rule pen ink. \
    --flow-bullet-align aligns unordered bullet and checkbox text \
    while their symbols stay fixed. --flow-bullet-style selects \
    dash, dot or outlined ring for unordered bullets only. \
    --flow-bullet-color changes only synthetic bullet ink. \
    --flow-ordered-align aligns \
    authored numbered list labels with their item text, preserving \
    the measured hanging indent on continued rows. \
    --flow-task-align overrides bullet alignment for checkbox text; \
    checkbox boxes remain fixed at their original gutter. \
    --flow-task-style selects square or diamond checkbox outlines, \
    retaining the authored task body and original marker gutter. \
    --flow-task-color changes only checkbox and check ink. \
    --flow-list-quote-inset-um shifts list/quote ink 0..12 mm right; \
    --flow-list-quote-right-inset-um reserves 0..12 mm on their right. \
    --inspect-flow prints the nominal measured row positions and page \
    count without requiring glyph strokes or writing any output file. \
    --inspect-flow-inventory lists exact visible graphemes after Markdown \
    parsing, excluding synthetic ornaments and ASCII spaces. \
    --inspect-flow-page-inventory splits those requirements per sheet. \
    --inspect-flow-shape-coverage checks exact authored glyph shapes \
    and explicit --fallback-glyph mappings without rendering ink. \
    --inspect-flow-stroke-demand counts known nominal pen segments and \
    unresolved glyph uses against the native 4096-segment export budget. \
    --inspect-flow-stroke-breakdown attributes this demand to each exact \
    or borrowed shape, globally and optionally per page. \
    --inspect-flow-ink-validation checks expanded native ink geometry \
    with the PDF validator without writing a file. \
    --inspect-flow-ink-bounds reports conservative pen-inclusive \
    physical rectangles per page after validation, including blank pages. \
    --inspect-flow-ink-palette lists actual RGB ink colors and physical \
    segment counts per page, excluding paper marks. \
    --inspect-flow-ink-widths lists actual physical pen thicknesses in \
    micrometres and segment counts per page, excluding paper marks. \
    --inspect-flow-ink-length sums rounded physical pen-down centerline \
    lengths in micrometres per page, excluding pen-up moves and paper. \
    --inspect-flow-ink-length-by-color groups those physical lengths by \
    selected pen RGB color for the whole notebook and each sheet. \
    --inspect-flow-ink-gaps sums consecutive emitted-segment endpoint \
    distances on each sheet, excluding cross-page and device moves. \
    --inspect-flow-ink-gap-details separates coincident and displaced \
    endpoints, RGB changes and the largest per-sheet separation. \
    Shape coverage is opt-in and does not validate physical ink bounds. \
    Explicit --place-text overlays may coexist with flow, outside its \
    rectangle, but do not reserve flow space or prevent collisions. \
    PDF page count includes the last manual ink page, with or without \
    flow; explicit --pages remains a strict upper bound. \
    Markdown headings default to left-aligned dark underlines; choose \
    --heading-align center|right to shift titles and their ornaments \
    by measured ink contour within the selected physical flow region. \
    Select a user-colored single/double underline, opaque pastel \
    marker band or no ornament. \
    --heading-marker-scale adjusts the marker underlay pen width (percent) \
    only with --heading-style marker; the default is 100. \
    --heading-ink-color recolors heading glyph ink, and \
    --heading-ink-scale enlarges only heading pen widths (percent); \
    the authored body strokes and glyph templates remain unchanged. \
    --handwriting-profile applies selected mean dimensions to authored \
    shapes, not statistical sampling. --glyph-variation explicitly \
    applies reproducible per-occurrence creative jitter to the same shapes; \
    the six spreads use percent, percent, percent, degrees, um, um. \
    --glyph-line-drift adds optional seeded smooth run-local vertical \
    motion (0..150 um), keeping authored shapes and row pitch fixed. \
    --glyph-word-drift adds a seeded run-local horizontal offset per word \
    (0..150 um) without changing nominal advances or pen contours. \
    --glyph-word-rise adds an independent seeded vertical offset per word \
    (0..150 um) without changing row pitch or pen contours. \
    --glyph-word-pressure varies shared physical pen width per word \
    (0..20 percent), with invalid ink refused before commit. \
    --glyph-word-pigment varies shared ink lightness by 0..20 percent. \
    --glyph-word-slant adds a shared baseline shear per word (0..5000 \
    millidegrees), with invalid physical pen geometry refused. \
    --glyph-word-width scales authored letter contours together per word \
    (0..15 percent) without changing measured text advances. \
    --glyph-word-height scales each word's letters about their lower \
    baseline (0..15 percent), preserving nominal row layout. \
    --glyph-word-tracking adds a shared seeded advance between letters \
    of the same word (0..150 um) without changing nominal text flow. \
    --glyph-word-baseline-step adds a signed vertical offset per letter \
    step within a word (0..150 um), resetting at every ASCII space. \
    Unprintable physical ink refuses instead of clipping. \
    No synthesized glyphs, inline Markdown styles or images are rendered.";

#[derive(Clone, Debug)]
struct GlyphStroke {
    end_um: (u32, u32),
    grapheme: String,
    rgb: [u8; 3],
    start_um: (u32, u32),
    width_um: u32,
}

#[derive(Clone, Debug)]
struct GlyphFallback {
    authored: String,
    missing: String,
}

// Coverage is exact-key, with an optional explicit one-hop substitution.
// It never promises that the borrowed contours are visually equivalent.
enum ShapeCoverage<'shape> {
    Authored,
    Borrowed(&'shape str),
    Missing,
}

impl ShapeCoverage<'_> {
    fn is_present(&self) -> bool {
        !matches!(self, Self::Missing)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct HeadingInk {
    color: Option<[u8; 3]>,
    scale_percent: u32,
}

#[derive(Clone, Debug)]
struct TextRun {
    heading_ink: Option<HeadingInk>,
    origin_um: (u32, u32),
    page: usize,
    text: String,
}

#[derive(Debug)]
struct TextJoin {
    // Zero-based authored-stroke ordinals within each exact glyph.
    // None preserves the historical last-to-first join behavior.
    anchors: Option<(usize, usize)>,
    left: String,
    right: String,
}

#[derive(Debug, Default)]
struct GlyphOverlay {
    fallbacks: Vec<GlyphFallback>,
    line_drift_um: Option<u32>,
    loaded_file: bool,
    runs: Vec<TextRun>,
    strokes: Vec<GlyphStroke>,
    text_joins: Vec<TextJoin>,
    variation: Option<GlyphVariation>,
    word_baseline_step_um: Option<u32>,
    word_drift_um: Option<u32>,
    word_height_percent: Option<u32>,
    word_pigment_percent: Option<u32>,
    word_pressure_percent: Option<u32>,
    word_rise_um: Option<u32>,
    word_slant_millidegrees: Option<u32>,
    word_tracking_um: Option<u32>,
    word_width_percent: Option<u32>,
}

#[derive(Debug)]
struct HandwritingRequest {
    inspect: bool,
    inventory_text: Option<String>,
    output: Option<String>,
    preview_glyph: Option<(String, u64)>,
    profile: HandwritingStatisticsProfile,
    replace: bool,
}

#[derive(Debug)]
struct NotebookRequest {
    dpi: u32,
    export_format: Option<NotebookExportFormat>,
    flow: Option<FlowSource>,
    flow_region: Option<FlowRegion>,
    flow_typography: FlowTypography,
    glyphs: GlyphOverlay,
    heading: HeadingDecoration,
    ink: Vec<NotebookInkLine>,
    inspect: bool,
    inspect_flow: bool,
    inspect_flow_inventory: bool,
    inspect_flow_page_inventory: bool,
    inspect_flow_shape_coverage: bool,
    inspect_flow_stroke_demand: bool,
    inspect_flow_stroke_breakdown: bool,
    inspect_flow_ink_validation: bool,
    inspect_flow_ink_bounds: bool,
    inspect_flow_ink_palette: bool,
    inspect_flow_ink_widths: bool,
    inspect_flow_ink_length: bool,
    inspect_flow_ink_length_by_color: bool,
    inspect_flow_ink_gaps: bool,
    inspect_flow_ink_gap_details: bool,
    output: Option<String>,
    pages: usize,
    pages_explicit: bool,
    png_page: usize,
    profile: NotebookProfile,
    replace: bool,
    ruler_variation: Option<RulerVariation>,
    statistics: Option<HandwritingStatisticsProfile>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NotebookExportFormat {
    InkTsv,
    Pdf,
    Png,
}

#[derive(Default)]
struct NotebookExportOptions {
    dpi: Option<u32>,
    format: Option<NotebookExportFormat>,
    pages: Option<usize>,
    png_page: Option<usize>,
    quality_dpi: Option<u32>,
    ruler_seed: Option<String>,
    ruler_error_mm: Option<String>,
}

#[derive(Default)]
struct NotebookFlagPreflight {
    explicit_pages: Option<usize>,
    export_format: Option<NotebookExportFormat>,
    heading: HeadingDecoration,
    imported_ink_count: usize,
    imported_overlay_selected: bool,
    selected: BTreeSet<String>,
    ruler_seed: Option<String>,
    ruler_error_mm: Option<String>,
}

// Pair export-only ruler options before any selected file or stdin is read.
fn resolve_notebook_ruler(
    seed: Option<&str>, error: Option<&str>,
) -> Result<Option<RulerVariation>, String> {
    match (seed, error) {
        (None, None) => Ok(None),
        (Some(seed), Some(error)) => {
            RulerVariation::from_decimal_mm(seed, error).map(Some)
        }
        _ => Err(concat!(
            "--ruler-seed and --ruler-error-mm must be supplied together",
        ).into()),
    }
}

// Output and heading selections do not depend on source bytes. Consume their
// arity before opening an earlier file or waiting for stdin to close.
impl NotebookFlagPreflight {
    fn select_body_row_indent(
        &mut self, option: &str, iter: &mut Iter<'_, String>,
    ) -> Result<usize, String> {
        self.unique(option, &format!("{option} may appear only once"))?;
        if option == "--flow-body-first-line-indent-um" {
            let _inset = parse_notebook_flow_first_line_indent(iter)?;
        } else {
            let _inset = parse_notebook_flow_continuation_indent(iter)?;
        }
        Ok(0)
    }

    fn select_flow_align(
        &mut self, option: &str, iter: &mut Iter<'_, String>,
    ) -> Result<usize, String> {
        self.unique(option, &format!("{option} may appear only once"))?;
        let _alignment = parse_notebook_flow_align(iter, option)?;
        Ok(0)
    }

    fn select_flow_body_inset(
        &mut self, option: &str, iter: &mut Iter<'_, String>,
    ) -> Result<usize, String> {
        self.unique(option, &format!("{option} may appear only once"))?;
        let _inset = parse_notebook_flow_body_inset(
            iter, option == "--flow-body-right-inset-um",
        )?;
        Ok(0)
    }

    fn select_flow_bullet_style(
        &mut self, iter: &mut Iter<'_, String>,
    ) -> Result<usize, String> {
        self.unique("--flow-bullet-style",
            "--flow-bullet-style may appear only once")?;
        let _style = parse_notebook_flow_bullet_style(iter)?;
        Ok(0)
    }

    fn select_flow_pen_color(
        &mut self, option: &str, iter: &mut Iter<'_, String>,
    ) -> Result<usize, String> {
        self.unique(option, &format!("{option} may appear only once"))?;
        let _color = parse_notebook_flow_marker_color(iter, option)?;
        Ok(0)
    }

    fn select_flow_pitch(
        &mut self, iter: &mut Iter<'_, String>,
    ) -> Result<usize, String> {
        self.unique("--flow-pitch-um",
            "--flow-pitch-um may appear only once")?;
        let _pitch = parse_notebook_flow_pitch(iter)?;
        Ok(0)
    }

    fn select_flow_task_style(
        &mut self, iter: &mut Iter<'_, String>,
    ) -> Result<usize, String> {
        self.unique("--flow-task-style",
            "--flow-task-style may appear only once")?;
        let _style = parse_notebook_flow_task_style(iter)?;
        Ok(0)
    }

    fn select_heading(&mut self, option: &str) -> Result<(), String> {
        let diagnostic = match option {
            "--heading-ink-scale" => "--heading-ink-scale repeated".to_owned(),
            "--heading-marker-scale" =>
                "--heading-marker-scale repeated".to_owned(),
            _ => format!("{option} may appear only once"),
        };
        self.unique(option, &diagnostic)
    }

    fn select_heading_setting(
        &mut self, option: &str, iter: &mut Iter<'_, String>,
    ) -> Result<(), String> {
        self.select_heading(option)?;
        self.heading.parse(option, iter)
    }

    fn select_list_quote_inset(
        &mut self, option: &str, iter: &mut Iter<'_, String>,
    ) -> Result<usize, String> {
        self.unique(option, &format!("{option} may appear only once"))?;
        let _inset = parse_notebook_flow_list_quote_inset(
            iter, option == "--flow-list-quote-right-inset-um",
        )?;
        Ok(0)
    }

    fn select_option(
        &mut self, option: &str, iter: &mut Iter<'_, String>,
    ) -> Result<Option<usize>, String> {
        self.imported_ink_count = self.imported_ink_count
            .saturating_add(usize::from(option == "--ink-file"));
        let operands = match option {
            "--inspect" | "--inspect-flow" | "--inspect-flow-inventory"
            | "--inspect-flow-page-inventory"
            | "--inspect-flow-shape-coverage"
            | "--inspect-flow-stroke-demand"
            | "--inspect-flow-stroke-breakdown"
            | "--inspect-flow-ink-validation"
            | "--inspect-flow-ink-bounds"
            | "--inspect-flow-ink-palette"
            | "--inspect-flow-ink-widths"
            | "--inspect-flow-ink-length"
            | "--inspect-flow-ink-length-by-color"
            | "--inspect-flow-ink-gaps"
            | "--inspect-flow-ink-gap-details" => {
                self.unique(option, &format!("{option} may appear only once"))?;
                0
            }
            "--replace" => {
                self.unique(option, "--replace may appear only once")?;
                0
            }
            "--emit-ink-file" => {
                if self.export_format.is_some() {
                    return Err(concat!(
                        "--emit-ink-file may appear only once and ",
                        "cannot accompany --export-paper",
                    ).into());
                }
                self.export_format = Some(NotebookExportFormat::InkTsv);
                0
            }
            "--export-paper" => {
                if self.export_format.is_some() {
                    return Err(concat!(
                        "--export-paper may appear only once and ",
                        "cannot accompany --emit-ink-file",
                    ).into());
                }
                let selected = iter.next().map(String::as_str);
                self.export_format = Some(match selected {
                    Some("pdf") => NotebookExportFormat::Pdf,
                    Some("png") => NotebookExportFormat::Png,
                    _ => {
                        return Err("--export-paper requires pdf or png".into());
                    }
                });
                0
            }
            "--output" => {
                self.unique(option, "--output requires one explicit filename")?;
                let _selected = parse_selected_output(iter, false)?;
                0
            }
            "--pages" | "--dpi" | "--png-page" => {
                self.unique(option, &format!("{option} may appear only once"))?;
                let (minimum, maximum) = if option == "--dpi" {
                    (72, 300)
                } else {
                    (1, 32)
                };
                let value = export_integer(
                    iter.next().map(String::as_str), option,
                    minimum, maximum,
                )?;
                if option == "--pages" {
                    self.explicit_pages = Some(usize::try_from(value)
                        .map_err(|error| {
                            format!("page count refused: {error}")
                        })?);
                }
                0
            }
            "--ruler-seed" | "--ruler-error-mm" => {
                self.unique(option, &format!("{option} may appear only once"))?;
                let value = iter.next().ok_or_else(|| {
                    format!("{option} requires a decimal value")
                })?;
                if option == "--ruler-seed" {
                    self.ruler_seed = Some(value.to_owned());
                } else {
                    self.ruler_error_mm = Some(value.to_owned());
                }
                0
            }
            "--quality" => {
                self.unique(option, "--quality may appear only once")?;
                let _validated =
                    parse_export_quality(iter.next().map(String::as_str))?;
                0
            }
            "--flow-paragraph-gap-um" => {
                self.unique(option, &format!("{option} may appear only once"))?;
                let _gap = parse_notebook_flow_paragraph_gap(iter)?;
                0
            }
            "--flow-body-align" | "--flow-quote-align"
            | "--flow-bullet-align" | "--flow-ordered-align"
            | "--flow-task-align" =>
                self.select_flow_align(option, iter)?,
            "--flow-bullet-style" => self.select_flow_bullet_style(iter)?,
            "--flow-task-style" => self.select_flow_task_style(iter)?,
            "--flow-bullet-color" | "--flow-task-color"
            | "--flow-quote-color" =>
                self.select_flow_pen_color(option, iter)?,
            "--flow-body-first-line-indent-um"
            | "--flow-body-continuation-indent-um" =>
                self.select_body_row_indent(option, iter)?,
            "--flow-body-inset-um" | "--flow-body-right-inset-um" =>
                self.select_flow_body_inset(option, iter)?,
            "--flow-list-quote-inset-um"
            | "--flow-list-quote-right-inset-um" =>
                self.select_list_quote_inset(option, iter)?,
            "--flow-pitch-um" => self.select_flow_pitch(iter)?,
            "--heading-style" | "--heading-align" | "--heading-color"
            | "--heading-marker-scale" | "--heading-ink-color"
            | "--heading-ink-scale" => {
                self.select_heading_setting(option, iter)?;
                0
            }
            _ => return Ok(None),
        };
        Ok(Some(operands))
    }

    fn unique(
        &mut self, option: &str, message: &str,
    ) -> Result<(), String> {
        if !self.selected.insert(option.to_owned()) {
            return Err(message.to_owned());
        }
        Ok(())
    }

    fn validate(
        self, flow_format: Option<FlowFormat>, region_selected: bool,
        inline_ink_count: usize, inline_last_page: usize,
        glyph_has_any: bool, glyph_non_shape_selected: bool,
    ) -> Result<(), String> {
        let has = |option: &str| self.selected.contains(option);
        let inspect_flow = has("--inspect-flow");
        if has("--inspect-flow-inventory") && !inspect_flow {
            return Err(
                "--inspect-flow-inventory requires --inspect-flow".into()
            );
        }
        if has("--inspect-flow-ink-gap-details")
            && !has("--inspect-flow-ink-gaps") {
            return Err(concat!(
                "--inspect-flow-ink-gap-details requires ",
                "--inspect-flow-ink-gaps",
            ).into());
        }
        if has("--inspect-flow-ink-gaps")
            && !has("--inspect-flow-ink-validation") {
            return Err(concat!(
                "--inspect-flow-ink-gaps requires ",
                "--inspect-flow-ink-validation",
            ).into());
        }
        if has("--inspect-flow-ink-length-by-color")
            && !has("--inspect-flow-ink-length") {
            return Err(concat!(
                "--inspect-flow-ink-length-by-color requires ",
                "--inspect-flow-ink-length",
            ).into());
        }
        if has("--inspect-flow-ink-length")
            && !has("--inspect-flow-ink-validation") {
            return Err(concat!(
                "--inspect-flow-ink-length requires ",
                "--inspect-flow-ink-validation",
            ).into());
        }
        if has("--inspect-flow-ink-widths")
            && !has("--inspect-flow-ink-validation") {
            return Err(concat!(
                "--inspect-flow-ink-widths requires ",
                "--inspect-flow-ink-validation",
            ).into());
        }
        if has("--inspect-flow-ink-palette")
            && !has("--inspect-flow-ink-validation") {
            return Err(concat!(
                "--inspect-flow-ink-palette requires ",
                "--inspect-flow-ink-validation",
            ).into());
        }
        if has("--inspect-flow-ink-bounds")
            && !has("--inspect-flow-ink-validation") {
            return Err(concat!(
                "--inspect-flow-ink-bounds requires ",
                "--inspect-flow-ink-validation",
            ).into());
        }
        if has("--inspect-flow-ink-validation")
            && !has("--inspect-flow-shape-coverage") {
            return Err(concat!(
                "--inspect-flow-ink-validation requires ",
                "--inspect-flow-shape-coverage",
            ).into());
        }
        if has("--inspect-flow-stroke-breakdown")
            && !has("--inspect-flow-stroke-demand") {
            return Err(concat!(
                "--inspect-flow-stroke-breakdown requires ",
                "--inspect-flow-stroke-demand",
            ).into());
        }
        if has("--inspect-flow-stroke-demand")
            && !has("--inspect-flow-shape-coverage") {
            return Err(concat!(
                "--inspect-flow-stroke-demand requires ",
                "--inspect-flow-shape-coverage",
            ).into());
        }
        if has("--inspect-flow-shape-coverage") && !inspect_flow {
            return Err(
                "--inspect-flow-shape-coverage requires --inspect-flow".into()
            );
        }
        if has("--inspect-flow-page-inventory")
            && !has("--inspect-flow-inventory") {
            return Err(concat!(
                "--inspect-flow-page-inventory requires ",
                "--inspect-flow-inventory",
            ).into());
        }
        if inspect_flow && flow_format.is_none() {
            return Err("--inspect-flow requires a text or Markdown flow"
                .into());
        }
        if inspect_flow && (has("--inspect") || has("--output")
            || self.export_format.is_some()) {
            return Err(concat!(
                "--inspect-flow is stdout only and cannot accompany ",
                "--inspect, --output or export",
            ).into());
        }
        if inspect_flow && (inline_ink_count > 0
            || self.imported_ink_count > 0
            || (has("--inspect-flow-shape-coverage")
                && glyph_non_shape_selected)
            || (!has("--inspect-flow-shape-coverage")
                && (glyph_has_any || self.imported_overlay_selected))) {
            return Err("--inspect-flow cannot include authored ink or glyphs"
                .into());
        }
        if has("--replace") && !has("--output") {
            return Err("--replace requires --output".into());
        }
        if has("--inspect") && has("--output") {
            return Err("--inspect is stdout only; omit --output".into());
        }
        if has("--inspect") && self.export_format.is_some() {
            return Err("--inspect cannot export paper or page ink".into());
        }
        if has("--pages")
            && self.export_format != Some(NotebookExportFormat::Pdf)
        {
            return Err("--pages requires --export-paper pdf".into());
        }
        if (has("--dpi") || has("--png-page") || has("--quality"))
            && self.export_format != Some(NotebookExportFormat::Png)
        {
            return Err(
                "--dpi, --quality and --png-page require --export-paper png"
                    .into()
            );
        }
        if has("--dpi") && has("--quality") {
            return Err("--quality and --dpi are mutually exclusive".into());
        }
        let ruler = resolve_notebook_ruler(
            self.ruler_seed.as_deref(), self.ruler_error_mm.as_deref(),
        )?;
        if ruler.is_some() && !matches!(
            self.export_format,
            Some(NotebookExportFormat::Pdf | NotebookExportFormat::Png)
        ) {
            return Err("ruler variation requires --export-paper pdf or png"
                .into());
        }
        validate_explicit_manual_page_budget(
            self.explicit_pages, inline_last_page, flow_format.is_some(),
        )?;
        // These relationships depend on selected flags, never on the bytes
        // supplied by a previous flow, glyph library or page-ink source.
        validate_notebook_flow_flag_compatibility(
            flow_format, region_selected,
            FlowTypography {
                body_align: has("--flow-body-align")
                    .then_some(BodyAlign::Left),
                quote_align: has("--flow-quote-align")
                    .then_some(BodyAlign::Left),
                quote_color: has("--flow-quote-color")
                    .then_some([30, 48, 74]),
                bullet_align: has("--flow-bullet-align")
                    .then_some(BodyAlign::Left),
                bullet_style: has("--flow-bullet-style")
                    .then_some(BulletStyle::Dash),
                bullet_color: has("--flow-bullet-color")
                    .then_some([30, 48, 74]),
                ordered_align: has("--flow-ordered-align")
                    .then_some(BodyAlign::Left),
                task_align: has("--flow-task-align")
                    .then_some(BodyAlign::Left),
                task_style: has("--flow-task-style")
                    .then_some(TaskStyle::Square),
                task_color: has("--flow-task-color")
                    .then_some([30, 48, 74]),
                body_first_line_indent:
                    has("--flow-body-first-line-indent-um").then_some(0),
                body_continuation_indent:
                    has("--flow-body-continuation-indent-um").then_some(0),
                pitch: has("--flow-pitch-um").then_some(9_000),
                paragraph_gap: has("--flow-paragraph-gap-um")
                    .then_some(0),
                body_inset: has("--flow-body-inset-um").then_some(0),
                body_right_inset: has("--flow-body-right-inset-um")
                    .then_some(0),
                list_quote_inset: has("--flow-list-quote-inset-um")
                    .then_some(0),
                list_quote_right_inset: has("--flow-list-quote-right-inset-um")
                    .then_some(0),
            }, self.heading, self.export_format, inspect_flow,
        )?;
        // Explicit ink and glyph intent needs an output mode even before
        // selected library or stdin bytes are available.
        if self.export_format.is_none() && !inspect_flow
            && (inline_ink_count != 0 || glyph_has_any
                || self.imported_overlay_selected)
        {
            return Err(concat!(
                "ink and glyph overlays need ",
                "--export-paper pdf or png or --emit-ink-file",
            ).into());
        }
        // An admitted imported ink TSV always supplies at least one line.
        // Refuse an impossible total before reading the first source.
        if inline_ink_count.saturating_add(self.imported_ink_count)
            > MAX_NOTEBOOK_INK_LINES
        {
            return Err("ink exceeds 4096 explicit line segments".into());
        }
        if self.export_format == Some(NotebookExportFormat::InkTsv)
            && inline_ink_count == 0 && flow_format.is_none()
            && !glyph_has_any && !self.imported_overlay_selected
        {
            return Err(
                "page ink export needs at least one physical segment".into()
            );
        }
        Ok(())
    }
}

fn read_handwriting_profile(
    path: &str,
) -> Result<HandwritingStatisticsProfile, String> {
    // Bind the selected path to one opened regular-file descriptor. A
    // metadata preflight followed by File::open could follow a symlink or
    // change targets between the check and the actual read.
    let file = open_selected_profile_file(Path::new(path))
        .map_err(|error| format!("handwriting file refused: {error:?}"))?;
    let bytes = read_bounded_profile_file(file, MAXIMUM_STATISTICS_JSON_BYTES)
        .map_err(|error| format!("handwriting file refused: {error:?}"))?;
    decode_statistics_json(bytes.bytes(), &UnicodeGraphemeSegmentation)
        .map_err(|error| format!("handwriting profile refused: {error:?}"))
}

// The same unsigned decimal seed grammar applies before opening a selected
// profile and when applying the final edit. Rust integers otherwise accept a
// leading '+' that cannot occur in the canonical JSON seed representation.
fn parse_handwriting_seed(value: &str) -> Result<u64, String> {
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("invalid seed: requires unsigned decimal digits".into());
    }
    value.parse::<u64>()
        .map_err(|error| format!("invalid seed: {error}"))
}

fn parse_parameter(values: &[String]) -> Result<DimensionStatistics, String> {
    let [name, min, max, mean, median, deviation] = values else {
        return Err(
            "statistics require DIM MIN MAX MEAN MEDIAN DEVIATION".into()
        );
    };
    let dimension = Dimension::from_key(name)
        .ok_or_else(|| format!("unknown statistics dimension: {name}"))?;
    // Rust's integer parser permits a leading '+', while canonical JSON
    // and the local browser editor use explicit decimal scalar syntax.
    // Validate the original CLI text before reading a selected profile.
    let integer = |value: &str| {
        let digits = value.strip_prefix('-').unwrap_or(value);
        if digits.is_empty()
            || !digits.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err(
                "statistics integer requires signed decimal digits".into()
            );
        }
        value.parse::<i32>()
            .map_err(|error| format!("invalid statistics integer: {error}"))
    };
    if deviation.is_empty()
        || !deviation.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err("deviation requires unsigned decimal digits".into());
    }
    let standard_deviation = deviation
        .parse::<u32>()
        .map_err(|error| format!("deviation must be non-negative: {error}"))?;
    Ok(DimensionStatistics {
        dimension,
        statistics: ScalarStatistics {
            maximum: integer(max)?,
            mean: integer(mean)?,
            median: integer(median)?,
            minimum: integer(min)?,
            standard_deviation,
        },
    })
}

fn remove_glyph_option(
    operation: &str,
    arguments: &mut Iter<'_, String>,
    profile: &HandwritingStatisticsProfile,
) -> Result<HandwritingStatisticsProfile, String> {
    let grapheme = arguments.next().ok_or("removal requires a grapheme")?;
    let dimension = if operation == "--remove-glyph-dimension" {
        let key = arguments.next().ok_or("removal requires a dimension")?;
        Some(
            Dimension::from_key(key)
                .ok_or_else(|| format!("unknown dimension: {key}"))?,
        )
    } else {
        None
    };
    profile
        .without_glyph_override(
            grapheme,
            dimension,
            &UnicodeGraphemeSegmentation,
        )
        .map_err(|error| format!("statistics refused: {error:?}"))
}

// Match FlowSource: only one leading UTF-8 BOM is transport metadata.
// The original source still determines the 4096-byte admission limit.
fn strip_leading_source_bom(source: &str) -> &str {
    source.strip_prefix('\u{feff}').unwrap_or(source)
}

// Only implicit ASCII space and source row boundaries are not ink.
// Distinct Unicode spaces (NBSP, thin space, and others) remain auditable.
const fn is_non_ink_audit_character(ch: char) -> bool {
    ch == ' ' || ch.is_control()
        || matches!(ch, '\u{2028}' | '\u{2029}')
}

fn is_non_ink_audit_grapheme(grapheme: &str) -> bool {
    matches!(grapheme, " " | "\u{2028}" | "\u{2029}")
        || grapheme.chars().all(char::is_control)
}

fn parse_inventory_text(
    arguments: &mut Iter<'_, String>,
    previous: Option<&str>,
) -> Result<String, String> {
    if previous.is_some() {
        return Err("--inventory-text may appear only once".into());
    }
    let text = arguments.next().ok_or("--inventory-text requires text")?;
    let admitted = strip_leading_source_bom(text);
    if text.is_empty() || text.len() > MAX_INVENTORY_TEXT_BYTES
        // Tabs, CRLF/LF and Unicode line/paragraph separators are
        // structure, not visible ink in a coverage report.
        || !admitted.chars().any(|ch| !is_non_ink_audit_character(ch))
        || text.chars().any(|ch| {
            ch.is_control() && !matches!(ch, '\t' | '\r' | '\n')
        })
        || text.replace("\r\n", "").contains('\r')
    {
        return Err(
            concat!(
                "inventory text must be 1..4096 bytes ",
                "without control characters",
            ).into()
        );
    }
    Ok(text.clone())
}

fn parse_preview_glyph(
    arguments: &mut Iter<'_, String>,
    already_selected: bool,
) -> Result<(String, u64), String> {
    if already_selected {
        return Err("--preview-glyph may appear only once".into());
    }
    let glyph = arguments
        .next()
        .ok_or("--preview-glyph requires a grapheme")?;
    validate_statistics_glyph_key(glyph, "numeric preview")?;
    let count = arguments
        .next()
        .ok_or("--preview-glyph requires an occurrence")?;
    if count.is_empty() || !count.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("preview occurrence requires an unsigned integer".into());
    }
    let occurrence = count
        .parse::<u64>()
        .map_err(|error| format!("invalid occurrence: {error}"))?;
    Ok((glyph.clone(), occurrence))
}

fn parse_selected_output(
    arguments: &mut Iter<'_, String>,
    already_selected: bool,
) -> Result<String, String> {
    let path = arguments.next().ok_or("--output requires a filename")?;
    if already_selected || path.starts_with('-') || path.is_empty() {
        return Err("--output requires one explicit filename".into());
    }
    Ok(path.clone())
}
fn validate_handwriting_flags(
    inspect: bool,
    inventory_text: Option<&str>,
    preview_glyph: bool,
    output: Option<&str>,
    replace: bool,
) -> Result<(), String> {
    if preview_glyph
        && (inspect || inventory_text.is_some() || output.is_some())
    {
        return Err(concat!(
            "--preview-glyph is stdout-only and cannot combine with ",
            "--inspect, --inventory-text or --output",
        ).into());
    }
    if replace && output.is_none() {
        return Err("--replace requires --output".into());
    }
    if inspect && output.is_some() {
        return Err("--inspect is stdout only; omit --output".into());
    }
    if inventory_text.is_some() && (inspect || output.is_some()) {
        return Err(
            concat!(
                "--inventory-text is stdout only and cannot combine ",
                "with --inspect or --output",
            ).into()
        );
    }
    Ok(())
}

fn validate_statistics_glyph_key(
    grapheme: &str, operation: &str,
) -> Result<(), String> {
    if !atrament_handwriting_statistics::valid_exact_glyph(
        grapheme, &UnicodeGraphemeSegmentation,
    ) {
        return Err(format!(
            "{operation} requires one exact statistical grapheme",
        ));
    }
    Ok(())
}

// A saved statistics profile must not be opened for a command that cannot
// complete regardless of the source bytes. Consume exact operand counts so
// literal values resembling option names remain ordinary data. This checks
// source-independent syntax and flag compatibility; the full profile and
// merged statistics still validate in the final parser.
fn preflight_handwriting_options(args: &[String]) -> Result<(), String> {
    let (mut iter, mut seen_profile, mut seen_edit) =
        (args.iter(), false, false);
    let (mut inspect, mut inventory_text) = (false, None);
    let (mut preview_glyph, mut output) = (false, None);
    let mut replace = false;
    while let Some(name) = iter.next() {
        match name.as_str() {
            "--profile" => {
                if seen_profile || seen_edit {
                    return Err(
                        "--profile must appear once, before edits".into()
                    );
                }
                let selected = iter.next()
                    .ok_or("--profile requires one input file")?;
                if selected.is_empty() || selected.starts_with('-') {
                    return Err(concat!(
                        "handwriting --profile requires an explicit ",
                        "source path (use ./--name)",
                    ).into());
                }
                seen_profile = true;
            }
            "--seed" => {
                let value = iter.next()
                    .ok_or("--seed requires an unsigned integer")?;
                let _seed = parse_handwriting_seed(value)?;
                seen_edit = true;
            }
            "--set-global" | "--set-glyph" => {
                if name == "--set-glyph" {
                    let grapheme = iter.next()
                        .ok_or("--set-glyph requires a grapheme")?;
                    validate_statistics_glyph_key(grapheme, "glyph edit")?;
                }
                let values = iter.by_ref().take(6).cloned()
                    .collect::<Vec<_>>();
                let parameter = parse_parameter(&values)?;
                parameter.statistics.validate(parameter.dimension)
                    .map_err(|error| format!(
                        "statistics range refused: {error:?}",
                    ))?;
                seen_edit = true;
            }
            "--remove-glyph" | "--remove-glyph-dimension" => {
                let grapheme = iter.next()
                    .ok_or("removal requires a grapheme")?;
                validate_statistics_glyph_key(grapheme, "glyph removal")?;
                if name == "--remove-glyph-dimension" {
                    let key = iter.next()
                        .ok_or("removal requires a dimension")?;
                    let _dimension = Dimension::from_key(key)
                        .ok_or_else(|| format!("unknown dimension: {key}"))?;
                }
                seen_edit = true;
            }
            "--inspect" if inspect =>
                return Err("--inspect may appear only once".into()),
            "--inspect" => inspect = true,
            "--inventory-text" => {
                inventory_text = Some(parse_inventory_text(
                    &mut iter, inventory_text.as_deref(),
                )?);
            }
            "--preview-glyph" => {
                let _selected = parse_preview_glyph(
                    &mut iter, preview_glyph,
                )?;
                preview_glyph = true;
            }
            "--output" => {
                output = Some(parse_selected_output(
                    &mut iter, output.is_some(),
                )?);
            }
            "--replace" if replace =>
                return Err("--replace may appear only once".into()),
            "--replace" => replace = true,
            _ => return Err(format!("unknown handwriting option: {name}")),
        }
    }
    validate_handwriting_flags(
        inspect, inventory_text.as_deref(), preview_glyph,
        output.as_deref(), replace,
    )
}

fn parse_handwriting_options(
    args: &[String],
) -> Result<HandwritingRequest, String> {
    preflight_handwriting_options(args)?;
    let boundaries = UnicodeGraphemeSegmentation;
    let mut profile = HandwritingStatisticsProfile::baseline(0);
    let mut seen_profile = false;
    let mut seen_edit = false;
    let (mut inspect, mut inventory_text) = (false, None);
    let mut preview_glyph = None;
    let mut output = None;
    let mut replace = false;
    let mut iter = args.iter();
    while let Some(name) = iter.next() {
        match name.as_str() {
            "--profile" => {
                if seen_profile || seen_edit {
                    return Err(
                        "--profile must appear once, before edits".into()
                    );
                }
                let selected =
                    iter.next().ok_or("--profile requires one input file")?;
                profile = read_handwriting_profile(selected)?;
                seen_profile = true;
            }
            "--seed" => {
                let value =
                    iter.next().ok_or("--seed requires an unsigned integer")?;
                profile.seed = parse_handwriting_seed(value)?;
                seen_edit = true;
            }
            "--set-global" => {
                let values = iter.by_ref().take(6).cloned().collect::<Vec<_>>();
                let parameter = parse_parameter(&values)?;
                profile = profile.with_global(parameter, &boundaries).map_err(
                    |error| format!("statistics refused: {error:?}"),
                )?;
                seen_edit = true;
            }
            "--set-glyph" => {
                let grapheme =
                    iter.next().ok_or("--set-glyph requires a grapheme")?;
                let values = iter.by_ref().take(6).cloned().collect::<Vec<_>>();
                let parameter = parse_parameter(&values)?;
                profile = profile
                    .with_glyph(grapheme, parameter, &boundaries)
                    .map_err(|error| {
                        format!("statistics refused: {error:?}")
                    })?;
                seen_edit = true;
            }
            "--remove-glyph" | "--remove-glyph-dimension" => {
                profile = remove_glyph_option(name, &mut iter, &profile)?;
                seen_edit = true;
            }
            "--inspect" if inspect =>
                return Err("--inspect may appear only once".into()),
            "--inspect" => inspect = true,
            "--inventory-text" => {
                let selected = parse_inventory_text(
                    &mut iter, inventory_text.as_deref(),
                )?;
                inventory_text = Some(selected);
            }
            "--preview-glyph" => {
                preview_glyph = Some(parse_preview_glyph(
                    &mut iter, preview_glyph.is_some(),
                )?);
            }
            "--output" => {
                output = Some(parse_selected_output(&mut iter,
                    output.is_some())?);
            }
            "--replace" => {
                if replace {
                    return Err("--replace may appear only once".into());
                }
                replace = true;
            }
            _ => return Err(format!("unknown handwriting option: {name}")),
        }
    }
    validate_handwriting_flags(
        inspect, inventory_text.as_deref(), preview_glyph.is_some(),
        output.as_deref(), replace,
    )?;
    profile
        .validate(&boundaries)
        .map_err(|error| format!("invalid handwriting profile: {error:?}"))?;
    Ok(HandwritingRequest {
        inspect,
        inventory_text,
        output,
        preview_glyph,
        profile,
        replace,
    })
}

const fn unit_name(unit: Unit) -> &'static str {
    match unit {
        Unit::Micrometres => "micrometres",
        Unit::Millidegrees => "millidegrees",
        Unit::PartsPerThousand => "parts-per-thousand",
    }
}

fn inspect_handwriting(profile: &HandwritingStatisticsProfile) -> Vec<u8> {
    let mut result = format!(
        "Version: {}\nSeed: {}\nGlobal parameters: {}\nGlyph overrides: {}\n",
        profile.version,
        profile.seed,
        profile.globals.len(),
        profile.glyphs.len(),
    );
    for global in &profile.globals {
        let value = global.statistics;
        let _written = writeln!(
            result,
            "{} ({}): {}..{}, mean {}, median {}, deviation {}",
            global.dimension.key(),
            unit_name(global.dimension.unit()),
            value.minimum,
            value.maximum,
            value.mean,
            value.median,
            value.standard_deviation,
        );
    }
    for glyph in &profile.glyphs {
        let _header_written = writeln!(result, "Glyph {}:", glyph.grapheme);
        for parameter in &glyph.parameters {
            let value = parameter.statistics;
            let _parameter_written = writeln!(
                result,
                "  {} ({}): {}..{}, mean {}, median {}, deviation {}",
                parameter.dimension.key(),
                unit_name(parameter.dimension.unit()),
                value.minimum,
                value.maximum,
                value.mean,
                value.median,
                value.standard_deviation,
            );
        }
    }
    result.into_bytes()
}

fn inventory_handwriting(
    profile: &HandwritingStatisticsProfile,
    text: &str,
) -> Result<Vec<u8>, String> {
    let mut distinct = Vec::new();
    let mut descriptions = String::new();
    let mut outside_baseline = 0usize;
    let mut remaining = strip_leading_source_bom(text);
    while !remaining.is_empty() {
        // Walk each suffix once; numbered offsets rescan long source prefixes.
        let (grapheme, rest) = first_grapheme(remaining)?;
        remaining = rest;
        // Native source line/paragraph boundaries never require ink.
        // Other exact Unicode whitespace remains auditable.
        if is_non_ink_audit_grapheme(grapheme)
            || distinct.contains(&grapheme)
        {
            continue;
        }
        distinct.push(grapheme);
        let listed = REQUIRED_TEXT_GRAPHEMES
            .iter()
            .any(|item| item.grapheme == grapheme);
        if !listed {
            outside_baseline = outside_baseline.saturating_add(1);
        }
        let overridden = profile
            .glyphs
            .iter()
            .any(|glyph| glyph.grapheme == grapheme);
        let codepoints = grapheme.chars()
            .map(|ch| format!("U+{:04X}", u32::from(ch)))
            .collect::<Vec<_>>().join(" ");
        let _written = writeln!(
            descriptions,
            "\"{}\": {} / {} [{codepoints}]",
            grapheme.escape_debug(),
            if listed {
                "text-baseline"
            } else {
                "outside-text-baseline"
            },
            if overridden {
                "statistical-override"
            } else {
                "global-statistics"
            },
        );
    }
    Ok(format!(
        concat!(
            "Exact text-inventory audit (not a renderer)\n",
            "Distinct visible graphemes: {}\n",
            "Outside English/Spanish text baseline: {}\n",
            "{}",
            "No glyph shapes or visible fallbacks are supplied by this ",
            "statistics profile; membership does not imply renderability.\n",
        ),
        distinct.len(),
        outside_baseline,
        descriptions,
    )
    .into_bytes())
}

fn preview_handwriting(
    profile: &HandwritingStatisticsProfile,
    grapheme: &str,
    occurrence: u64,
) -> Result<Vec<u8>, String> {
    let mut report = format!(
        concat!(
            "Seeded numeric preview (not ink or calibrated statistics)\n",
            "Grapheme: \"{}\"\nSeed: {}\nOccurrence: {}\n",
            "Median and exact population moments are not reproduced.\n",
        ),
        grapheme.escape_debug(),
        profile.seed,
        occurrence,
    );
    for dimension in Dimension::ALL {
        let value = preview_value(
            profile,
            grapheme,
            dimension,
            occurrence,
            &UnicodeGraphemeSegmentation,
        )
        .map_err(|error| format!("numeric preview refused: {error:?}"))?;
        let _written = writeln!(
            report,
            "{} ({}): {value}",
            dimension.key(),
            unit_name(dimension.unit()),
        );
    }
    Ok(report.into_bytes())
}

fn execute_handwriting(args: &[String]) -> Result<Option<Vec<u8>>, String> {
    let request = parse_handwriting_options(args)?;
    let boundaries = UnicodeGraphemeSegmentation;
    let canonical = encode_statistics_json(&request.profile, &boundaries)
        .map_err(|error| format!("statistics encoding refused: {error:?}"))?;
    if request.inspect {
        return Ok(Some(inspect_handwriting(&request.profile)));
    }
    if let Some(text) = request.inventory_text.as_deref() {
        return inventory_handwriting(&request.profile, text).map(Some);
    }
    if let Some((grapheme, occurrence)) = request.preview_glyph.as_ref() {
        return preview_handwriting(
            &request.profile, grapheme, *occurrence,
        ).map(Some);
    }
    if let Some(output) = request.output {
        let disposition = if request.replace {
            ProfileFileCommitDisposition::ReplaceExisting
        } else {
            ProfileFileCommitDisposition::PreserveExisting
        };
        let _evidence = commit_profile_file(
            Path::new(&output),
            &canonical,
            MAXIMUM_STATISTICS_JSON_BYTES,
            disposition,
        )
        .map_err(|error| {
            if error.may_have_committed() {
                format!(
                    "output durability unknown; inspect before retry: {error:?}"
                )
            } else {
                format!("output not saved: {error:?}")
            }
        })?;
        return Ok(None);
    }
    Ok(Some(canonical))
}

fn read_notebook_input(path: &str, limit: usize) -> Result<Vec<u8>, String> {
    let file = open_selected_profile_file(Path::new(path))
        .map_err(|error| format!("notebook input refused: {error:?}"))?;
    let bytes = read_bounded_profile_file(file, limit)
        .map_err(|error| format!("notebook input refused: {error:?}"))?;
    Ok(bytes.bytes().to_vec())
}

// The CLI's explicit `-` source is bounded before UTF-8 or format
// admission. Never read an unbounded pipe or silently persist its contents.
fn read_bounded_stdin(limit: usize, label: &str) -> Result<Vec<u8>, String> {
    let maximum = u64::try_from(
        limit.checked_add(1).ok_or("stdin input limit overflow")?,
    )
    .map_err(|_error| "stdin input limit exceeds u64")?;
    let mut bytes = Vec::new();
    let _read = io::stdin()
        .lock()
        .take(maximum)
        .read_to_end(&mut bytes)
        .map_err(|_error| format!("{label} stdin read failed"))?;
    if bytes.len() > limit {
        return Err(format!("{label} stdin exceeds {limit} UTF-8 bytes"));
    }
    Ok(bytes)
}

fn read_glyph_input(
    path: &str, stdin_selected: &mut bool,
) -> Result<Vec<u8>, String> {
    if path == "-" {
        if *stdin_selected {
            return Err("standard input may be selected only once".into());
        }
        *stdin_selected = true;
        return read_bounded_stdin(MAX_GLYPH_LIBRARY_BYTES, "glyph");
    }
    let file = open_selected_profile_file(Path::new(path))
        .map_err(|error| format!("glyph file refused: {error:?}"))?;
    let bytes = read_bounded_profile_file(file, MAX_GLYPH_LIBRARY_BYTES)
        .map_err(|error| format!("glyph file refused: {error:?}"))?;
    Ok(bytes.bytes().to_vec())
}

// Reject invalid library source order before opening any path or blocking
// on an unclosed stdin pipe. The parser also checks typed callers.
fn load_selected_glyph_file(
    glyphs: &mut GlyphOverlay, path: &str,
    stdin_selected: &mut bool,
) -> Result<(), String> {
    glyphs.ensure_can_load_file()?;
    let bytes = read_glyph_input(path, stdin_selected)?;
    glyphs.load_file_bytes(&bytes)
}

/// An explicit reusable glyph shape file, distinct from numeric statistics.
/// Keep human-entered shape coverage audits bounded and explicit.
fn validate_glyph_audit_text(selected: &str) -> Result<(), String> {
    let admitted = strip_leading_source_bom(selected);
    if selected.is_empty()
        || selected.len() > MAX_INVENTORY_TEXT_BYTES
        // Keep Unicode line/paragraph separators non-ink like CRLF/LF;
        // other Unicode spaces can have exact authored pen strokes.
        || !admitted.chars().any(|ch| !is_non_ink_audit_character(ch))
        || selected.chars().any(|ch| {
            ch.is_control() && !matches!(ch, '\t' | '\r' | '\n')
        })
        || selected.replace("\r\n", "").contains('\r') {
        return Err(concat!(
            "glyph audit needs 1..4096 UTF-8 bytes of ",
            "visible text without control characters",
        ).into());
    }
    Ok(())
}

// A later incomplete glyph command must refuse before a previously selected
// library may block on stdin. Preserve literal flag-looking authored values,
// but require an explicit spelling for selected library paths.
fn preflight_glyph_operands(
    option: &str, count: usize, iter: &mut Iter<'_, String>,
) -> Result<(), String> {
    for _ in 0..count {
        let selected = iter.next().ok_or_else(|| match option {
            "--profile" => "--profile requires a glyph library path".into(),
            "--audit-text" => "--audit-text requires UTF-8 text".into(),
            "--inspect-glyph" | "--inspect-paths" =>
                "inspection requires an exact grapheme".into(),
            "--remove-glyph" =>
                "--remove-glyph requires an exact grapheme".into(),
            _ => format!("{option} requires {count} operands"),
        })?;
        if option == "--profile" && (selected.is_empty()
            || (selected.starts_with('-') && selected != "-")) {
            return Err(concat!(
                "glyphs --profile requires an explicit library path ",
                "(use ./--name or - for stdin)",
            ).into());
        }
    }
    Ok(())
}

// Admit source-independent glyph shape, path and transform operands with
// the same bounded native readers used by final edits. Library-dependent
// stroke/path existence and transformed pen clearance remain deferred.
fn preflight_glyph_typed_option(
    option: &str, iter: &mut Iter<'_, String>,
    validated_strokes: &mut GlyphOverlay,
) -> Result<bool, String> {
    let count = match option {
        "--glyph-stroke" => 7,
        "--rotate-path-ink" => 5,
        "--set-stroke" => 8,
        "--join-next-path-curved" | "--close-path-curved" | "--translate-path"
        | "--style-path" | "--scale-path" | "--shear-path"
        | "--reflect-path" | "--rotate-glyph" => 4,
        "--move-stroke" | "--move-path" | "--rotate-path"
        | "--shear-glyph" | "--scale-glyph-height"
        | "--scale-glyph-width" | "--translate-glyph"
        | "--reflect-glyph" => 3,
        "--reverse-stroke" | "--split-stroke"
        | "--split-stroke-exact" | "--merge-strokes"
        | "--join-strokes" | "--remove-stroke"
        | "--reverse-path" | "--close-path" | "--join-next-path"
        | "--remove-path"
        | "--scale-glyph-ink" => 2,
        _ => return Ok(false),
    };
    let mut complete = iter.clone();
    preflight_glyph_operands(option, count, &mut complete)?;
    match option {
        "--glyph-stroke" => validated_strokes.parse(option, iter)?,
        "--set-stroke" | "--move-stroke" | "--reverse-stroke"
        | "--split-stroke" | "--split-stroke-exact"
        | "--merge-strokes" | "--join-strokes"
        | "--remove-stroke" | "--reverse-path"
        | "--close-path" | "--join-next-path"
        | "--join-next-path-curved" | "--close-path-curved" | "--rotate-path"
        | "--move-path"
        | "--translate-path" | "--style-path" | "--scale-path"
        | "--reflect-path" | "--shear-path" | "--remove-path"
        | "--rotate-path-ink" =>
            GlyphOverlay::preflight_stroke_or_path_edit(option, iter)?,
        _ => GlyphOverlay::preflight_transform_option(option, iter)?,
    }
    Ok(true)
}

// A repeated selected glyph library is invalid even if the first source is
// an unclosed stdin pipe. Respect command operand counts so authored text
// such as "--profile" cannot masquerade as another source selection.
// Validate an explicit path-copy grammar before loading a selected TSV.
// The shifted form shares source/path/target identity, but needs two offsets.
fn preflight_glyph_copy_path(
    option: &str, iter: &mut Iter<'_, String>,
) -> Result<(), String> {
    let shifted = option == "--copy-path-shifted";
    let mut complete = iter.clone();
    preflight_glyph_operands(option, if shifted { 5 } else { 3 },
        &mut complete)?;
    if shifted {
        GlyphOverlay::preflight_copy_path_shifted_option(iter)
    } else {
        GlyphOverlay::preflight_copy_path_option(iter)
    }
}

fn preflight_glyph_profile_sources(args: &[String]) -> Result<(), String> {
    let mut iter = args.iter();
    let mut profile_selected = false;
    let mut audit_selected = false;
    let mut inspection_selected = false;
    let mut output_selected = false;
    let mut replace_selected = false;
    // Authored stroke geometry is independent of an imported glyph library.
    // Admit it before an earlier --profile - source can wait for stdin EOF.
    let mut validated_strokes = GlyphOverlay::default();
    while let Some(option) = iter.next() {
        if preflight_glyph_typed_option(
            option, &mut iter, &mut validated_strokes,
        )? {
            continue;
        }
        let operands: usize = match option.as_str() {
            "--audit-baseline" => {
                preflight_unique_selection(&mut audit_selected,
                    concat!(
                        "--audit-text and --audit-baseline ",
                        "may be selected only once",
                    ))?;
                0
            }
            "--replace" => {
                preflight_unique_selection(&mut replace_selected,
                    "--replace may appear only once")?;
                0
            }
            "--copy-path" | "--copy-path-shifted" => {
                preflight_glyph_copy_path(option, &mut iter)?;
                0
            }
            "--copy-glyph" => {
                let mut complete = iter.clone();
                preflight_glyph_operands(option, 2, &mut complete)?;
                let source = iter.next()
                    .ok_or("--copy-glyph requires 2 operands")?;
                let target = iter.next()
                    .ok_or("--copy-glyph requires 2 operands")?;
                glyph::validate_copy_glyph_keys(source, target)?;
                0
            }
            "--profile" => {
                if profile_selected {
                    return Err(concat!(
                        "--glyph-file or --profile must precede authored ",
                        "strokes and may appear only once",
                    ).into());
                }
                profile_selected = true;
                1
            }
            "--audit-text" => {
                preflight_unique_selection(&mut audit_selected,
                    "--audit-text may appear only once")?;
                let selected = iter.next()
                    .ok_or("--audit-text requires UTF-8 text")?;
                validate_glyph_audit_text(selected)?;
                0
            }
            "--inspect-glyph" | "--inspect-paths" => {
                preflight_unique_selection(&mut inspection_selected,
                    "glyph inspection may appear only once")?;
                let selected = iter.next()
                    .ok_or("inspection requires an exact grapheme")?;
                glyph::validate_inspection_glyph_key(selected)?;
                0
            }
            "--output" => {
                preflight_unique_selection(&mut output_selected,
                    "--output requires one explicit filename")?;
                let _selected = parse_selected_output(&mut iter, false)?;
                0
            }
            "--remove-glyph" => {
                let selected = iter.next()
                    .ok_or("--remove-glyph requires an exact grapheme")?;
                glyph::validate_removal_glyph_key(selected)?;
                0
            }
            _ => return Err(format!("unknown glyph library option: {option}")),
        };
        preflight_glyph_operands(option, operands, &mut iter)?;
    }
    if inspection_selected && audit_selected {
        return Err("glyph inspection cannot accompany a glyph audit".into());
    }
    if inspection_selected && (output_selected || replace_selected) {
        return Err("glyph inspection is stdout-only; omit --output".into());
    }
    if audit_selected && (output_selected || replace_selected) {
        return Err("--audit-text is stdout-only; omit --output".into());
    }
    if replace_selected && !output_selected {
        return Err("--replace requires --output".into());
    }
    Ok(())
}

fn execute_glyph_library(args: &[String]) -> Result<Option<Vec<u8>>, String> {
    preflight_glyph_profile_sources(args)?;
    let mut glyphs = GlyphOverlay::default();
    let (mut output, mut audit_text) = (None, None);
    let (mut audit_baseline, mut replace, mut piped) = (false, false, false);
    let mut inspection: Option<(String, bool)> = None;
    let mut iter = args.iter();
    while let Some(option) = iter.next() {
        match option.as_str() {
            "--profile" => {
                let selected = iter.next()
                    .ok_or("--profile requires a glyph library path")?;
                load_selected_glyph_file(&mut glyphs, selected, &mut piped)?;
            }
            "--copy-path" => glyphs.copy_path_option(&mut iter)?,
            "--copy-path-shifted" =>
                glyphs.copy_path_shifted_option(&mut iter)?,
            "--copy-glyph" | "--glyph-stroke" | "--rotate-glyph"
            | "--reflect-glyph"
            | "--translate-glyph" | "--shear-glyph"
            | "--scale-glyph-height" | "--scale-glyph-width"
            | "--scale-glyph-ink" => glyphs.parse(option, &mut iter)?,
            "--reverse-stroke" | "--move-stroke" | "--join-strokes"
            | "--split-stroke" | "--split-stroke-exact"
            | "--merge-strokes" | "--set-stroke" | "--remove-stroke" =>
                glyphs.edit_stroke(option, &mut iter)?,
            "--reverse-path" | "--rotate-path" | "--close-path"
            | "--join-next-path" | "--join-next-path-curved"
            | "--close-path-curved"
            | "--move-path"
            | "--translate-path" | "--style-path" | "--scale-path"
            | "--reflect-path" | "--shear-path" | "--remove-path"
            | "--rotate-path-ink" =>
                glyphs.edit_path(option, &mut iter)?,
            "--remove-glyph" => {
                let grapheme = iter.next()
                    .ok_or("--remove-glyph requires an exact grapheme")?;
                glyphs.remove_glyph(grapheme)?;
            }
            "--audit-baseline" => {
                if audit_text.is_some() {
                    return Err(concat!(
                        "--audit-text and --audit-baseline ",
                        "may be selected only once",
                    ).into());
                }
                audit_baseline = true;
                audit_text = Some(REQUIRED_TEXT_GRAPHEMES.iter()
                    .map(|entry| entry.grapheme)
                    .collect::<Vec<_>>().join(" "));
            }
            "--audit-text" => {
                if audit_text.is_some() {
                    return Err("--audit-text may appear only once".into());
                }
                let selected = iter.next()
                    .ok_or("--audit-text requires UTF-8 text")?;
                validate_glyph_audit_text(selected)?;
                audit_text = Some(selected.clone());
            }
            "--inspect-paths" | "--inspect-glyph" => {
                if inspection.is_some() {
                    return Err("glyph inspection may appear only once".into());
                }
                inspection = Some((iter.next()
                    .ok_or("inspection requires an exact grapheme")?
                    .clone(), option == "--inspect-paths"));
            }
            "--output" => {
                output = Some(parse_selected_output(&mut iter,
                    output.is_some())?);
            }
            "--replace" => {
                if replace {
                    return Err("--replace may appear only once".into());
                }
                replace = true;
            }
            _ => return Err(format!("unknown glyph library option: {option}")),
        }
    }
    if replace && output.is_none() {
        return Err("--replace requires --output".into());
    }
    if let Some((grapheme, paths)) = inspection {
        if audit_text.is_some() {
            return Err(concat!(
                "glyph inspection cannot accompany ",
                "a glyph audit",
            ).into());
        }
        if output.is_some() || replace {
            return Err("glyph inspection is stdout-only; omit --output".into());
        }
        return if paths {
            glyphs.inspect_paths(&grapheme).map(Some)
        } else {
            glyphs.inspect_stroke_order(&grapheme).map(Some)
        };
    }
    if let Some(text) = audit_text {
        if output.is_some() || replace {
            return Err("--audit-text is stdout-only; omit --output".into());
        }
        return glyph_audit_report(&glyphs, &text, audit_baseline).map(Some);
    }
    let bytes = glyphs.encode_file()?;
    if let Some(path) = output {
        save_glyph_library_file(&path, &bytes, replace)?;
        return Ok(None);
    }
    Ok(Some(bytes))
}

fn glyph_audit_report(
    glyphs: &GlyphOverlay, text: &str, audit_baseline: bool,
) -> Result<Vec<u8>, String> {
    let mut report = glyphs.audit_text(text)?;
    if audit_baseline {
        let mut label = concat!(
            "Frozen English/Spanish text baseline; ",
            "math symbols are separate.\n",
        ).as_bytes().to_vec();
        label.append(&mut report);
        return Ok(label);
    }
    Ok(report)
}

fn save_glyph_library_file(
    path: &str, bytes: &[u8], replace: bool,
) -> Result<(), String> {
    let disposition = if replace {
        ProfileFileCommitDisposition::ReplaceExisting
    } else {
        ProfileFileCommitDisposition::PreserveExisting
    };
    let _evidence = commit_profile_file(
        Path::new(path), bytes, MAX_GLYPH_LIBRARY_BYTES, disposition,
    )
    .map_err(|error| {
        if error.may_have_committed() {
            format!(
                "{}: {error:?}",
                "glyph save durability unknown; inspect before retry",
            )
        } else {
            format!("glyph library not saved: {error:?}")
        }
    })?;
    Ok(())
}

fn apply_notebook_edit(
    profile: &mut NotebookProfile,
    option: &str,
    value: &str,
) -> Result<(), String> {
    match option {
        "--date-style" => profile.date_style = DateStyle::parse(value)?,
        "--date-text" => {
            validate_printed_date_text(value)?;
            value.clone_into(&mut profile.date_text);
        }
        "--date-placement" => {
            profile.date_placement = DatePlacement::parse(value)?;
        }
        "--border-radius-um" => {
            if value.is_empty()
                || !value.bytes().all(|byte| byte.is_ascii_digit())
            {
                return Err(
                    "border radius requires unsigned integer micrometres"
                        .into(),
                );
            }
            profile.border_radius_um = value
                .parse::<u32>()
                .map_err(|error| format!("invalid border radius: {error}"))?;
        }
        "--page-numbers" | "--printed-sheet" => {
            let enabled = match value {
                "true" => true,
                "false" => false,
                _ => return Err(format!("{option} requires true or false")),
            };
            if option == "--page-numbers" {
                profile.page_numbers = enabled;
            } else {
                profile.printed_sheet = enabled;
            }
        }
        _ => return Err(format!("unsupported notebook edit: {option}")),
    }
    Ok(())
}

// Reuse the final notebook edit parser for source-independent arguments.
// Profile-level combinations still validate after loading the source.
fn preflight_notebook_edit(
    option: &str, iter: &mut Iter<'_, String>,
    profile: &mut NotebookProfile,
) -> Result<(), String> {
    let value = iter.next()
        .ok_or_else(|| format!("{option} requires a value"))?;
    apply_notebook_edit(profile, option, value)
}

fn export_integer(
    text: Option<&str>,
    option: &str,
    minimum: u32,
    maximum: u32,
) -> Result<u32, String> {
    let value = text.ok_or_else(|| format!("{option} requires a value"))?;
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(format!("{option} requires {minimum}..{maximum}"));
    }
    let number = value.parse::<u32>()
        .map_err(|error| format!("{option} integer refused: {error}"))?;
    if !(minimum..=maximum).contains(&number) {
        return Err(format!("{option} requires {minimum}..{maximum}"));
    }
    Ok(number)
}

// Keep source-independent quality selection identical in preflight and
// final output parsing, before any earlier stdin source can block.
fn parse_export_quality(selected: Option<&str>) -> Result<u32, String> {
    match selected {
        Some("preview") => Ok(72),
        Some("final") => Ok(150),
        _ => Err("--quality requires preview or final".into()),
    }
}

fn parse_ink_line(
    iter: &mut Iter<'_, String>,
) -> Result<NotebookInkLine, String> {
    let page = export_integer(
        iter.next().map(String::as_str), "ink page", 1, 32,
    )?;
    let x1 = export_integer(
        iter.next().map(String::as_str), "ink x1", 0, 700_000,
    )?;
    let y1 = export_integer(
        iter.next().map(String::as_str), "ink y1", 0, 700_000,
    )?;
    let x2 = export_integer(
        iter.next().map(String::as_str), "ink x2", 0, 700_000,
    )?;
    let y2 = export_integer(
        iter.next().map(String::as_str), "ink y2", 0, 700_000,
    )?;
    let width_um = export_integer(
        iter.next().map(String::as_str), "ink width", 80, 1_500,
    )?;
    let color = iter.next().ok_or("ink line needs #RRGGBB")?;
    if color.len() != 7 || !color.starts_with('#')
        || !color.bytes().skip(1).all(|byte| byte.is_ascii_hexdigit()) {
        return Err("ink color must be #RRGGBB".into());
    }
    let byte = |start: usize| -> Result<u8, String> {
        let end = start.saturating_add(2);
        let digits = color.get(start..end).ok_or("ink color must be #RRGGBB")?;
        u8::from_str_radix(digits, 16)
            .map_err(|_error| "ink color must be #RRGGBB".into())
    };
    let line = NotebookInkLine {
        page: usize::try_from(page)
            .map_err(|error| format!("invalid ink page: {error}"))?,
        start_um: (x1, y1),
        end_um: (x2, y2),
        width_um,
        rgb: [byte(1)?, byte(3)?, byte(5)?],
    };
    validate_native_ink_line(&line)?;
    Ok(line)
}

// This profile-independent admission is shared by CLI-parsed manual ink and
// direct typed callers. The 700-mm coordinate ceiling includes pen clearance,
// not just the centerline. The downstream paper renderer additionally verifies
// the selected sheet's margins and printable region.
fn validate_native_ink_line(line: &NotebookInkLine) -> Result<(), String> {
    let clearance = line.width_um.div_ceil(2);
    if !(1..=32).contains(&line.page)
        || !(80..=1_500).contains(&line.width_um)
        || line.start_um == line.end_um
        || [line.start_um, line.end_um].iter().any(|(x, y)| {
            *x < clearance || *y < clearance
                || x.checked_add(clearance)
                    .is_none_or(|edge| edge > 700_000)
                || y.checked_add(clearance)
                    .is_none_or(|edge| edge > 700_000)
        })
    {
        return Err(format!(
            "page {} ink needs printable endpoints and valid width",
            line.page,
        ));
    }
    Ok(())
}

// Reuse final ink parsing while bounding explicit lines before selected IO.
fn preflight_inline_ink_line(
    iter: &mut Iter<'_, String>, lines: &mut Vec<NotebookInkLine>,
) -> Result<(), String> {
    if lines.len() == MAX_NOTEBOOK_INK_LINES {
        return Err("ink exceeds 4096 explicit line segments".into());
    }
    lines.push(parse_ink_line(iter)?);
    Ok(())
}

// A caller-selected page ink library is a strict portable source of the
// same physical segments as --ink-line. It is never notebook v1 state.
fn load_page_ink_file(
    path: &str,
    ink: &mut Vec<NotebookInkLine>,
) -> Result<(), String> {
    // Every valid file has at least one row. If no capacity remains,
    // refuse before opening another file or waiting for stdin EOF.
    if ink.len() >= MAX_NOTEBOOK_INK_LINES {
        return Err("ink exceeds 4096 explicit line segments".into());
    }
    let bytes = if path == "-" {
        read_bounded_stdin(MAX_PAGE_INK_FILE_BYTES, "ink")?
    } else {
        read_notebook_input(path, MAX_PAGE_INK_FILE_BYTES)?
    };
    let source = from_utf8(&bytes).map_err(|_error| {
        if path == "-" {
            "ink stdin must be UTF-8"
        } else {
            "ink file must be UTF-8"
        }
    })?;
    let mut lines = source.lines();
    if lines.next() != Some(PAGE_INK_FILE_HEADER) {
        return Err("ink file needs atrament.page-ink.v1 header".into());
    }
    // Keep previously authored manual ink unchanged if any later file row
    // fails. The CLI exits on refusal, but typed in-memory callers should
    // not inherit a valid prefix from an invalid imported layer.
    let mut staged = Vec::new();
    for (index, row) in lines.enumerate() {
        let values = row.split('\t').map(str::to_owned).collect::<Vec<_>>();
        if values.len() != 7 {
            return Err(format!(
                "ink file line {} needs seven tab-separated fields",
                index.saturating_add(2),
            ));
        }
        if ink.len().saturating_add(staged.len()) >= MAX_NOTEBOOK_INK_LINES {
            return Err("ink exceeds 4096 explicit line segments".into());
        }
        let segment = parse_ink_line(&mut values.iter()).map_err(|error| {
            format!(
                "ink file line {} refused: {error}",
                index.saturating_add(2)
            )
        })?;
        staged.push(segment);
    }
    if staged.is_empty() {
        return Err("ink file needs at least one physical segment".into());
    }
    ink.extend(staged);
    Ok(())
}

// Reusable *physical* layer artifact: no paper/date decorations, semantic
// source or authored glyph definitions are silently treated as persisted.
fn encode_page_ink_file(ink: &[NotebookInkLine]) -> Result<Vec<u8>, String> {
    if ink.is_empty() {
        return Err(
            "page ink export needs at least one physical segment".into()
        );
    }
    let mut text = format!("{PAGE_INK_FILE_HEADER}\n");
    for line in ink {
        writeln!(
            text,
            "{}\t{}\t{}\t{}\t{}\t{}\t#{:02X}{:02X}{:02X}",
            line.page, line.start_um.0, line.start_um.1,
            line.end_um.0, line.end_um.1, line.width_um,
            line.rgb[0], line.rgb[1], line.rgb[2],
        )
        .map_err(|error| format!("page ink TSV encoding refused: {error}"))?;
        if text.len() > MAX_PAGE_INK_FILE_BYTES {
            return Err("page ink TSV exceeds 262144 bytes".into());
        }
    }
    Ok(text.into_bytes())
}

fn parse_notebook_export_option(
    option: &str,
    iter: &mut Iter<'_, String>,
    export: &mut NotebookExportOptions,
    ink: &mut Vec<NotebookInkLine>,
    glyphs: &mut GlyphOverlay,
    stdin_selected: &mut bool,
) -> Result<(), String> {
    match option {
        "--emit-ink-file" => {
            if export.format.is_some() {
                return Err(concat!(
                    "--emit-ink-file may appear only once and ",
                    "cannot accompany --export-paper",
                ).into());
            }
            export.format = Some(NotebookExportFormat::InkTsv);
        }
        "--export-paper" => {
            if export.format.is_some() {
                return Err(concat!(
                    "--export-paper may appear only once and ",
                    "cannot accompany --emit-ink-file",
                ).into());
            }
            export.format = Some(match iter.next().map(String::as_str) {
                Some("pdf") => NotebookExportFormat::Pdf,
                Some("png") => NotebookExportFormat::Png,
                _ => return Err("--export-paper requires pdf or png".into()),
            });
        }
        "--ruler-seed" | "--ruler-error-mm" => {
            let field = if option == "--ruler-seed" {
                &mut export.ruler_seed
            } else {
                &mut export.ruler_error_mm
            };
            if field.is_some() {
                return Err(format!("{option} may appear only once"));
            }
            *field = Some(iter.next().ok_or_else(|| {
                format!("{option} requires a decimal value")
            })?.to_owned());
        }
        "--pages" => {
            if export.pages.is_some() {
                return Err("--pages may appear only once".into());
            }
            let count = export_integer(
                iter.next().map(String::as_str), "--pages", 1, 32,
            )?;
            export.pages = Some(usize::try_from(count)
                .map_err(|error| format!("page count refused: {error}"))?);
        }
        "--dpi" => {
            if export.dpi.is_some() {
                return Err("--dpi may appear only once".into());
            }
            export.dpi = Some(export_integer(
                iter.next().map(String::as_str), "--dpi", 72, 300,
            )?);
        }
        "--quality" => {
            if export.quality_dpi.is_some() {
                return Err("--quality may appear only once".into());
            }
            export.quality_dpi = Some(parse_export_quality(
                iter.next().map(String::as_str),
            )?);
        }
        "--png-page" => {
            if export.png_page.is_some() {
                return Err("--png-page may appear only once".into());
            }
            let page = export_integer(
                iter.next().map(String::as_str), "--png-page", 1, 32,
            )?;
            export.png_page = Some(usize::try_from(page)
                .map_err(|error| format!("PNG page refused: {error}"))?);
        }
        "--glyph-stroke" | "--place-text" | "--fallback-glyph"
        | "--glyph-variation" | "--glyph-line-drift"
        | "--glyph-word-drift" | "--glyph-word-rise"
        | "--glyph-word-height" | "--glyph-word-slant"
        | "--glyph-word-pressure" | "--glyph-word-pigment"
        | "--glyph-word-width"
        | "--glyph-word-tracking" | "--glyph-word-baseline-step" => {
            glyphs.parse(option, iter)?;
        }
        "--join-text-pair" => glyphs.add_text_join_pair(iter)?,
        "--join-text-anchors" => glyphs.add_text_join_anchors(iter)?,
        "--glyph-file" => {
            let selected = iter.next()
                .ok_or("--glyph-file requires a library path")?;
            load_selected_glyph_file(glyphs, selected, stdin_selected)?;
        }
        "--ink-file" => {
            let path = iter.next().ok_or("--ink-file requires a file")?;
            if path == "-" {
                if *stdin_selected {
                    return Err(
                        "standard input may be selected only once".into()
                    );
                }
                *stdin_selected = true;
            }
            load_page_ink_file(path, ink)?;
        }
        "--ink-line" => {
            if ink.len() == MAX_NOTEBOOK_INK_LINES {
                return Err("ink exceeds 4096 explicit line segments".into());
            }
            ink.push(parse_ink_line(iter)?);
        }
        _ => return Err(format!("unknown notebook export option: {option}")),
    }
    Ok(())
}

fn validate_notebook_export_flags(
    replace: bool,
    output: Option<&str>,
    inspect: bool, inspect_flow: bool,
    inspect_flow_shape_coverage: bool,
    export: &NotebookExportOptions,
    ink: &[NotebookInkLine],
    glyphs: &GlyphOverlay,
    statistics: Option<&HandwritingStatisticsProfile>,
) -> Result<(), String> {
    if replace && output.is_none() {
        return Err("--replace requires --output".into());
    }
    if inspect && output.is_some() {
        return Err("--inspect is stdout only; omit --output".into());
    }
    if inspect && export.format.is_some() {
        return Err("--inspect cannot export paper or page ink".into());
    }
    if export.pages.is_some()
        && export.format != Some(NotebookExportFormat::Pdf)
    {
        return Err("--pages requires --export-paper pdf".into());
    }
    if (export.dpi.is_some() || export.png_page.is_some()
        || export.quality_dpi.is_some())
        && export.format != Some(NotebookExportFormat::Png)
    {
        return Err(
            "--dpi, --quality and --png-page require --export-paper png".into()
        );
    }
    if export.dpi.is_some() && export.quality_dpi.is_some() {
        return Err("--quality and --dpi are mutually exclusive".into());
    }
    if resolve_notebook_ruler(
        export.ruler_seed.as_deref(), export.ruler_error_mm.as_deref(),
    )?.is_some() && !matches!(
        export.format,
        Some(NotebookExportFormat::Pdf | NotebookExportFormat::Png)
    ) {
        return Err("ruler variation requires --export-paper pdf or png"
            .into());
    }
    if (glyphs.has_any() || !ink.is_empty()) && export.format.is_none()
        && !(inspect_flow && inspect_flow_shape_coverage) {
        return Err(concat!(
            "ink and glyph overlays need ",
            "--export-paper pdf or png or --emit-ink-file",
        ).into());
    }
    if glyphs.variation.is_some() && glyphs.strokes.is_empty() {
        return Err("--glyph-variation needs authored glyph shapes".into());
    }
    if statistics.is_some() && !glyphs.has_any() && !inspect_flow {
        return Err(concat!(
            "--handwriting-profile requires both --glyph-stroke ",
            "and --place-text",
        ).into());
    }
    Ok(())
}

// A selected statistics filename is source-independent. Share the same
// admission boundary before and after any earlier selected stdin source.
fn parse_notebook_handwriting_path<'source>(
    iter: &mut Iter<'source, String>,
) -> Result<&'source str, String> {
    let path = iter.next().ok_or(
        "--handwriting-profile requires a saved profile path"
    )?;
    if path.is_empty() || path.starts_with('-') {
        return Err("invalid handwriting profile path".into());
    }
    Ok(path)
}

fn parse_notebook_handwriting_profile(
    iter: &mut Iter<'_, String>,
    statistics: &mut Option<HandwritingStatisticsProfile>,
) -> Result<(), String> {
    if statistics.is_some() {
        return Err("--handwriting-profile may appear only once".into());
    }
    let path = parse_notebook_handwriting_path(iter)?;
    *statistics = Some(read_handwriting_profile(path)?);
    Ok(())
}

fn parse_notebook_flow(
    iter: &mut Iter<'_, String>,
    option: &str,
    flow: &mut Option<FlowSource>,
    stdin_selected: &mut bool,
) -> Result<(), String> {
    if flow.is_some() {
        return Err("only one flow source may be supplied".into());
    }
    let source = iter.next().ok_or(
        "flow needs a UTF-8 text value or a selected file path"
    )?;
    let format = if option.starts_with("--flow-text") {
        FlowFormat::Plain
    } else {
        FlowFormat::Markdown
    };
    if option.ends_with("-file") && source == "-" {
        if *stdin_selected {
            return Err("standard input may be selected only once".into());
        }
        *stdin_selected = true;
    }
    *flow = Some(if option.ends_with("-file") {
        FlowSource::from_file(format, source)?
    } else {
        FlowSource::new(format, source)?
    });
    Ok(())
}

fn parse_notebook_flow_pitch(
    iter: &mut Iter<'_, String>,
) -> Result<u32, String> {
    let raw = iter.next()
        .ok_or("--flow-pitch-um requires 9000..30000 um")?;
    if raw.is_empty() || !raw.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("flow pitch must be within 9000..30000 um".into());
    }
    let pitch = raw.parse::<u32>()
        .map_err(|_error| "flow pitch must be within 9000..30000 um")?;
    if !(9_000..=30_000).contains(&pitch) {
        return Err("flow pitch must be within 9000..30000 um".into());
    }
    Ok(pitch)
}

fn select_notebook_flow_pitch(
    iter: &mut Iter<'_, String>, selected: &mut Option<u32>,
) -> Result<(), String> {
    if selected.is_some() {
        return Err("--flow-pitch-um may appear only once".into());
    }
    *selected = Some(parse_notebook_flow_pitch(iter)?);
    Ok(())
}

fn parse_notebook_flow_paragraph_gap(
    iter: &mut Iter<'_, String>,
) -> Result<u32, String> {
    let raw = iter.next()
        .ok_or("--flow-paragraph-gap-um requires 0..12000 um")?;
    if raw.is_empty() || !raw.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("flow paragraph gap must be within 0..12000 um".into());
    }
    let gap = raw.parse::<u32>()
        .map_err(|_error| "flow paragraph gap must be within 0..12000 um")?;
    if gap > 12_000 {
        return Err("flow paragraph gap must be within 0..12000 um".into());
    }
    Ok(gap)
}

fn select_notebook_flow_paragraph_gap(
    iter: &mut Iter<'_, String>, selected: &mut Option<u32>,
) -> Result<(), String> {
    if selected.is_some() {
        return Err("--flow-paragraph-gap-um may appear only once".into());
    }
    *selected = Some(parse_notebook_flow_paragraph_gap(iter)?);
    Ok(())
}

// Reuse the physical 0..12 mm bound on both preflight and typed layout.
fn parse_notebook_flow_body_inset(
    iter: &mut Iter<'_, String>, right: bool,
) -> Result<u32, String> {
    let option = if right { "--flow-body-right-inset-um" }
        else { "--flow-body-inset-um" };
    let label = if right { "flow body right inset" }
        else { "flow body inset" };
    parse_bounded_flow_inset(iter, option, label)
}

// All Markdown insets use one unsigned 0..12 mm parse contract. Retain
// their distinct option names and diagnostics in both preflight and CLI.
fn parse_bounded_flow_inset(
    iter: &mut Iter<'_, String>, option: &str, label: &str,
) -> Result<u32, String> {
    let raw = iter.next()
        .ok_or_else(|| format!("{option} requires 0..12000 um"))?;
    if raw.is_empty() || !raw.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(format!("{label} must be within 0..12000 um"));
    }
    let inset = raw.parse::<u32>()
        .map_err(|_error| format!("{label} must be within 0..12000 um"))?;
    if inset > 12_000 {
        return Err(format!("{label} must be within 0..12000 um"));
    }
    Ok(inset)
}

fn parse_notebook_flow_first_line_indent(
    iter: &mut Iter<'_, String>,
) -> Result<u32, String> {
    parse_bounded_flow_inset(iter,
        "--flow-body-first-line-indent-um", "flow body first-line indent")
}

fn parse_notebook_flow_continuation_indent(
    iter: &mut Iter<'_, String>,
) -> Result<u32, String> {
    parse_bounded_flow_inset(iter,
        "--flow-body-continuation-indent-um", "flow body continuation indent")
}

fn select_flow_inset(
    iter: &mut Iter<'_, String>, typography: &mut FlowTypography,
    option: &str,
) -> Result<(), String> {
    if option == "--flow-body-first-line-indent-um" {
        if typography.body_first_line_indent.is_some() {
            return Err(format!("{option} may appear only once"));
        }
        typography.body_first_line_indent =
            Some(parse_notebook_flow_first_line_indent(iter)?);
        return Ok(());
    }
    if option == "--flow-body-continuation-indent-um" {
        if typography.body_continuation_indent.is_some() {
            return Err(format!("{option} may appear only once"));
        }
        typography.body_continuation_indent =
            Some(parse_notebook_flow_continuation_indent(iter)?);
        return Ok(());
    }
    if matches!(option, "--flow-list-quote-inset-um"
        | "--flow-list-quote-right-inset-um") {
        return select_list_quote_inset(iter, typography, option);
    }
    let right = option == "--flow-body-right-inset-um";
    let selected = if right { &mut typography.body_right_inset }
        else { &mut typography.body_inset };
    if selected.is_some() {
        return Err(format!("{option} may appear only once"));
    }
    *selected = Some(parse_notebook_flow_body_inset(iter, right)?);
    Ok(())
}

fn parse_notebook_flow_list_quote_inset(
    iter: &mut Iter<'_, String>, right: bool,
) -> Result<u32, String> {
    let option = if right { "--flow-list-quote-right-inset-um" }
        else { "--flow-list-quote-inset-um" };
    let label = if right { "flow list/quote right inset" }
        else { "flow list/quote inset" };
    parse_bounded_flow_inset(iter, option, label)
}

fn select_list_quote_inset(
    iter: &mut Iter<'_, String>, typography: &mut FlowTypography,
    option: &str,
) -> Result<(), String> {
    let right = option == "--flow-list-quote-right-inset-um";
    let selected = if right { &mut typography.list_quote_right_inset }
        else { &mut typography.list_quote_inset };
    if selected.is_some() {
        return Err(format!("{option} may appear only once"));
    }
    *selected = Some(parse_notebook_flow_list_quote_inset(iter, right)?);
    Ok(())
}

fn parse_notebook_flow_bullet_style(
    iter: &mut Iter<'_, String>,
) -> Result<BulletStyle, String> {
    match iter.next().map(String::as_str) {
        Some("dash") => Ok(BulletStyle::Dash),
        Some("dot") => Ok(BulletStyle::Dot),
        Some("ring") => Ok(BulletStyle::Ring),
        _ => Err("--flow-bullet-style requires dash, dot or ring".into()),
    }
}

fn parse_notebook_flow_task_style(
    iter: &mut Iter<'_, String>,
) -> Result<TaskStyle, String> {
    match iter.next().map(String::as_str) {
        Some("square") => Ok(TaskStyle::Square),
        Some("diamond") => Ok(TaskStyle::Diamond),
        _ => Err("--flow-task-style requires square or diamond".into()),
    }
}

// Parse a caller-selected structural pen color as exact six-digit RGB.
fn parse_notebook_flow_marker_color(
    iter: &mut Iter<'_, String>, option: &str,
) -> Result<[u8; 3], String> {
    let source = iter.next()
        .ok_or_else(|| format!("{option} requires #RRGGBB"))?;
    if source.len() != 7 || !source.starts_with('#')
        || !source.bytes().skip(1).all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!("{option} requires #RRGGBB"));
    }
    let parse = |start: usize| -> Result<u8, String> {
        let pair = source.get(start..start.saturating_add(2))
            .ok_or_else(|| format!("{option} requires #RRGGBB"))?;
        u8::from_str_radix(pair, 16)
            .map_err(|_error| format!("{option} requires #RRGGBB"))
    };
    Ok([parse(1)?, parse(3)?, parse(5)?])
}

fn parse_notebook_flow_align(
    iter: &mut Iter<'_, String>, option: &str,
) -> Result<BodyAlign, String> {
    match iter.next().map(String::as_str) {
        Some("left") => Ok(BodyAlign::Left),
        Some("center") => Ok(BodyAlign::Center),
        Some("right") => Ok(BodyAlign::Right),
        _ => Err(format!("{option} requires left, center or right")),
    }
}

fn select_notebook_flow_typography(
    iter: &mut Iter<'_, String>, typography: &mut FlowTypography,
    option: &str,
) -> Result<(), String> {
    match option {
        "--flow-pitch-um" =>
            select_notebook_flow_pitch(iter, &mut typography.pitch),
        "--flow-bullet-style" => {
            if typography.bullet_style.is_some() {
                return Err("--flow-bullet-style may appear only once".into());
            }
            typography.bullet_style =
                Some(parse_notebook_flow_bullet_style(iter)?);
            Ok(())
        }
        "--flow-task-style" => {
            if typography.task_style.is_some() {
                return Err("--flow-task-style may appear only once".into());
            }
            typography.task_style =
                Some(parse_notebook_flow_task_style(iter)?);
            Ok(())
        }
        "--flow-bullet-color" | "--flow-task-color"
        | "--flow-quote-color" => {
            let selected = match option {
                "--flow-bullet-color" => &mut typography.bullet_color,
                "--flow-task-color" => &mut typography.task_color,
                _ => &mut typography.quote_color,
            };
            if selected.is_some() {
                return Err(format!("{option} may appear only once"));
            }
            *selected = Some(parse_notebook_flow_marker_color(iter, option)?);
            Ok(())
        }
        "--flow-paragraph-gap-um" =>
            select_notebook_flow_paragraph_gap(
                iter, &mut typography.paragraph_gap,
            ),
        "--flow-body-align" | "--flow-quote-align"
        | "--flow-bullet-align" | "--flow-ordered-align"
        | "--flow-task-align" => {
            let selected = match option {
                "--flow-body-align" => &mut typography.body_align,
                "--flow-quote-align" => &mut typography.quote_align,
                "--flow-bullet-align" => &mut typography.bullet_align,
                "--flow-task-align" => &mut typography.task_align,
                _ => &mut typography.ordered_align,
            };
            if selected.is_some() {
                return Err(format!("{option} may appear only once"));
            }
            *selected = Some(parse_notebook_flow_align(iter, option)?);
            Ok(())
        }
        _ => select_flow_inset(iter, typography, option),
    }
}

fn parse_notebook_flow_region(
    iter: &mut Iter<'_, String>, selected: bool,
) -> Result<FlowRegion, String> {
    if selected {
        return Err("--flow-region may appear only once".into());
    }
    FlowRegion::parse(iter)
}

fn load_notebook_profile_option(
    iter: &mut Iter<'_, String>, loaded: bool, edited: bool,
) -> Result<NotebookProfile, String> {
    if loaded || edited {
        return Err("--profile must appear once, before edits".into());
    }
    let path = iter.next().ok_or("--profile requires a file")?;
    NotebookProfile::from_json(&read_notebook_input(
        path, MAX_NOTEBOOK_JSON_BYTES,
    )?)
}

fn load_notebook_paper_option(
    iter: &mut Iter<'_, String>, loaded: &mut bool,
) -> Result<String, String> {
    if *loaded {
        return Err("--paper-profile may appear only once".into());
    }
    let path = iter.next().ok_or("--paper-profile requires a file")?;
    let paper = read_notebook_input(
        path, atrament_notebook_paper_cli::MAX_PROFILE_BYTES,
    )?;
    let canonical = canonical_paper_profile_json(&paper)?;
    *loaded = true;
    Ok(canonical)
}

fn validate_notebook_flow_flag_compatibility(
    flow_format: Option<FlowFormat>, region_selected: bool,
    typography: FlowTypography, heading: HeadingDecoration,
    export_format: Option<NotebookExportFormat>, inspect_flow: bool,
) -> Result<(), String> {
    // Transient text is not a field in the saved notebook v1 profile.
    if flow_format.is_some() && export_format.is_none() && !inspect_flow {
        return Err(concat!(
            "flow source needs --export-paper pdf or png ",
            "or --emit-ink-file; ",
            "notebook profiles do not save the supplied text",
        ).into());
    }
    if region_selected && flow_format.is_none() {
        return Err("--flow-region requires a text or Markdown flow".into());
    }
    if typography.pitch.is_some() && flow_format.is_none() {
        return Err("--flow-pitch-um requires a text or Markdown flow".into());
    }
    if typography.paragraph_gap.is_some()
        && flow_format != Some(FlowFormat::Markdown) {
        return Err("--flow-paragraph-gap-um requires a Markdown flow".into());
    }
    if typography.body_align.is_some()
        && flow_format != Some(FlowFormat::Markdown) {
        return Err("--flow-body-align requires a Markdown flow".into());
    }
    if typography.quote_align.is_some()
        && flow_format != Some(FlowFormat::Markdown) {
        return Err("--flow-quote-align requires a Markdown flow".into());
    }
    if typography.quote_color.is_some()
        && flow_format != Some(FlowFormat::Markdown) {
        return Err("--flow-quote-color requires a Markdown flow".into());
    }
    if typography.bullet_align.is_some()
        && flow_format != Some(FlowFormat::Markdown) {
        return Err("--flow-bullet-align requires a Markdown flow".into());
    }
    if typography.bullet_style.is_some()
        && flow_format != Some(FlowFormat::Markdown) {
        return Err("--flow-bullet-style requires a Markdown flow".into());
    }
    if typography.bullet_color.is_some()
        && flow_format != Some(FlowFormat::Markdown) {
        return Err("--flow-bullet-color requires a Markdown flow".into());
    }
    if typography.ordered_align.is_some()
        && flow_format != Some(FlowFormat::Markdown) {
        return Err("--flow-ordered-align requires a Markdown flow".into());
    }
    if typography.task_align.is_some()
        && flow_format != Some(FlowFormat::Markdown) {
        return Err("--flow-task-align requires a Markdown flow".into());
    }
    if typography.task_style.is_some()
        && flow_format != Some(FlowFormat::Markdown) {
        return Err("--flow-task-style requires a Markdown flow".into());
    }
    if typography.task_color.is_some()
        && flow_format != Some(FlowFormat::Markdown) {
        return Err("--flow-task-color requires a Markdown flow".into());
    }
    if typography.body_first_line_indent.is_some()
        && flow_format != Some(FlowFormat::Markdown) {
        return Err(
            "--flow-body-first-line-indent-um requires a Markdown flow".into()
        );
    }
    if typography.body_continuation_indent.is_some()
        && flow_format != Some(FlowFormat::Markdown) {
        return Err(
            "--flow-body-continuation-indent-um requires a Markdown flow".into()
        );
    }
    if typography.body_inset.is_some()
        && flow_format != Some(FlowFormat::Markdown) {
        return Err("--flow-body-inset-um requires a Markdown flow".into());
    }
    if typography.body_right_inset.is_some()
        && flow_format != Some(FlowFormat::Markdown) {
        return Err(
            "--flow-body-right-inset-um requires a Markdown flow".into()
        );
    }
    if typography.list_quote_inset.is_some()
        && flow_format != Some(FlowFormat::Markdown) {
        return Err(
            "--flow-list-quote-inset-um requires a Markdown flow".into()
        );
    }
    if typography.list_quote_right_inset.is_some()
        && flow_format != Some(FlowFormat::Markdown) {
        return Err(
            "--flow-list-quote-right-inset-um requires a Markdown flow".into()
        );
    }
    heading.validate().map_err(str::to_owned)?;
    if heading.is_selected() && flow_format != Some(FlowFormat::Markdown) {
        return Err("heading customization requires --flow-markdown".into());
    }
    Ok(())
}

fn validate_notebook_flow_selection(
    flow: Option<&FlowSource>, glyphs: &GlyphOverlay,
    heading: HeadingDecoration, region: Option<FlowRegion>,
    typography: FlowTypography, export: &NotebookExportOptions,
    inspect_flow: bool,
) -> Result<(), String> {
    validate_notebook_flow_flag_compatibility(
        flow.map(|source| source.format), region.is_some(),
        typography, heading, export.format, inspect_flow,
    )?;
    if flow.is_none() {
        glyphs.validate_pairing()?;
    } else if glyphs.strokes.is_empty() && !inspect_flow {
        return Err("flow requires authored glyph shapes".into());
    }
    Ok(())
}

// Reject duplicate selections before parsing would read a prior input source.
fn preflight_unique_selection(
    selected: &mut bool, message: &'static str,
) -> Result<(), String> {
    if *selected {
        return Err(message.into());
    }
    *selected = true;
    Ok(())
}

// The same typed glyph parser must run during source-independent preflight
// and final expansion; its inputs never require reading a selected file.
fn preflight_notebook_glyph_option(
    option: &str, glyphs: &mut GlyphOverlay, iter: &mut Iter<'_, String>,
) -> Result<(), String> {
    match option {
        "--join-text-anchors" => glyphs.add_text_join_anchors(iter),
        "--join-text-pair" => glyphs.add_text_join_pair(iter),
        _ => glyphs.parse(option, iter),
    }
}

// Inline source text needs the same admission as its final flow parser.
// File-backed sources defer their bounded read and content validation.
fn preflight_notebook_flow_option(
    option: &str, iter: &mut Iter<'_, String>,
) -> Result<(FlowFormat, usize, bool), String> {
    let format = if option.starts_with("--flow-text") {
        FlowFormat::Plain
    } else {
        FlowFormat::Markdown
    };
    if option.ends_with("-file") {
        return Ok((format, 1, true));
    }
    let source = iter.next().ok_or(
        "flow needs a UTF-8 text value or a selected file path"
    )?;
    drop(FlowSource::new(format, source)?);
    Ok((format, 0, false))
}

// A second source is source-independent, even when the first is stdin.
fn preflight_notebook_flow_selection(
    option: &str, iter: &mut Iter<'_, String>,
    flow_format: &mut Option<FlowFormat>,
) -> Result<(usize, bool), String> {
    if flow_format.is_some() {
        return Err("only one flow source may be supplied".into());
    }
    let (format, operands, stdin_source) =
        preflight_notebook_flow_option(option, iter)?;
    *flow_format = Some(format);
    Ok((operands, stdin_source))
}

// Match final file-option arity before earlier selected stdin or file I/O.
// A bare flag-shaped token is not an implicit filename: callers may spell
// intentional dash-prefixed filenames as ./--name instead.
fn preflight_notebook_file_operand(
    option: &str, iter: &mut Iter<'_, String>, stdin_source: bool,
    stdin_selected: &mut bool,
) -> Result<(), String> {
    let diagnostic = match option {
        "--glyph-file" => "--glyph-file requires a library path",
        "--flow-text-file" | "--flow-markdown-file" =>
            "flow needs a UTF-8 text value or a selected file path",
        "--ink-file" => "--ink-file requires a file",
        "--profile" => "--profile requires a file",
        "--paper-profile" => "--paper-profile requires a file",
        _ => return Err(format!("unknown notebook source option: {option}")),
    };
    let value = iter.next().filter(|value| !value.is_empty())
        .ok_or(diagnostic)?;
    if value.starts_with('-') && (value != "-" || !stdin_source) {
        return Err(format!(
            "{option} requires an explicit source path (use ./--name)"
        ));
    }
    if stdin_source && value == "-" {
        if *stdin_selected {
            return Err("standard input may be selected only once".into());
        }
        *stdin_selected = true;
    }
    Ok(())
}

// Library geometry must precede inline glyph edits even when the selected
// source would otherwise wait indefinitely on stdin.
fn preflight_glyph_file_selection(
    selected: &mut bool, edited: bool,
) -> Result<(), String> {
    if *selected || edited {
        return Err(concat!(
            "--glyph-file or --profile must precede authored ",
            "strokes and may appear only once",
        ).into());
    }
    *selected = true;
    Ok(())
}

// Reject impossible source selections before the first file or stdin read.
// Count actual operands: Markdown text and glyph values can contain strings
// that look like flags without selecting another input source.
fn preflight_notebook_sources(args: &[String]) -> Result<(), String> {
    let (mut iter, mut stdin_selected) = (args.iter(), false);
    let (mut glyph_file_selected, mut glyph_edited) = (false, false);
    let mut glyph_non_shape_selected = false;
    let (mut flow_format, mut region_selected) = (None, false);
    let (mut region_geometry, mut preview) = (None, NotebookProfile::default());
    let (mut profile_selected, mut profile_edited) = (false, false);
    let (mut paper_selected, mut handwriting_selected) = (false, false);
    let mut flags = NotebookFlagPreflight::default();
    // Reuse the exact, side-effect-free glyph parser before opening any
    // selected library, profile, or flow source. These temporary shapes are
    // only validation evidence; final composition parses its own library.
    let (mut inline_ink, mut glyph_preflight) =
        (Vec::new(), GlyphOverlay::default());
    while let Some(option) = iter.next() {
        flags.imported_overlay_selected |=
            matches!(option.as_str(), "--ink-file" | "--glyph-file");
        let (operands, stdin_source): (usize, bool) = if let Some(count) =
            flags.select_option(option, &mut iter)?
        {
            (count, false)
        } else {
            match option.as_str() {
                "--ink-line" => {
                    preflight_inline_ink_line(&mut iter, &mut inline_ink)?;
                    (0, false)
                }
                "--glyph-word-slant" | "--glyph-word-width" | "--fallback-glyph"
                | "--join-text-anchors" | "--glyph-word-height" | "--place-text"
                | "--glyph-line-drift" | "--glyph-word-drift" | "--glyph-stroke"
                | "--glyph-variation" | "--glyph-word-rise" | "--join-text-pair"
                | "--glyph-word-pressure" | "--glyph-word-pigment"
                | "--glyph-word-tracking" | "--glyph-word-baseline-step" => {
                    glyph_edited = true;
                    glyph_non_shape_selected |= !matches!(option.as_str(),
                        "--glyph-stroke" | "--fallback-glyph");
                    preflight_notebook_glyph_option(
                        option, &mut glyph_preflight, &mut iter)?;
                    (0, false)
                }
                "--flow-region" => {
                    preflight_unique_selection(&mut region_selected,
                        "--flow-region may appear only once")?;
                    region_geometry = Some(FlowRegion::parse(&mut iter)?);
                    (0, false)
                }
                "--glyph-file" => {
                    preflight_glyph_file_selection(
                        &mut glyph_file_selected, glyph_edited,
                    )?;
                    (1, true)
                }
                "--flow-text" | "--flow-markdown"
                | "--flow-text-file" | "--flow-markdown-file" => {
                    preflight_notebook_flow_selection(
                        option, &mut iter, &mut flow_format,
                    )?
                }
                "--ink-file" => (1, true),
                "--profile" => {
                    if profile_selected || profile_edited {
                        return Err("--profile must appear once, before edits"
                            .into());
                    }
                    profile_selected = true;
                    (1, false)
                }
                "--paper-profile" => {
                    preflight_unique_selection(&mut paper_selected,
                        "--paper-profile may appear only once")?;
                    profile_edited = true;
                    (1, false)
                }
                "--date-style" | "--date-placement" | "--date-text"
                | "--border-radius-um" | "--page-numbers"
                | "--printed-sheet" => {
                    preflight_notebook_edit(option, &mut iter, &mut preview)?;
                    profile_edited = true;
                    (0, false)
                }
                "--handwriting-profile" => {
                    preflight_unique_selection(&mut handwriting_selected,
                        "--handwriting-profile may appear only once")?;
                    let _path = parse_notebook_handwriting_path(&mut iter)?;
                    (0, false)
                }
                _ => return Err(format!("unknown notebook option: {option}")),
            }
        };
        if operands != 0 {
            preflight_notebook_file_operand(
                option, &mut iter, stdin_source, &mut stdin_selected,
            )?;
        }
    }
    // Imported-only ink is checked after its bounded source read.
    preflight_notebook_export_and_ink(
        flags, flow_format, region_selected,
        &inline_ink, glyph_file_selected, handwriting_selected,
        &glyph_preflight, glyph_non_shape_selected)?;
    preflight_default_notebook_metadata(&preview, profile_edited,
        profile_selected, paper_selected)?;
    // A caller-selected notebook/paper profile may change the margins.
    // Defer all physical geometry that depends on those source bytes.
    if !profile_selected && !paper_selected {
        // Defer contours that depend on imported shapes, stats or flow.
        preflight_default_paper_geometry(&preview, region_geometry,
            &inline_ink, (!glyph_file_selected && !handwriting_selected
                && flow_format.is_none()).then_some(&glyph_preflight))?;
    }
    Ok(())
}

// Validate known default paper geometry using the same pen clearance rule
// as selected flow rectangles, without rendering or reading earlier sources.
fn preflight_default_paper_geometry(
    preview: &NotebookProfile, region_geometry: Option<FlowRegion>,
    inline_ink: &[NotebookInkLine], authored_glyphs: Option<&GlyphOverlay>,
) -> Result<(), String> {
    if region_geometry.is_none() && inline_ink.is_empty()
        && authored_glyphs.is_none_or(|glyphs| glyphs.runs.is_empty()) {
        return Ok(());
    }
    let printable = preview.printable_rect_um()?;
    if let Some(region) = region_geometry {
        region.validate_ink(printable, &[])?;
    }
    let paper_region = FlowRegion {
        left: printable.0,
        top: printable.1,
        width: printable.2.checked_sub(printable.0)
            .ok_or("paper horizontal bounds reversed")?,
        height: printable.3.checked_sub(printable.1)
            .ok_or("paper vertical bounds reversed")?,
    };
    for line in inline_ink {
        paper_region.validate_ink(printable, from_ref(line))
            .map_err(|_error| format!(
                "page {} ink needs printable endpoints and valid width",
                line.page,
            ))?;
    }
    if let Some(glyphs) = authored_glyphs
        && !glyphs.runs.is_empty() {
        // Reuse the real authored pen compositor rather than estimating
        // a run's bounding box from its origin or nominal advance.
        for line in glyphs.expand(&[], None)? {
            paper_region.validate_ink(printable, from_ref(&line))
                .map_err(|_error| format!(
                    "page {} ink needs printable endpoints and valid width",
                    line.page,
                ))?;
        }
    }
    Ok(())
}

// A selected notebook may change date fields, but a selected paper only
// changes geometry. Refuse known contradictory date edits before source IO;
// defer paper-dependent clearance until its selected profile is loaded.
fn preflight_default_notebook_metadata(
    preview: &NotebookProfile, edited: bool, profile_selected: bool,
    paper_selected: bool,
) -> Result<(), String> {
    if edited && !profile_selected {
        if paper_selected {
            preview.validate_date_metadata()?;
        } else {
            preview.validate()?;
        }
    }
    Ok(())
}

// All source-independent output and physical-ink gates precede selected I/O.
fn preflight_notebook_export_and_ink(
    flags: NotebookFlagPreflight, flow_format: Option<FlowFormat>,
    region_selected: bool, inline_ink: &[NotebookInkLine],
    glyph_file_selected: bool, handwriting_selected: bool,
    glyphs: &GlyphOverlay, glyph_non_shape_selected: bool,
) -> Result<(), String> {
    glyphs.validate_optional_rhythm()?;
    let manual_last_page = inline_ink.iter().map(|line| line.page)
        .chain(glyphs.runs.iter().map(|run| run.page))
        .max().unwrap_or(0);
    let inspect_flow = flags.selected.contains("--inspect-flow");
    flags.validate(flow_format, region_selected, inline_ink.len(),
        manual_last_page, glyphs.has_any(),
        glyph_non_shape_selected)?;
    if flow_format.is_some() && !glyphs.fallbacks.is_empty() {
        glyphs.validate_fallbacks(glyph_file_selected)?;
    }
    if inspect_flow {
        return Ok(());
    }
    preflight_notebook_ink_requirements(
        flow_format, glyph_file_selected, handwriting_selected, glyphs,
    )
}

// Check impossible physical ink requirements before any selected source is
// opened. Imported glyph libraries may supply strokes but neither saved page
// ink nor handwriting statistics can supply missing placed text.
fn preflight_notebook_ink_requirements(
    flow_format: Option<FlowFormat>, glyph_file_selected: bool,
    handwriting_selected: bool, glyphs: &GlyphOverlay,
) -> Result<(), String> {
    if flow_format.is_some() && !glyph_file_selected
        && glyphs.strokes.is_empty()
    {
        return Err("flow requires authored glyph shapes".into());
    }
    if handwriting_selected && (
        (!glyph_file_selected && glyphs.strokes.is_empty())
            || (flow_format.is_none() && glyphs.runs.is_empty())
    ) {
        return Err(concat!(
            "--handwriting-profile requires both --glyph-stroke ",
            "and --place-text",
        ).into());
    }
    // Without a glyph library or flow, the complete authored ink is already
    // known: page-ink files cannot supply text or exact glyph outlines.
    if flow_format.is_none() && !glyph_file_selected {
        glyphs.validate_pairing()?;
    }
    // A selected library can supply glyph strokes, but not placed text.
    // Preserve its deferred shape checks while rejecting impossible edit
    // requests before an earlier source might wait on stdin.
    if flow_format.is_none() && glyph_file_selected && glyphs.runs.is_empty() {
        if glyphs.variation.is_some() {
            return Err("--glyph-variation requires placed text".into());
        }
        if !glyphs.text_joins.is_empty() {
            return Err("--join-text-pair requires placed text".into());
        }
        if !glyphs.fallbacks.is_empty() {
            return Err(concat!(
                "--fallback-glyph requires authored glyphs ",
                "and placed text",
            ).into());
        }
        return Err(concat!(
            "--glyph-stroke and --place-text require one another; ",
            "all visible graphemes need an authored shape",
        ).into());
    }
    Ok(())
}

// Keep the CLI selection vocabulary in one place for typed option
// dispatch; source-independent preflight validates the same flags first.
fn is_notebook_flow_typography_option(option: &str) -> bool {
    matches!(option,
        "--flow-pitch-um" | "--flow-body-align" | "--flow-quote-align"
        | "--flow-quote-color"
        | "--flow-bullet-align" | "--flow-bullet-style"
        | "--flow-bullet-color" | "--flow-task-color"
        | "--flow-ordered-align" | "--flow-task-align" | "--flow-task-style"
        | "--flow-body-first-line-indent-um" | "--flow-paragraph-gap-um"
        | "--flow-body-inset-um" | "--flow-body-continuation-indent-um"
        | "--flow-body-right-inset-um" | "--flow-list-quote-inset-um"
        | "--flow-list-quote-right-inset-um")
}

fn parse_notebook_options(args: &[String]) -> Result<NotebookRequest, String> {
    preflight_notebook_sources(args)?;
    let mut profile = NotebookProfile::default();
    let (mut inspect, mut output, mut replace) = (false, None, false);
    let (mut inspect_flow, mut inspect_flow_inventory) = (false, false);
    let mut inspect_flow_page_inventory = false;
    let mut inspect_flow_shape_coverage = false;
    let mut inspect_flow_stroke_demand = false;
    let mut inspect_flow_stroke_breakdown = false;
    let mut inspect_flow_ink_validation = false;
    let mut inspect_flow_ink_bounds = false;
    let mut inspect_flow_ink_palette = false;
    let mut inspect_flow_ink_widths = false;
    let mut inspect_flow_ink_length = false;
    let mut inspect_flow_ink_length_by_color = false;
    let mut inspect_flow_ink_gaps = false;
    let mut inspect_flow_ink_gap_details = false;
    let (mut export, mut ink) = (NotebookExportOptions::default(), Vec::new());
    let (mut glyphs, mut statistics) = (GlyphOverlay::default(), None);
    let (mut flow, mut flow_region) = (None, None);
    let mut flow_typography = FlowTypography::default();
    let (mut heading, mut stdin_used) = (HeadingDecoration::default(), false);
    let (mut loaded, mut paper_loaded, mut edited) = (false, false, false);
    let mut iter = args.iter();
    while let Some(option) = iter.next() {
        match option.as_str() {
            "--profile" => {
                profile = load_notebook_profile_option(
                    &mut iter, loaded, edited)?;
                loaded = true;
            }
            "--paper-profile" => {
                profile.paper_json = load_notebook_paper_option(
                    &mut iter, &mut paper_loaded)?;
                edited = true;
            }
            "--date-style" | "--date-placement" | "--date-text"
            | "--border-radius-um" | "--page-numbers"
            | "--printed-sheet" => {
                let value = iter.next()
                    .ok_or_else(|| format!("{option} requires a value"))?;
                apply_notebook_edit(&mut profile, option, value)?;
                edited = true;
            }
            "--inspect" if inspect =>
                return Err("--inspect may appear only once".into()),
            "--inspect" => inspect = true,
            "--inspect-flow" if inspect_flow =>
                return Err("--inspect-flow may appear only once".into()),
            "--inspect-flow" => inspect_flow = true,
            "--inspect-flow-inventory" if inspect_flow_inventory =>
                return Err(
                    "--inspect-flow-inventory may appear only once".into()
                ),
            "--inspect-flow-inventory" => inspect_flow_inventory = true,
            "--inspect-flow-page-inventory" if inspect_flow_page_inventory =>
                return Err(
                    "--inspect-flow-page-inventory may appear only once".into()
                ),
            "--inspect-flow-page-inventory" =>
                inspect_flow_page_inventory = true,
            "--inspect-flow-shape-coverage" if inspect_flow_shape_coverage =>
                return Err(
                    "--inspect-flow-shape-coverage may appear only once".into()
                ),
            "--inspect-flow-shape-coverage" =>
                inspect_flow_shape_coverage = true,
            "--inspect-flow-stroke-demand" if inspect_flow_stroke_demand =>
                return Err(
                    "--inspect-flow-stroke-demand may appear only once".into()
                ),
            "--inspect-flow-stroke-demand" =>
                inspect_flow_stroke_demand = true,
            "--inspect-flow-stroke-breakdown"
                if inspect_flow_stroke_breakdown => return Err(
                    "--inspect-flow-stroke-breakdown may appear only once"
                        .into()
                ),
            "--inspect-flow-stroke-breakdown" =>
                inspect_flow_stroke_breakdown = true,
            "--inspect-flow-ink-validation" if inspect_flow_ink_validation =>
                return Err(
                    "--inspect-flow-ink-validation may appear only once".into()
                ),
            "--inspect-flow-ink-validation" =>
                inspect_flow_ink_validation = true,
            "--inspect-flow-ink-bounds" if inspect_flow_ink_bounds =>
                return Err(
                    "--inspect-flow-ink-bounds may appear only once".into()
                ),
            "--inspect-flow-ink-bounds" => inspect_flow_ink_bounds = true,
            "--inspect-flow-ink-palette" if inspect_flow_ink_palette =>
                return Err(
                    "--inspect-flow-ink-palette may appear only once".into()
                ),
            "--inspect-flow-ink-palette" => inspect_flow_ink_palette = true,
            "--inspect-flow-ink-widths" if inspect_flow_ink_widths =>
                return Err(
                    "--inspect-flow-ink-widths may appear only once".into()
                ),
            "--inspect-flow-ink-widths" => inspect_flow_ink_widths = true,
            "--inspect-flow-ink-length" if inspect_flow_ink_length =>
                return Err(
                    "--inspect-flow-ink-length may appear only once".into()
                ),
            "--inspect-flow-ink-length" => inspect_flow_ink_length = true,
            "--inspect-flow-ink-length-by-color"
                if inspect_flow_ink_length_by_color => return Err(
                    "--inspect-flow-ink-length-by-color may appear only once"
                        .into()
                ),
            "--inspect-flow-ink-length-by-color" =>
                inspect_flow_ink_length_by_color = true,
            "--inspect-flow-ink-gaps" if inspect_flow_ink_gaps =>
                return Err(
                    "--inspect-flow-ink-gaps may appear only once".into()
                ),
            "--inspect-flow-ink-gaps" => inspect_flow_ink_gaps = true,
            "--inspect-flow-ink-gap-details"
                if inspect_flow_ink_gap_details => return Err(
                    "--inspect-flow-ink-gap-details may appear only once"
                        .into()
                ),
            "--inspect-flow-ink-gap-details" =>
                inspect_flow_ink_gap_details = true,
            "--handwriting-profile" =>
                parse_notebook_handwriting_profile(&mut iter, &mut statistics)?,
            "--heading-style" | "--heading-align" | "--heading-color"
            | "--heading-ink-color" | "--heading-ink-scale"
            | "--heading-marker-scale" => heading.parse(option, &mut iter)?,
            "--flow-region" => flow_region = Some(parse_notebook_flow_region(
                &mut iter, flow_region.is_some())?),
            selected if is_notebook_flow_typography_option(selected) =>
                select_notebook_flow_typography(
                    &mut iter, &mut flow_typography, selected,
                )?,
            "--flow-text" | "--flow-markdown"
            | "--flow-text-file" | "--flow-markdown-file" =>
                parse_notebook_flow(&mut iter, option, &mut flow,
                    &mut stdin_used)?,
            "--emit-ink-file" | "--export-paper" | "--pages" | "--dpi"
            | "--ruler-seed" | "--ruler-error-mm"
            | "--png-page" | "--ink-line" | "--ink-file" | "--join-text-anchors"
            | "--glyph-stroke" | "--glyph-file" | "--place-text" | "--quality"
            | "--fallback-glyph" | "--glyph-variation" | "--glyph-word-rise"
            | "--glyph-line-drift" | "--glyph-word-drift" | "--join-text-pair"
            | "--glyph-word-pressure" | "--glyph-word-pigment"
            | "--glyph-word-slant" | "--glyph-word-width"
            | "--glyph-word-height" | "--glyph-word-tracking"
            | "--glyph-word-baseline-step" => {
                parse_notebook_export_option(option, &mut iter,
                    &mut export, &mut ink, &mut glyphs, &mut stdin_used)?;
            }
            "--output" => output = Some(parse_selected_output(
                &mut iter, output.is_some())?),
            "--replace" => preflight_unique_selection(
                &mut replace, "--replace may appear only once")?,
            _ => return Err(format!("unknown notebook option: {option}")),
        }
    }
    validate_notebook_flow_selection(
        flow.as_ref(), &glyphs, heading, flow_region, flow_typography,
        &export, inspect_flow,
    )?;
    validate_notebook_export_flags(
        replace, output.as_deref(), inspect, inspect_flow,
        inspect_flow_shape_coverage, &export, &ink,
        &glyphs, statistics.as_ref(),
    )?;
    profile.validate()?;
    let ruler_variation = resolve_notebook_ruler(
        export.ruler_seed.as_deref(), export.ruler_error_mm.as_deref(),
    )?;
    Ok(NotebookRequest {
        flow,
        flow_typography,
        flow_region,
        pages_explicit: export.pages.is_some(),
        glyphs,
        heading,
        dpi: export.dpi.or(export.quality_dpi).unwrap_or(150),
        export_format: export.format,
        ink,
        inspect,
        inspect_flow,
        inspect_flow_inventory,
        inspect_flow_page_inventory,
        inspect_flow_shape_coverage,
        inspect_flow_stroke_demand,
        inspect_flow_stroke_breakdown,
        inspect_flow_ink_validation,
        inspect_flow_ink_bounds,
        inspect_flow_ink_palette,
        inspect_flow_ink_widths,
        inspect_flow_ink_length,
        inspect_flow_ink_length_by_color,
        inspect_flow_ink_gaps,
        inspect_flow_ink_gap_details,
        output,
        pages: export.pages.unwrap_or(1),
        png_page: export.png_page.unwrap_or(1),
        profile,
        replace,
        ruler_variation,
        statistics,
    })
}

// Source-independent manual annotations contribute to PDF page count.
fn manual_last_page(request: &NotebookRequest) -> usize {
    request
        .ink
        .iter()
        .map(|line| line.page)
        .chain(request.glyphs.runs.iter().map(|run| run.page))
        .max()
        .unwrap_or(0)
}

// A known manual page cannot be hidden by a smaller explicit PDF budget,
// even when an earlier selected source has not yet been read.
fn validate_explicit_manual_page_budget(
    selected: Option<usize>, last_page: usize, flow_selected: bool,
) -> Result<(), String> {
    if let Some(pages) = selected && pages < last_page {
        let source = if flow_selected { "flow and manual ink" }
            else { "manual ink" };
        let verb = if flow_selected { "require" } else { "requires" };
        return Err(format!(
            "{source} {verb} {last_page} physical pages, \
             more than the explicit --pages count"
        ));
    }
    Ok(())
}

// Standalone manual ink also selects its last written PDF page by default.
// An explicit --pages remains a hard caller-selected upper bound.
fn admit_manual_page_selection(
    request: &mut NotebookRequest,
) -> Result<(), String> {
    let last_page = manual_last_page(request);
    validate_explicit_manual_page_budget(
        request.pages_explicit.then_some(request.pages), last_page, false,
    )?;
    request.pages = request.pages.max(last_page);
    Ok(())
}

// Flow and explicit manual annotations share physical PDF page accounting.
// A PNG page past flowed text is selectable only with actual authored ink.
fn admit_flow_page_selection(
    request: &mut NotebookRequest,
    flow_pages: usize,
) -> Result<(), String> {
    let manual_last_page = manual_last_page(request);
    let required_pages = flow_pages.max(manual_last_page);
    if request.pages_explicit && request.pages < required_pages {
        return Err(if manual_last_page > flow_pages {
            format!(
                concat!(
                    "flow and manual ink require {} physical pages, ",
                    "more than the explicit --pages count",
                ),
                required_pages
            )
        } else {
            format!(
                concat!(
                    "flow requires {} physical pages, ",
                    "more than the explicit --pages count",
                ),
                required_pages
            )
        });
    }
    if request.export_format == Some(NotebookExportFormat::Png)
        && request.png_page > flow_pages
        && !request.ink.iter().any(|line| line.page == request.png_page)
        && !request.glyphs.runs.iter().any(|run| run.page == request.png_page)
    {
        return Err(format!(
            "flow has {flow_pages} pages; selected PNG page is empty"
        ));
    }
    request.pages = request.pages.max(required_pages);
    Ok(())
}

// Match exact caller-authored stroke keys only after the same nominal flow
// has placed all visible graphemes. Do not interpret membership as rendered
// pen clearance, implicit fallbacks or persistent semantic content.
#[expect(clippy::use_debug, reason =
    "preserve escaped exact Unicode grapheme-key report spelling")]
fn append_flow_shape_coverage(
    report: &mut String, glyphs: &GlyphOverlay,
    inventory: &BTreeMap<String, usize>,
    page_inventories: &BTreeMap<usize, BTreeMap<String, usize>>,
    pages: usize, show_page_detail: bool,
) -> Result<(), String> {
    let shapes = glyphs.exact_authored_shapes()?;
    let status = |grapheme: &str| {
        if shapes.contains(grapheme) {
            ShapeCoverage::Authored
        } else if let Some(mapping) = glyphs.fallbacks.iter()
            .find(|mapping| mapping.missing == grapheme) {
            ShapeCoverage::Borrowed(mapping.authored.as_str())
        } else {
            ShapeCoverage::Missing
        }
    };
    let present_forms = inventory.keys()
        .filter(|key| status(key).is_present()).count();
    let present_uses: usize = inventory.iter()
        .filter(|(key, _)| status(key).is_present())
        .map(|(_, occurrences)| *occurrences).sum();
    let total_uses: usize = inventory.values().sum();
    let description = if glyphs.fallbacks.is_empty() {
        "Exact"
    } else {
        "Selected"
    };
    writeln!(report,
        "{description} shape coverage: {present_forms}/{} forms, \
         {present_uses}/{total_uses} uses (ink clearance unchecked)",
        inventory.len(),
    ).map_err(|error| format!("flow coverage refused: {error}"))?;
    if !glyphs.fallbacks.is_empty() {
        let borrowed_forms = inventory.keys()
            .filter(|key| matches!(status(key), ShapeCoverage::Borrowed(_)))
            .count();
        let borrowed_uses: usize = inventory.iter()
            .filter(|(key, _)| matches!(
                status(key), ShapeCoverage::Borrowed(_),
            ))
            .map(|(_, occurrences)| *occurrences).sum();
        writeln!(report,
            "Borrowed shape coverage: {borrowed_forms} forms, \
             {borrowed_uses} uses",
        ).map_err(|error| format!("flow coverage refused: {error}"))?;
    }
    for (grapheme, count) in inventory {
        match status(grapheme) {
            ShapeCoverage::Authored =>
                writeln!(report, "Authored {grapheme:?}: {count}"),
            ShapeCoverage::Borrowed(authored) =>
                writeln!(report,
                    "Borrowed {grapheme:?} from {authored:?}: {count}"),
            ShapeCoverage::Missing =>
                writeln!(report, "Missing {grapheme:?}: {count}"),
        }.map_err(|error| format!("flow coverage refused: {error}"))?;
    }
    if show_page_detail {
        for page in 1..=pages {
            let Some(forms) = page_inventories.get(&page) else {
                writeln!(report,
                    "Page {page} shape coverage: 0/0 forms, 0/0 uses",
                ).map_err(|error| {
                    format!("flow coverage refused: {error}")
                })?;
                continue;
            };
            let count = forms.keys()
                .filter(|key| status(key).is_present()).count();
            let covered: usize = forms.iter()
                .filter(|(key, _)| status(key).is_present())
                .map(|(_, occurrences)| *occurrences).sum();
            let page_uses: usize = forms.values().sum();
            writeln!(report,
                "Page {page} shape coverage: {count}/{} forms, \
                 {covered}/{page_uses} uses",
                forms.len(),
            ).map_err(|error| format!("flow coverage refused: {error}"))?;
            for (grapheme, occurrences) in forms {
                let line = match status(grapheme) {
                    ShapeCoverage::Borrowed(authored) => Some(format!(
                        concat!("  Borrowed {:?} from ", "{:?}: {}"),
                        grapheme, authored, occurrences,
                    )),
                    ShapeCoverage::Missing => Some(format!(
                        "  Missing {grapheme:?}: {occurrences}"
                    )),
                    ShapeCoverage::Authored => None,
                };
                if let Some(line) = line {
                    writeln!(report, "{line}").map_err(|error| {
                        format!("flow coverage refused: {error}")
                    })?;
                }
            }
        }
    }
    Ok(())
}

// Demand is a lower bound until every visible form is resolved. This counts
// selected authored segments and actual Markdown ornaments, not projected ink
// clearance, optional text joins or any persistent semantic notebook state.
fn append_flow_stroke_demand(
    report: &mut String, glyphs: &GlyphOverlay,
    inventory: &BTreeMap<String, usize>,
    page_inventories: &BTreeMap<usize, BTreeMap<String, usize>>,
    ornaments: &[NotebookInkLine], pages: usize, show_page_detail: bool,
) -> Result<(), String> {
    let mut authored = BTreeMap::<&str, usize>::new();
    for stroke in &glyphs.strokes {
        let count = authored.entry(stroke.grapheme.as_str()).or_default();
        *count = count.checked_add(1)
            .ok_or("flow stroke demand overflow")?;
    }
    let demand = |forms: &BTreeMap<String, usize>|
        -> Result<(usize, usize), String> {
        let mut segments = 0usize;
        let mut missing_uses = 0usize;
        for (form, uses) in forms {
            let selected = glyphs.fallbacks.iter()
                .find(|mapping| mapping.missing == *form)
                .map_or(form.as_str(), |mapping| mapping.authored.as_str());
            if let Some(strokes) = authored.get(selected) {
                segments = segments.checked_add(
                    strokes.checked_mul(*uses)
                        .ok_or("flow stroke demand overflow")?
                ).ok_or("flow stroke demand overflow")?;
            } else {
                missing_uses = missing_uses.checked_add(*uses)
                    .ok_or("flow stroke demand overflow")?;
            }
        }
        Ok((segments, missing_uses))
    };
    let describe = |out: &mut String, label: &str,
                    forms: &BTreeMap<String, usize>, ornament_count: usize|
                    -> Result<(), String> {
        let (authored_segments, unresolved) = demand(forms)?;
        let total = authored_segments.checked_add(ornament_count)
            .ok_or("flow stroke demand overflow")?;
        // Every unresolved visible grapheme needs at least one authored
        // segment. Even without a library for it, that lower bound can prove
        // that no complete shape selection will fit the native export cap.
        let minimum = total.checked_add(unresolved)
            .ok_or("flow stroke demand overflow")?;
        let budget = if total > MAX_NOTEBOOK_INK_LINES {
            "over budget"
        } else if minimum > MAX_NOTEBOOK_INK_LINES {
            "minimum over budget"
        } else if unresolved > 0 {
            "incomplete"
        } else {
            "within budget"
        };
        writeln!(out,
            "{label}: {authored_segments} glyph + {ornament_count} ornament = \
             {total} known segments, {unresolved} unresolved glyph uses \
             ({budget}; native limit {MAX_NOTEBOOK_INK_LINES})",
        ).map_err(|error| format!("flow stroke demand refused: {error}"))
    };
    describe(report, "Nominal stroke demand", inventory, ornaments.len())?;
    if show_page_detail {
        for page in 1..=pages {
            let empty = BTreeMap::new();
            let page_forms = page_inventories.get(&page).unwrap_or(&empty);
            let page_ornaments = ornaments.iter()
                .filter(|ornament| ornament.page == page).count();
            describe(report, &format!("Page {page} stroke demand"),
                page_forms, page_ornaments)?;
        }
    }
    Ok(())
}

// Attribute the nominal stroke count to the exact visible grapheme, not to
// the borrowed source glyph. This stays read-only and preserves the previous
// report bytes whenever the caller does not opt into the breakdown.
#[expect(clippy::use_debug, reason =
    "preserve escaped exact Unicode grapheme-key report spelling")]
fn append_flow_stroke_breakdown(
    report: &mut String, glyphs: &GlyphOverlay,
    inventory: &BTreeMap<String, usize>,
    page_inventories: &BTreeMap<usize, BTreeMap<String, usize>>,
    pages: usize, show_page_detail: bool,
) -> Result<(), String> {
    let mut strokes = BTreeMap::<&str, usize>::new();
    for stroke in &glyphs.strokes {
        let count = strokes.entry(stroke.grapheme.as_str()).or_default();
        *count = count.checked_add(1)
            .ok_or("flow stroke breakdown overflow")?;
    }
    let describe = |out: &mut String, title: &str,
                    forms: &BTreeMap<String, usize>|
                    -> Result<(), String> {
        let uses = forms.values().try_fold(0usize, |sum, count| {
            sum.checked_add(*count)
                .ok_or("flow stroke breakdown overflow")
        })?;
        let form_noun = if forms.len() == 1 { "form" } else { "forms" };
        let use_noun = if uses == 1 { "use" } else { "uses" };
        writeln!(out, "{title}: {} {form_noun}, {uses} {use_noun}",
            forms.len())
            .map_err(|error| {
                format!("flow stroke breakdown refused: {error}")
            })?;
        for (form, count) in forms {
            let target = glyphs.fallbacks.iter()
                .find(|mapping| mapping.missing == *form)
                .map(|mapping| mapping.authored.as_str());
            let selected = target.unwrap_or(form.as_str());
            let use_noun = if *count == 1 { "use" } else { "uses" };
            let line = match strokes.get(selected) {
                Some(per_use) => {
                    let segments = per_use.checked_mul(*count)
                        .ok_or("flow stroke breakdown overflow")?;
                    let stroke_noun = if *per_use == 1 {
                        "stroke"
                    } else { "strokes" };
                    let segment_noun = if segments == 1 {
                        "segment"
                    } else { "segments" };
                    let label = if let Some(source) = target {
                        format!("Borrowed {form:?} from {source:?}")
                    } else {
                        format!("Authored {form:?}")
                    };
                    format!(
                        "  {label}: {count} {use_noun} x {per_use} \
                         {stroke_noun} = {segments} {segment_noun}"
                    )
                }
                None => {
                    let noun = if *count == 1 { "segment" }
                        else { "segments" };
                    format!(
                        "  Missing {form:?}: {count} {use_noun} \
                         (at least {count} {noun})"
                    )
                }
            };
            writeln!(out, "{line}")
                .map_err(|error| {
                    format!("flow stroke breakdown refused: {error}")
                })?;
        }
        Ok(())
    };
    describe(report, "Stroke demand by form", inventory)?;
    if show_page_detail {
        for page in 1..=pages {
            let empty = BTreeMap::new();
            let forms = page_inventories.get(&page).unwrap_or(&empty);
            describe(report, &format!("Page {page} stroke demand by form"),
                forms)?;
        }
    }
    Ok(())
}

// Every page receives the conservative pen-inclusive bounding rectangle of
// all physical strokes. This deliberately includes half the pen width on
// both sides, as rounded upward by the actual native PDF clearance rule.
fn append_flow_ink_bounds(
    report: &mut String, ink: &[NotebookInkLine], pages: usize,
) -> Result<(), String> {
    let mut page_bounds = vec![None::<(u32, u32, u32, u32)>; pages];
    for line in ink {
        let radius = line.width_um.div_ceil(2);
        let (x1, y1) = line.start_um;
        let (x2, y2) = line.end_um;
        let left = x1.min(x2).checked_sub(radius)
            .ok_or("validated ink horizontal bounds underflow")?;
        let top = y1.min(y2).checked_sub(radius)
            .ok_or("validated ink vertical bounds underflow")?;
        let right = x1.max(x2).checked_add(radius)
            .ok_or("validated ink horizontal bounds overflow")?;
        let bottom = y1.max(y2).checked_add(radius)
            .ok_or("validated ink vertical bounds overflow")?;
        let bounds = line.page.checked_sub(1)
            .and_then(|index| page_bounds.get_mut(index))
            .ok_or("validated ink references nonexistent physical page")?;
        *bounds = Some(match *bounds {
            Some((prior_left, prior_top, prior_right, prior_bottom)) => (
                prior_left.min(left), prior_top.min(top),
                prior_right.max(right), prior_bottom.max(bottom),
            ),
            None => (left, top, right, bottom),
        });
    }
    for (index, bounds) in page_bounds.iter().enumerate() {
        if let Some((left, top, right, bottom)) = bounds {
            writeln!(report,
                "Page {} validated ink bounds (um): left={left} \
                 top={top} right={right} bottom={bottom}",
                index + 1,
            )
        } else {
            writeln!(report,
                "Page {} validated ink bounds: empty", index + 1,
            )
        }.map_err(|error| {
            format!("flow ink bounds report refused: {error}")
        })?;
    }
    Ok(())
}

// Inventory actual native pen RGB after all glyph and ornament selection.
// Paper ruling, printed annotations and absent authored shapes are excluded.
fn append_flow_ink_palette(
    report: &mut String, ink: &[NotebookInkLine], pages: usize,
) -> Result<(), String> {
    let mut global = BTreeMap::<[u8; 3], usize>::new();
    let mut per_page = vec![BTreeMap::<[u8; 3], usize>::new(); pages];
    for line in ink {
        let local = line.page.checked_sub(1)
            .and_then(|index| per_page.get_mut(index))
            .ok_or("validated ink references nonexistent physical page")?;
        for palette in [&mut *local, &mut global] {
            let count = palette.entry(line.rgb).or_default();
            *count = count.checked_add(1)
                .ok_or("validated ink palette count overflow")?;
        }
    }
    let describe = |out: &mut String, title: &str,
                    palette: &BTreeMap<[u8; 3], usize>|
                    -> Result<(), String> {
        if palette.is_empty() {
            writeln!(out, "{title}: empty")
                .map_err(|error| {
                    format!("flow ink palette report refused: {error}")
                })?;
        } else {
            let color_noun = if palette.len() == 1 { "color" }
                else { "colors" };
            writeln!(out,
                "{title}: {} RGB {color_noun}", palette.len(),
            ).map_err(|error| {
                format!("flow ink palette report refused: {error}")
            })?;
            for (rgb, count) in palette {
                let segment_noun = if *count == 1 { "segment" }
                    else { "segments" };
                writeln!(out,
                    "  #{:02X}{:02X}{:02X}: {count} {segment_noun}",
                    rgb[0], rgb[1], rgb[2],
                ).map_err(|error| {
                    format!("flow ink palette report refused: {error}")
                })?;
            }
        }
        Ok(())
    };
    describe(report, "Validated ink palette", &global)?;
    for (index, palette) in per_page.iter().enumerate() {
        describe(report, &format!("Page {} validated ink palette", index + 1),
            palette)?;
    }
    Ok(())
}

// Inspect actual output pen thickness instead of nominal glyph statistics.
// This captures heading and ornament widths separately from authored ink.
fn append_flow_ink_widths(
    report: &mut String, ink: &[NotebookInkLine], pages: usize,
) -> Result<(), String> {
    let mut global = BTreeMap::<u32, usize>::new();
    let mut per_page = vec![BTreeMap::<u32, usize>::new(); pages];
    for line in ink {
        let local = line.page.checked_sub(1)
            .and_then(|index| per_page.get_mut(index))
            .ok_or("validated ink references nonexistent physical page")?;
        for counts in [&mut *local, &mut global] {
            let count = counts.entry(line.width_um).or_default();
            *count = count.checked_add(1)
                .ok_or("validated ink width count overflow")?;
        }
    }
    let describe = |out: &mut String, title: &str,
                    widths: &BTreeMap<u32, usize>|
                    -> Result<(), String> {
        if widths.is_empty() {
            writeln!(out, "{title}: empty")
                .map_err(|error| {
                    format!("flow ink widths report refused: {error}")
                })?;
        } else {
            let noun = if widths.len() == 1 { "width" }
                else { "widths" };
            writeln!(out,
                "{title}: {} physical pen {noun}", widths.len(),
            ).map_err(|error| {
                format!("flow ink widths report refused: {error}")
            })?;
            for (width, count) in widths {
                let noun = if *count == 1 { "segment" } else { "segments" };
                writeln!(out, "  {width} um: {count} {noun}")
                    .map_err(|error| {
                        format!("flow ink widths report refused: {error}")
                    })?;
            }
        }
        Ok(())
    };
    describe(report, "Validated ink widths", &global)?;
    for (index, widths) in per_page.iter().enumerate() {
        describe(report, &format!("Page {} validated ink widths", index + 1),
            widths)?;
    }
    Ok(())
}

// Sum the centerline distances of already validated physical segments.
// Round each square root to the nearest micrometre using integers only:
// neither floating-point drift nor a pen-up travel assumption is admitted.
fn rounded_coordinate_length_um(
    start: (u32, u32), end: (u32, u32),
) -> Result<u128, &'static str> {
    let dx = u128::from(start.0.abs_diff(end.0));
    let dy = u128::from(start.1.abs_diff(end.1));
    let squared = dx.checked_mul(dx)
        .and_then(|x| dy.checked_mul(dy)
            .and_then(|y| x.checked_add(y)))
        .ok_or("validated ink squared distance overflow")?;
    let floor = squared.isqrt();
    let upper = floor.checked_add(1)
        .ok_or("validated ink rounded distance overflow")?;
    let lower_sq = floor.checked_mul(floor)
        .ok_or("validated ink floor square overflow")?;
    let upper_sq = upper.checked_mul(upper)
        .ok_or("validated ink ceiling square overflow")?;
    let lower_error = squared.checked_sub(lower_sq)
        .ok_or("validated ink floor distance underflow")?;
    let upper_error = upper_sq.checked_sub(squared)
        .ok_or("validated ink ceiling distance underflow")?;
    Ok(if lower_error >= upper_error { upper } else { floor })
}

fn rounded_ink_segment_length_um(
    line: &NotebookInkLine,
) -> Result<u128, &'static str> {
    rounded_coordinate_length_um(line.start_um, line.end_um)
}

fn append_flow_ink_length(
    report: &mut String, ink: &[NotebookInkLine], pages: usize,
) -> Result<(), String> {
    let mut lengths = vec![None::<u128>; pages];
    let mut global = 0u128;
    for line in ink {
        let local = line.page.checked_sub(1)
            .and_then(|index| lengths.get_mut(index))
            .ok_or("validated ink references nonexistent physical page")?;
        let segment_length = rounded_ink_segment_length_um(line)?;
        *local = Some(local.unwrap_or(0).checked_add(segment_length)
            .ok_or("validated ink length overflow")?);
        global = global.checked_add(segment_length)
            .ok_or("validated ink total length overflow")?;
    }
    writeln!(report, "Validated ink pen-down length: {global} um")
        .map_err(|error| format!("flow ink length report refused: {error}"))?;
    for (index, page_length) in lengths.iter().enumerate() {
        let page = index.checked_add(1)
            .ok_or("validated ink page number overflow")?;
        match page_length {
            Some(length) => writeln!(report,
                "Page {page} validated ink pen-down length: {length} um"),
            None => writeln!(report,
                "Page {page} validated ink pen-down length: empty"),
        }.map_err(|error| {
            format!("flow ink length report refused: {error}")
        })?;
    }
    Ok(())
}

// Color-grouped drawn distances retain physical ink provenance, including
// colored Markdown ornaments, without predicting pen-up or material use.
fn append_flow_ink_length_by_color(
    report: &mut String, ink: &[NotebookInkLine], pages: usize,
) -> Result<(), String> {
    let mut global = BTreeMap::<[u8; 3], (u128, usize)>::new();
    let mut per_page = vec![BTreeMap::<[u8; 3], (u128, usize)>::new(); pages];
    for line in ink {
        let page_colors = line.page.checked_sub(1)
            .and_then(|index| per_page.get_mut(index))
            .ok_or("validated ink references nonexistent physical page")?;
        let distance = rounded_ink_segment_length_um(line)?;
        for colors in [page_colors, &mut global] {
            let totals = colors.entry(line.rgb).or_default();
            totals.0 = totals.0.checked_add(distance)
                .ok_or("validated ink color distance overflow")?;
            totals.1 = totals.1.checked_add(1)
                .ok_or("validated ink color segment count overflow")?;
        }
    }
    let describe = |out: &mut String, title: &str,
                    colors: &BTreeMap<[u8; 3], (u128, usize)>|
                    -> Result<(), String> {
        if colors.is_empty() {
            writeln!(out, "{title}: empty")
                .map_err(|error| format!(
                    "flow ink color length report refused: {error}"))?;
        } else {
            let noun = if colors.len() == 1 { "color" } else { "colors" };
            writeln!(out, "{title}: {} RGB {noun}", colors.len())
                .map_err(|error| format!(
                    "flow ink color length report refused: {error}"))?;
            for (rgb, (distance, segments)) in colors {
                let segment_noun = if *segments == 1 { "segment" }
                    else { "segments" };
                writeln!(out,
                    "  #{:02X}{:02X}{:02X}: {distance} um \
                     across {segments} {segment_noun}",
                    rgb[0], rgb[1], rgb[2],
                ).map_err(|error| format!(
                    "flow ink color length report refused: {error}"))?;
            }
        }
        Ok(())
    };
    describe(report, "Validated ink length by color", &global)?;
    for (index, colors) in per_page.iter().enumerate() {
        let page = index.checked_add(1)
            .ok_or("validated ink page number overflow")?;
        describe(report,
            &format!("Page {page} validated ink length by color"), colors)?;
    }
    Ok(())
}

// Report straight-line separations between consecutive segments as emitted
// within each physical sheet. A color switch or shared endpoint is still one
// transition: these are diagnostics, not a device itinerary or an optimized
// pen-up path. Do not invent any transition across a paper boundary.
#[derive(Clone, Copy, Default)]
struct InkEndpointSummary {
    segments: usize,
    transitions: usize,
    distance: u128,
    coincident: usize,
    separated: usize,
    color_changes: usize,
    maximum: u128,
}

fn append_flow_ink_gaps(
    report: &mut String, ink: &[NotebookInkLine], pages: usize,
    details: bool,
) -> Result<(), String> {
    let mut endpoints = vec![None::<((u32, u32), [u8; 3])>; pages];
    let mut summaries = vec![InkEndpointSummary::default(); pages];
    for line in ink {
        let page_index = line.page.checked_sub(1)
            .ok_or("validated ink references nonexistent physical page")?;
        let prior = endpoints.get_mut(page_index)
            .ok_or("validated ink references nonexistent physical page")?;
        let summary = summaries.get_mut(page_index)
            .ok_or("validated ink references nonexistent physical page")?;
        summary.segments = summary.segments.checked_add(1)
            .ok_or("validated ink segment count overflow")?;
        if let Some((end, previous_rgb)) = *prior {
            let distance = rounded_coordinate_length_um(end, line.start_um)?;
            summary.distance = summary.distance.checked_add(distance)
                .ok_or("validated ink gap length overflow")?;
            summary.transitions = summary.transitions.checked_add(1)
                .ok_or("validated ink gap count overflow")?;
            if end == line.start_um {
                summary.coincident = summary.coincident.checked_add(1)
                    .ok_or("validated ink coincident count overflow")?;
            } else {
                summary.separated = summary.separated.checked_add(1)
                    .ok_or("validated ink separated count overflow")?;
            }
            if previous_rgb != line.rgb {
                summary.color_changes = summary.color_changes.checked_add(1)
                    .ok_or("validated ink color change count overflow")?;
            }
            summary.maximum = summary.maximum.max(distance);
        }
        *prior = Some((line.end_um, line.rgb));
    }
    // Complete all checked arithmetic before appending any result. A late
    // invalid segment must not publish a partial inspection report.
    let mut global = InkEndpointSummary::default();
    for summary in &summaries {
        global.segments = global.segments.checked_add(summary.segments)
            .ok_or("validated ink global segment count overflow")?;
        global.transitions = global.transitions.checked_add(summary.transitions)
            .ok_or("validated ink global gap count overflow")?;
        global.distance = global.distance.checked_add(summary.distance)
            .ok_or("validated ink global gap length overflow")?;
        global.coincident = global.coincident.checked_add(summary.coincident)
            .ok_or("validated ink global coincident count overflow")?;
        global.separated = global.separated.checked_add(summary.separated)
            .ok_or("validated ink global separated count overflow")?;
        global.color_changes = global.color_changes
            .checked_add(summary.color_changes)
            .ok_or("validated ink global color change count overflow")?;
        global.maximum = global.maximum.max(summary.maximum);
    }
    let noun = if global.transitions == 1 { "transition" }
        else { "transitions" };
    writeln!(report,
        "Validated ink emitted endpoint gaps: {} um across {} {noun}",
        global.distance, global.transitions,
    ).map_err(|error| format!("flow ink gap report refused: {error}"))?;
    for (index, summary) in summaries.iter().enumerate() {
        let page = index + 1;
        if summary.segments == 0 {
            writeln!(report, "Page {page} validated ink endpoint gaps: empty")
        } else {
            let noun = if summary.transitions == 1 { "transition" }
                else { "transitions" };
            writeln!(report,
                "Page {page} validated ink endpoint gaps: {} um \
                 across {} {noun}", summary.distance, summary.transitions)
        }.map_err(|error| {
            format!("flow ink gap report refused: {error}")
        })?;
    }
    if details {
        let describe = |report: &mut String, label: &str,
                        summary: &InkEndpointSummary| -> Result<(), String> {
            if summary.segments == 0 {
                writeln!(report, "{label}: empty")
            } else {
                let coincident = if summary.coincident == 1 {
                    "endpoint" } else { "endpoints" };
                let separated = if summary.separated == 1 {
                    "endpoint" } else { "endpoints" };
                let change = if summary.color_changes == 1 {
                    "change" } else { "changes" };
                writeln!(report,
                    "{label}: {} coincident {coincident}, \
                     {} separated {separated}, {} RGB {change}, \
                     largest separation {} um",
                    summary.coincident, summary.separated,
                    summary.color_changes, summary.maximum)
            }.map_err(|error| format!(
                "flow ink gap detail report refused: {error}"))
        };
        describe(report, "Validated ink gap details", &global)?;
        for (index, summary) in summaries.iter().enumerate() {
            describe(report,
                &format!("Page {} validated ink gap details", index + 1),
                summary)?;
        }
    }
    Ok(())
}

// Opt-in full native geometry check. The same glyph expansion and physical
// PDF admission as export run in memory; no file is selected or written.
// Keep this separate from nominal coverage, which may inspect missing shapes.
fn append_flow_ink_validation(
    report: &mut String, request: &NotebookRequest,
    runs: Vec<TextRun>, ornaments: &[NotebookInkLine], pages: usize,
) -> Result<(), String> {
    // Match the export-only preflight: flow cannot be rendered without an
    // explicitly supplied glyph library, even for syntax-only ornaments.
    if request.glyphs.strokes.is_empty() {
        return Err("flow requires authored glyph shapes".into());
    }
    let flow_run_count = runs.len();
    let glyphs = GlyphOverlay {
        fallbacks: request.glyphs.fallbacks.clone(),
        runs,
        strokes: request.glyphs.strokes.clone(),
        ..GlyphOverlay::default()
    };
    // Flow rows are generated, not individually placed text. Preserve the
    // same generated-run boundary as export so the 16 *manual* run limit
    // never rejects a long, otherwise valid measured flow.
    let (ink, _) = glyphs.expand_with_run_boundary(
        ornaments, request.statistics.as_ref(), flow_run_count,
    )?;
    if let Some(region) = request.flow_region {
        region.validate_ink(request.profile.printable_rect_um()?, &ink)?;
    }
    // Validate every page, not only a selected PNG page. This also tests
    // the native global stroke budget and final printable pen clearance.
    drop(request.profile.paper_layer_pdf_with_ink_ruler(pages, &ink, None)?);
    let page_noun = if pages == 1 { "page" } else { "pages" };
    writeln!(report,
        "Native ink validation: {} segments across {pages} \
         physical {page_noun} (PDF geometry accepted; \
         no output file written)", ink.len(),
    ).map_err(|error| format!("flow ink validation refused: {error}"))?;
    if request.inspect_flow_page_inventory {
        let mut per_page = vec![0usize; pages];
        for line in &ink {
            let count = line.page.checked_sub(1)
                .and_then(|index| per_page.get_mut(index))
                .ok_or("validated ink references nonexistent physical page")?;
            *count = count.checked_add(1)
                .ok_or("flow ink page segment count overflow")?;
        }
        for (index, count) in per_page.iter().enumerate() {
            let noun = if *count == 1 { "segment" } else { "segments" };
            writeln!(report,
                "Page {} validated ink: {count} physical {noun}",
                index + 1,
            ).map_err(|error| {
                format!("flow ink page validation refused: {error}")
            })?;
        }
    }
    if request.inspect_flow_ink_bounds {
        append_flow_ink_bounds(report, &ink, pages)?;
    }
    if request.inspect_flow_ink_palette {
        append_flow_ink_palette(report, &ink, pages)?;
    }
    if request.inspect_flow_ink_widths {
        append_flow_ink_widths(report, &ink, pages)?;
    }
    if request.inspect_flow_ink_length {
        append_flow_ink_length(report, &ink, pages)?;
    }
    if request.inspect_flow_ink_length_by_color {
        append_flow_ink_length_by_color(report, &ink, pages)?;
    }
    if request.inspect_flow_ink_gaps {
        append_flow_ink_gaps(
            report, &ink, pages, request.inspect_flow_ink_gap_details,
        )?;
    }
    Ok(())
}

// Inspect nominal physical rows before drawing any authored glyph shapes.
fn inspect_notebook_flow(request: &NotebookRequest)
-> Result<Vec<u8>, String> {
    let source = request.flow.as_ref()
        .ok_or("--inspect-flow requires a text or Markdown flow")?;
    let (runs, ornaments, pages) = layout_flow_with_typography(
        source, &request.profile, request.statistics.as_ref(),
        request.heading, request.flow_region, request.flow_typography,
    )?;
    let mut report = format!(
        "Nominal flow (glyph coverage and ink clearance unchecked)\n\
         Pages: {pages}\nRows: {}\nOrnaments: {}\n",
        runs.len(), ornaments.len(),
    );
    let mut inventory = BTreeMap::<String, usize>::new();
    let mut page_inventories =
        BTreeMap::<usize, BTreeMap<String, usize>>::new();
    for row in &runs {
        write!(report,
            "Page {} x={} um y={} um: {:?}\n",
            row.page, row.origin_um.0, row.origin_um.1, row.text,
        ).map_err(|error| format!("flow report refused: {error}"))?;
        if request.inspect_flow_inventory
            || request.inspect_flow_shape_coverage {
            let mut remaining = row.text.as_str();
            while !remaining.is_empty() {
                let (grapheme, rest) = first_grapheme(remaining)?;
                if !is_non_ink_audit_grapheme(grapheme) {
                    let count = inventory.entry(grapheme.to_owned())
                        .or_default();
                    *count = count.checked_add(1)
                        .ok_or("flow inventory occurrence overflow")?;
                    if request.inspect_flow_page_inventory {
                        let page_count = page_inventories
                            .entry(row.page).or_default()
                            .entry(grapheme.to_owned()).or_default();
                        *page_count = page_count.checked_add(1)
                            .ok_or("flow page inventory occurrence overflow")?;
                    }
                }
                remaining = rest;
            }
        }
    }
    if request.inspect_flow_inventory {
        let total: usize = inventory.values().sum();
        write!(report,
            "Glyph inventory: {} exact forms, {total} uses\n",
            inventory.len(),
        ).map_err(|error| format!("flow inventory refused: {error}"))?;
        for (grapheme, uses) in &inventory {
            write!(report, "{grapheme:?}: {uses}\n")
                .map_err(|error| format!("flow inventory refused: {error}"))?;
        }
    }
    if request.inspect_flow_page_inventory {
        let page_noun = if pages == 1 { "page" } else { "pages" };
        write!(report, "Page glyph inventory: {pages} physical {page_noun}\n")
            .map_err(|error| format!("flow inventory refused: {error}"))?;
        for page in 1..=pages {
            let page_inventory = page_inventories.get(&page);
            let forms = page_inventory.map_or(0, BTreeMap::len);
            let uses: usize = page_inventory.map_or(0, |forms| {
                forms.values().sum()
            });
            write!(report,
                "Page {page}: {forms} exact forms, {uses} uses\n",
            ).map_err(|error| format!("flow inventory refused: {error}"))?;
            if let Some(forms) = page_inventory {
                for (grapheme, count) in forms {
                    write!(report, "  {grapheme:?}: {count}\n")
                        .map_err(|error| {
                            format!("flow inventory refused: {error}")
                        })?;
                }
            }
        }
    }
    if request.inspect_flow_shape_coverage {
        append_flow_shape_coverage(
            &mut report, &request.glyphs, &inventory, &page_inventories,
            pages, request.inspect_flow_page_inventory,
        )?;
    }
    if request.inspect_flow_stroke_demand {
        append_flow_stroke_demand(
            &mut report, &request.glyphs, &inventory, &page_inventories,
            &ornaments, pages, request.inspect_flow_page_inventory,
        )?;
    }
    if request.inspect_flow_stroke_breakdown {
        append_flow_stroke_breakdown(
            &mut report, &request.glyphs, &inventory, &page_inventories,
            pages, request.inspect_flow_page_inventory,
        )?;
    }
    if request.inspect_flow_ink_validation {
        append_flow_ink_validation(
            &mut report, request, runs, &ornaments, pages,
        )?;
    }
    Ok(report.into_bytes())
}

fn execute_notebook(args: &[String]) -> Result<Option<Vec<u8>>, String> {
    let mut request = parse_notebook_options(args)?;
    let canonical = request.profile.canonical_json()?;
    if request.inspect_flow {
        return Ok(Some(inspect_notebook_flow(&request)?));
    }
    if request.inspect {
        return Ok(Some(
            format!(
                concat!(
                    "Version: atrament.notebook.v1\n",
                    "Date: {} / {}\n",
                    "Printed date: {}\n",
                    "Border radius: {} um\n",
                    "Page numbers: {}\n",
                    "Printed sheet: {}\n"
                ),
                request.profile.date_style.key(),
                request.profile.date_placement.key(),
                request.profile.date_text,
                request.profile.border_radius_um,
                request.profile.page_numbers,
                request.profile.printed_sheet,
            )
            .into_bytes(),
        ));
    }
    // Manually authored ink and positioned glyph runs can extend the page
    // budget. They remain outside the optional flow-only rectangle while
    // the native compositor validates every pen segment on every page.
    let manual_ink_count = request.ink.len();
    let mut flow_run_count = None;
    if let Some(source) = &request.flow {
        let (runs, ornaments, flow_pages) = layout_flow_with_typography(
            source, &request.profile, request.statistics.as_ref(),
            request.heading, request.flow_region, request.flow_typography,
        )?;
        admit_flow_page_selection(&mut request, flow_pages)?;
        flow_run_count = Some(runs.len());
        let manual_runs = replace(&mut request.glyphs.runs, runs);
        request.glyphs.runs.extend(manual_runs);
        request.ink.extend(ornaments);
        request.glyphs.validate_pairing()?;
    } else {
        admit_manual_page_selection(&mut request)?;
    }
    let (ink, flow_ink_end) = if let Some(count) = flow_run_count {
        request.glyphs.expand_with_run_boundary(
            &request.ink, request.statistics.as_ref(), count,
        )?
    } else {
        (request.glyphs.expand(&request.ink, request.statistics.as_ref())?, 0)
    };
    if let Some(region) = request.flow_region {
        let generated = ink.get(manual_ink_count..flow_ink_end)
            .ok_or("flow ink boundary exceeds expanded stroke count")?;
        region.validate_ink(
            request.profile.printable_rect_um()?, generated,
        )?;
    }
    let (bytes, maximum_bytes) = match request.export_format {
        Some(NotebookExportFormat::InkTsv) => {
            // Reuse physical PDF admission on every page, including ink that
            // is not visible on the currently selected PNG page. These PDF
            // bytes are not returned or persisted.
            drop(request.profile.paper_layer_pdf_with_ink_ruler(
                request.pages, &ink, request.ruler_variation,
            )?);
            (encode_page_ink_file(&ink)?, MAX_PAGE_INK_FILE_BYTES)
        }
        Some(NotebookExportFormat::Pdf) => (
            request.profile.paper_layer_pdf_with_ink_ruler(
                request.pages, &ink, request.ruler_variation,
            )?,
            MAX_PAPER_OUTPUT_BYTES,
        ),
        Some(NotebookExportFormat::Png) => (
            request.profile.paper_layer_png_page_with_ink_ruler(
                request.dpi, request.png_page, &ink,
                request.ruler_variation,
            )?,
            MAX_PAPER_OUTPUT_BYTES,
        ),
        None => (canonical.into_bytes(), MAX_NOTEBOOK_JSON_BYTES),
    };
    let Some(output) = request.output else {
        return Ok(Some(bytes));
    };
    let disposition = if request.replace {
        ProfileFileCommitDisposition::ReplaceExisting
    } else {
        ProfileFileCommitDisposition::PreserveExisting
    };
    let _evidence = commit_profile_file(
        Path::new(&output),
        &bytes,
        maximum_bytes,
        disposition,
    )
    .map_err(|error| {
        if error.may_have_committed() {
            format!(
                "output durability unknown; inspect before retry: {error:?}"
            )
        } else {
            format!("output not saved: {error:?}")
        }
    })?;
    Ok(None)
}

fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    let Some((mode, rest)) = args.split_first() else {
        writeln!(io::stdout().lock(), "{USAGE}")?;
        return Ok(());
    };
    // A flag-looking value can be literal text for an audit or flow.
    // Only a sole mode option requests help; do not bypass admission for
    // --audit-text --help, --flow-text --help or invalid numeric operands.
    let help_only = matches!(rest, [option] if option == "--help");
    match mode.as_str() {
        "paper" => atrament_notebook_paper_cli::run_cli_args(rest)?,
        "glyphs" | "handwriting" if help_only => {
            writeln!(io::stdout().lock(), "{USAGE}")?;
        }
        "glyphs" => {
            if let Some(bytes) = execute_glyph_library(rest)? {
                io::stdout().lock().write_all(&bytes)?;
            }
        }
        "notebook" if help_only => {
            writeln!(io::stdout().lock(), "{NOTEBOOK_USAGE}")?;
        }
        "notebook" => {
            if let Some(bytes) = execute_notebook(rest)? {
                io::stdout().lock().write_all(&bytes)?;
            }
        }
        "handwriting" => {
            if let Some(bytes) = execute_handwriting(rest)? {
                io::stdout().lock().write_all(&bytes)?;
            }
        }
        "--help" | "help" => writeln!(io::stdout().lock(), "{USAGE}")?,
        _ => {
            return Err(
                format!("unknown profile family: {mode}; use --help").into()
            );
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    run(&args)
}

#[cfg(test)]
#[path = "../../../../tests/backend/profile-editor-cli/composition/lib.rs"]
mod tests;
