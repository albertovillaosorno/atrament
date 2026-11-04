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

use std::env;
use std::error::Error;
use std::fmt::Write as _;
use std::io::{self, Write as _};
use std::path::Path;
use std::slice::Iter;

#[cfg(test)]
use flow::layout_flow;
use flow::{
    FlowFormat, FlowRegion, FlowSource, HeadingDecoration,
    layout_flow_with_region,
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
    canonical_paper_profile_json,
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

const USAGE: &str = "Usage: atrament-profiles notebook [--help] \
    | paper [atrament-paper options] \
    | glyphs [--profile strokes.tsv] \
    [--glyph-stroke GRAPHEME X1 Y1 X2 Y2 WIDTH_UM '#RRGGBB'] \
    [--copy-glyph SOURCE TARGET] \
    [--remove-glyph GRAPHEME] \
    [--reverse-stroke GRAPHEME INDEX] \
    [--move-stroke GRAPHEME FROM TO] \
    [--reverse-path GRAPHEME PATH_INDEX] \
    [--close-path GRAPHEME PATH_INDEX] \
    [--rotate-path GRAPHEME PATH_INDEX START_STROKE] \
    [--move-path GRAPHEME FROM_PATH TO_PATH] \
    [--translate-path GRAPHEME PATH_INDEX DX_UM DY_UM] \
    [--shear-glyph GRAPHEME ANGLE_DEG BASELINE_Y_UM] \
    [--scale-glyph-height GRAPHEME PERCENT BASELINE_Y_UM] \
    [--scale-glyph-width GRAPHEME PERCENT ANCHOR_X_UM] \
    [--scale-glyph-ink GRAPHEME PERCENT] \
    [--translate-glyph GRAPHEME DX_UM DY_UM] \
    [--rotate-glyph GRAPHEME ANGLE_DEG PIVOT_X_UM PIVOT_Y_UM] \
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
    --set-stroke updates ink geometry; --remove-stroke deletes one segment.\n\
    --reverse-path flips a multi-stroke path; --move-path swaps adjacent\n\
    paths using the current pen-continuity inspection order.\n\
    --rotate-path changes the first stroke of a closed loop (one-based).\n\
    --close-path inserts a short closing segment on a selected open path.\n\
    --translate-path moves one path using bounded signed micrometres.\n\
    --shear-glyph slants all exact-glyph pen segments 0..20 degrees.\n\
    --scale-glyph-height rescales exact-glyph Y about the baseline.\n\
    --scale-glyph-width rescales exact-glyph X about the anchor.\n\
    --scale-glyph-ink rescales exact-glyph pen thickness 50..150%.\n\
    --translate-glyph moves all exact-glyph strokes within the pen cell.\n\
    --rotate-glyph rotates ordered ink clockwise around an X/Y pivot.\n\
    --join-strokes inserts a short same-pen connector to the next segment.\n\
    --copy-glyph clones explicit source ink under an unoccupied target.\n\
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
    [--printed-sheet true|false] [--inspect] \
    [--export-paper pdf|png] [--pages 1..32 (PDF only)] \
    [--dpi 72..300 (PNG only)] [--png-page 1..32 (PNG only)] \
    [--quality preview|final (PNG only, exclusive with --dpi)] \
    [--ink-line PAGE X1_UM Y1_UM X2_UM Y2_UM WIDTH_UM '#RRGGBB'] \
    [--glyph-file strokes.tsv] \
    [--glyph-stroke GRAPHEME X1 Y1 X2 Y2 WIDTH_UM '#RRGGBB'] \
    [--fallback-glyph MISSING AUTHORED] \
    [--place-text PAGE X_UM Y_UM UTF8_TEXT] \
    [--join-text-pair LEFT RIGHT (repeatable, max 32)] \
    [--join-text-anchors LEFT RIGHT EXIT_STROKE ENTRY_STROKE] \
    [--glyph-variation SEED WIDTH HEIGHT THICKNESS SLANT SPACING REGULARITY] \
    [--heading-style underline|double-underline|marker|none] \
    [--heading-color '#RRGGBB'] [--heading-marker-scale 50..105] \
    [--heading-ink-color '#RRGGBB'] [--heading-ink-scale 100..250] \
    [--flow-text TEXT | --flow-markdown MARKDOWN | \
     --flow-text-file FILE | --flow-markdown-file FILE] \
    [--flow-region LEFT_UM TOP_UM WIDTH_UM HEIGHT_UM] \
    [--handwriting-profile ink.json (requires glyph shapes)] \
    [--output FILE [--replace]]\n\
    Paper-layer export uses nested Rust paper settings and skips ruling \
    on preprinted sheets. The optional date prints in its selected box \
    or line; rounded borders render. Repeated --ink-line (max 4096) draws \
    explicit, unsaved physical pen segments. --glyph-stroke supplies exact \
    author-drawn segments in a 4 x 5 mm cell and --place-text repeats them; \
    missing shapes refuse; --glyph-file loads a reusable library. \
    --join-text-pair opt-in bridges selected adjacent glyph pen endpoints \
    within one row; matching physical ink and <=1200 um gap required. \
    --join-text-anchors chooses explicit 1-based source exit and target \
    entry strokes for an adjacent pair, without modifying glyph ink. \
    --fallback-glyph borrows a caller-selected existing authored shape \
    for one missing exact grapheme; no implicit or chained mappings. \
    --flow-text keeps explicit lines; --flow-markdown joins plain lines \
    into paragraphs. Two trailing spaces or one trailing backslash force \
    a hard row break. Markdown supports # through ###### headings, \
    -/*/+ bullets, \
    [ ]/[x]/[X] checkbox tasks after bullets, > quotes, 1./1) numbered \
    lists (authored digits/punctuation), and standalone ---/***/___ \
    horizontal ink rules. -file reads bounded UTF-8. \
    An initial UTF-8 BOM is ignored but counts toward the 4096-byte cap. \
    Both use physical printable regions and exact authored ink; \
    optional --flow-region repeats a fixed rectangle within margins on \
    every sheet, refusing invalid regions or unfit glyphs/rows. \
    It auto-selects PDF page count unless --pages is explicit. \
    Markdown headings default to dark underlines; select a user-colored \
    single/double underline, opaque pastel marker band or no ornament. \
    --heading-marker-scale adjusts the marker underlay pen width (percent) \
    only with --heading-style marker; the default is 100. \
    --heading-ink-color recolors heading glyph ink, and \
    --heading-ink-scale enlarges only heading pen widths (percent); \
    the authored body strokes and glyph templates remain unchanged. \
    --handwriting-profile applies selected mean dimensions to authored \
    shapes, not statistical sampling. --glyph-variation explicitly \
    applies reproducible per-occurrence creative jitter to the same shapes; \
    the six spreads use percent, percent, percent, degrees, um, um. \
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
    loaded_file: bool,
    runs: Vec<TextRun>,
    strokes: Vec<GlyphStroke>,
    text_joins: Vec<TextJoin>,
    variation: Option<GlyphVariation>,
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
    export_paper: Option<NotebookPaperFormat>,
    flow: Option<FlowSource>,
    flow_region: Option<FlowRegion>,
    glyphs: GlyphOverlay,
    heading: HeadingDecoration,
    ink: Vec<NotebookInkLine>,
    inspect: bool,
    output: Option<String>,
    pages: usize,
    pages_explicit: bool,
    png_page: usize,
    profile: NotebookProfile,
    replace: bool,
    statistics: Option<HandwritingStatisticsProfile>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NotebookPaperFormat {
    Pdf,
    Png,
}

#[derive(Default)]
struct NotebookExportOptions {
    dpi: Option<u32>,
    format: Option<NotebookPaperFormat>,
    pages: Option<usize>,
    png_page: Option<usize>,
    quality_dpi: Option<u32>,
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

fn parse_parameter(values: &[String]) -> Result<DimensionStatistics, String> {
    let [name, min, max, mean, median, deviation] = values else {
        return Err(
            "statistics require DIM MIN MAX MEAN MEDIAN DEVIATION".into()
        );
    };
    let dimension = Dimension::from_key(name)
        .ok_or_else(|| format!("unknown statistics dimension: {name}"))?;
    let integer = |value: &str| {
        value
            .parse::<i32>()
            .map_err(|error| format!("invalid statistics integer: {error}"))
    };
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

fn parse_handwriting_options(
    args: &[String],
) -> Result<HandwritingRequest, String> {
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
                profile.seed = value
                    .parse::<u64>()
                    .map_err(|error| format!("invalid seed: {error}"))?;
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

fn read_glyph_input(path: &str) -> Result<Vec<u8>, String> {
    let file = open_selected_profile_file(Path::new(path))
        .map_err(|error| format!("glyph file refused: {error:?}"))?;
    let bytes = read_bounded_profile_file(file, MAX_GLYPH_LIBRARY_BYTES)
        .map_err(|error| format!("glyph file refused: {error:?}"))?;
    Ok(bytes.bytes().to_vec())
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

fn execute_glyph_library(args: &[String]) -> Result<Option<Vec<u8>>, String> {
    let mut glyphs = GlyphOverlay::default();
    let (mut output, mut audit_text) = (None, None);
    let mut audit_baseline = false;
    let mut inspection: Option<(String, bool)> = None;
    let mut replace = false;
    let mut iter = args.iter();
    while let Some(option) = iter.next() {
        match option.as_str() {
            "--profile" => {
                let selected = iter.next()
                    .ok_or("--profile requires a glyph library path")?;
                glyphs.load_file_bytes(&read_glyph_input(selected)?)?;
            }
            "--copy-glyph" | "--glyph-stroke" | "--rotate-glyph"
            | "--translate-glyph" | "--shear-glyph"
            | "--scale-glyph-height" | "--scale-glyph-width"
            | "--scale-glyph-ink" => glyphs.parse(option, &mut iter)?,
            "--reverse-stroke" | "--move-stroke" | "--join-strokes"
            | "--set-stroke" | "--remove-stroke" =>
                glyphs.edit_stroke(option, &mut iter)?,
            "--reverse-path" | "--rotate-path" | "--close-path" | "--move-path"
            | "--translate-path" => glyphs.edit_path(option, &mut iter)?,
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
        "--date-text" => value.clone_into(&mut profile.date_text),
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
    if color.len() != 7 || !color.is_ascii() || !color.starts_with('#') {
        return Err("ink color must be #RRGGBB".into());
    }
    let byte = |start: usize| -> Result<u8, String> {
        let end = start.saturating_add(2);
        let digits = color.get(start..end).ok_or("ink color must be #RRGGBB")?;
        u8::from_str_radix(digits, 16)
            .map_err(|_error| "ink color must be #RRGGBB".into())
    };
    Ok(NotebookInkLine {
        page: usize::try_from(page)
            .map_err(|error| format!("invalid ink page: {error}"))?,
        start_um: (x1, y1),
        end_um: (x2, y2),
        width_um,
        rgb: [byte(1)?, byte(3)?, byte(5)?],
    })
}

fn parse_notebook_export_option(
    option: &str,
    iter: &mut Iter<'_, String>,
    export: &mut NotebookExportOptions,
    ink: &mut Vec<NotebookInkLine>,
    glyphs: &mut GlyphOverlay,
) -> Result<(), String> {
    match option {
        "--export-paper" => {
            if export.format.is_some() {
                return Err("--export-paper may appear only once".into());
            }
            export.format = Some(match iter.next().map(String::as_str) {
                Some("pdf") => NotebookPaperFormat::Pdf,
                Some("png") => NotebookPaperFormat::Png,
                _ => return Err("--export-paper requires pdf or png".into()),
            });
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
            export.quality_dpi = Some(match iter.next().map(String::as_str) {
                Some("preview") => 72,
                Some("final") => 150,
                _ => return Err("--quality requires preview or final".into()),
            });
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
        | "--glyph-variation" => {
            glyphs.parse(option, iter)?;
        }
        "--join-text-pair" => glyphs.add_text_join_pair(iter)?,
        "--join-text-anchors" => glyphs.add_text_join_anchors(iter)?,
        "--glyph-file" => {
            let selected = iter.next()
                .ok_or("--glyph-file requires a library path")?;
            glyphs.load_file_bytes(&read_glyph_input(selected)?)?;
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
    inspect: bool,
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
        return Err("--inspect cannot export paper".into());
    }
    if export.pages.is_some()
        && export.format != Some(NotebookPaperFormat::Pdf)
    {
        return Err("--pages requires --export-paper pdf".into());
    }
    if (export.dpi.is_some() || export.png_page.is_some()
        || export.quality_dpi.is_some())
        && export.format != Some(NotebookPaperFormat::Png)
    {
        return Err(
            "--dpi, --quality and --png-page require --export-paper png".into()
        );
    }
    if export.dpi.is_some() && export.quality_dpi.is_some() {
        return Err("--quality and --dpi are mutually exclusive".into());
    }
    if (glyphs.has_any() || !ink.is_empty()) && export.format.is_none() {
        return Err(concat!(
            "ink and glyph overlays need ",
            "--export-paper pdf or png",
        ).into());
    }
    if glyphs.variation.is_some() && glyphs.strokes.is_empty() {
        return Err("--glyph-variation needs authored glyph shapes".into());
    }
    if statistics.is_some() && !glyphs.has_any() {
        return Err(concat!(
            "--handwriting-profile requires both --glyph-stroke ",
            "and --place-text",
        ).into());
    }
    Ok(())
}

fn parse_notebook_handwriting_profile(
    iter: &mut Iter<'_, String>,
    statistics: &mut Option<HandwritingStatisticsProfile>,
) -> Result<(), String> {
    if statistics.is_some() {
        return Err("--handwriting-profile may appear only once".into());
    }
    let path = iter.next().ok_or(
        "--handwriting-profile requires a saved profile path"
    )?;
    if path.is_empty() || path.starts_with('-') {
        return Err("invalid handwriting profile path".into());
    }
    *statistics = Some(read_handwriting_profile(path)?);
    Ok(())
}

fn parse_notebook_flow(
    iter: &mut Iter<'_, String>, option: &str,
    flow: &mut Option<FlowSource>,
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
    *flow = Some(if option.ends_with("-file") {
        FlowSource::from_file(format, source)?
    } else {
        FlowSource::new(format, source)?
    });
    Ok(())
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

fn validate_notebook_flow_selection(
    flow: Option<&FlowSource>, glyphs: &GlyphOverlay,
    heading: HeadingDecoration, region: Option<FlowRegion>,
) -> Result<(), String> {
    if region.is_some() && flow.is_none() {
        return Err("--flow-region requires a text or Markdown flow".into());
    }
    heading.validate().map_err(str::to_owned)?;
    if heading.is_selected() && flow.is_none_or(|source| {
        source.format != FlowFormat::Markdown
    }) {
        return Err("heading customization requires --flow-markdown".into());
    }
    if flow.is_none() {
        glyphs.validate_pairing()?;
    } else if glyphs.strokes.is_empty() || !glyphs.runs.is_empty() {
        return Err(concat!(
            "flow requires authored glyph shapes without manual ",
            "--place-text runs",
        ).into());
    }
    Ok(())
}

fn parse_notebook_options(args: &[String]) -> Result<NotebookRequest, String> {
    let mut profile = NotebookProfile::default();
    let mut inspect = false;
    let mut output = None;
    let mut replace = false;
    let mut export = NotebookExportOptions::default();
    let mut ink = Vec::new();
    let mut glyphs = GlyphOverlay::default();
    let mut statistics = None;
    let mut flow = None;
    let mut flow_region = None;
    let mut heading = HeadingDecoration::default();
    let mut loaded = false;
    let mut edited = false;
    let mut iter = args.iter();
    while let Some(option) = iter.next() {
        match option.as_str() {
            "--profile" => {
                profile = load_notebook_profile_option(
                    &mut iter, loaded, edited,
                )?;
                loaded = true;
            }
            "--paper-profile" => {
                let path =
                    iter.next().ok_or("--paper-profile requires a file")?;
                let paper = read_notebook_input(
                    path,
                    atrament_notebook_paper_cli::MAX_PROFILE_BYTES,
                )?;
                profile.paper_json = canonical_paper_profile_json(&paper)?;
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
            "--handwriting-profile" =>
                parse_notebook_handwriting_profile(&mut iter, &mut statistics)?,
            "--heading-style" | "--heading-color" | "--heading-ink-color"
            | "--heading-ink-scale" | "--heading-marker-scale" =>
                heading.parse(option, &mut iter)?,
            "--flow-region" => flow_region = Some(parse_notebook_flow_region(
                &mut iter, flow_region.is_some())?),
            "--flow-text" | "--flow-markdown"
            | "--flow-text-file" | "--flow-markdown-file" =>
                parse_notebook_flow(&mut iter, option, &mut flow)?,
            "--export-paper" | "--pages" | "--dpi" | "--quality"
            | "--png-page" | "--ink-line" | "--glyph-stroke"
            | "--glyph-file" | "--place-text" | "--fallback-glyph"
            | "--glyph-variation" | "--join-text-pair"
            | "--join-text-anchors" => {
                parse_notebook_export_option(
                    option, &mut iter, &mut export, &mut ink,
                    &mut glyphs,
                )?;
            }
            "--output" => output = Some(parse_selected_output(
                &mut iter, output.is_some(),
            )?),
            "--replace" => {
                if replace {
                    return Err("--replace may appear only once".into());
                }
                replace = true;
            }
            _ => return Err(format!("unknown notebook option: {option}")),
        }
    }
    validate_notebook_export_flags(
        replace, output.as_deref(), inspect, &export, &ink,
        &glyphs, statistics.as_ref(),
    )?;
    validate_notebook_flow_selection(
        flow.as_ref(), &glyphs, heading, flow_region,
    )?;
    profile.validate()?;
    Ok(NotebookRequest {
        flow,
        flow_region,
        pages_explicit: export.pages.is_some(),
        glyphs,
        heading,
        dpi: export.dpi.or(export.quality_dpi).unwrap_or(150),
        export_paper: export.format,
        ink,
        inspect,
        output,
        pages: export.pages.unwrap_or(1),
        png_page: export.png_page.unwrap_or(1),
        profile,
        replace,
        statistics,
    })
}

fn execute_notebook(args: &[String]) -> Result<Option<Vec<u8>>, String> {
    let mut request = parse_notebook_options(args)?;
    let canonical = request.profile.canonical_json()?;
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
    // Explicit manual ink is independent of the requested flow rectangle.
    // Only ornaments and glyphs generated by the measured flow are bound.
    let manual_ink_count = request.ink.len();
    if let Some(source) = &request.flow {
        let (runs, ornaments, required_pages) = layout_flow_with_region(
            source, &request.profile, request.statistics.as_ref(),
            request.heading, request.flow_region,
        )?;
        if request.pages_explicit && request.pages < required_pages {
            return Err(format!(concat!(
                "flow requires {} physical pages, ",
                "more than the explicit --pages count",
            ), required_pages));
        }
        if request.export_paper == Some(NotebookPaperFormat::Png)
            && request.png_page > required_pages {
            return Err(format!(
                "flow has {required_pages} pages; selected PNG page is empty"
            ));
        }
        request.pages = request.pages.max(required_pages);
        request.glyphs.runs = runs;
        request.ink.extend(ornaments);
        request.glyphs.validate_pairing()?;
    }
    let ink = request.glyphs.expand(
        &request.ink, request.statistics.as_ref(),
    )?;
    if let Some(region) = request.flow_region {
        let generated = ink.get(manual_ink_count..)
            .ok_or("flow ink offset exceeds expanded stroke count")?;
        region.validate_ink(
            request.profile.printable_rect_um()?, generated,
        )?;
    }
    let (bytes, maximum_bytes) = match request.export_paper {
        Some(NotebookPaperFormat::Pdf) => (
            request.profile.paper_layer_pdf_with_ink(
                request.pages, &ink,
            )?,
            MAX_PAPER_OUTPUT_BYTES,
        ),
        Some(NotebookPaperFormat::Png) => (
            request.profile.paper_layer_png_page_with_ink(
                request.dpi, request.png_page, &ink,
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
    match mode.as_str() {
        "paper" => atrament_notebook_paper_cli::run_cli_args(rest)?,
        "glyphs" if rest.iter().any(|arg| arg == "--help") => {
            writeln!(io::stdout().lock(), "{USAGE}")?;
        }
        "glyphs" => {
            if let Some(bytes) = execute_glyph_library(rest)? {
                io::stdout().lock().write_all(&bytes)?;
            }
        }
        "notebook" if rest.iter().any(|arg| arg == "--help") => {
            writeln!(io::stdout().lock(), "{NOTEBOOK_USAGE}")?;
        }
        "notebook" => {
            if let Some(bytes) = execute_notebook(rest)? {
                io::stdout().lock().write_all(&bytes)?;
            }
        }
        "handwriting" if rest.iter().any(|arg| arg == "--help") => {
            writeln!(io::stdout().lock(), "{USAGE}")?;
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
