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
//   - One Rust CLI entrypoint for canonical paper and handwriting profiles.
// - Must-Not:
//   - Independently validate profile statistics, sample glyphs, or render ink.
// - Allows:
//   - Inputs: Explicit CLI arguments and bounded caller-selected JSON files.
//   - Outputs: Rust-admitted JSON, inspection text, or explicit refusal.
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

use std::env;
use std::error::Error;
use std::fmt::Write as _;
use std::fs::{File, metadata};
use std::io::{self, Write as _};
use std::path::Path;
use std::slice::Iter;

use atrament_handwriting_statistics::{
    Dimension, DimensionStatistics, HandwritingStatisticsProfile,
    ScalarStatistics, Unit,
};
use atrament_handwriting_statistics_json::{
    MAXIMUM_STATISTICS_JSON_BYTES, decode_statistics_json,
    encode_statistics_json,
};
use atrament_notebook_paper_cli::{
    MAX_PAPER_OUTPUT_BYTES, canonical_paper_profile_json,
};
use atrament_notebook_profile::{
    DatePlacement, DateStyle, MAX_NOTEBOOK_JSON_BYTES, NotebookProfile,
};
use atrament_profile_file_commit::{
    ProfileFileCommitDisposition, commit_profile_file,
};
use atrament_profile_file_reader::read_bounded_profile_file;
use atrament_unicode_grapheme_segmentation::UnicodeGraphemeSegmentation;

const USAGE: &str = "Usage: atrament-profiles notebook [--help] \
    | paper [atrament-paper options] \
    | handwriting [--profile ink.json] [--seed U64] \
    [--set-global DIM MIN MAX MEAN MEDIAN DEVIATION] \
    [--set-glyph GRAPHEME DIM MIN MAX MEAN MEDIAN DEVIATION] \
    [--remove-glyph GRAPHEME] [--remove-glyph-dimension GRAPHEME DIM] \
    [--inspect] [--output FILE [--replace]]\n\
    Paper is the existing Rust physical-page profile (not a full notebook).\n\
    Handwriting units: micrometres, millidegrees, or parts per thousand.\n\
    Run paper --help for page sizes, styles, margins and PDF options.";

const NOTEBOOK_USAGE: &str = "Usage: atrament-profiles notebook \
    [--profile notebook.json] [--paper-profile paper.json] \
    [--date-style none|line|box] \
    [--date-placement none|top-left|top-right] \
    [--border-radius-um INTEGER] [--page-numbers true|false] \
    [--printed-sheet true|false] [--inspect] \
    [--export-paper pdf|png] [--pages 1..32 (PDF only)] \
    [--dpi 72..300 (PNG only)] [--output FILE [--replace]]\n\
    Paper-layer export uses nested Rust paper settings and skips ruling \
    on preprinted sheets. The selected empty date line or box is drawn; \
    date text, borders and ink do not render. Page numbers use a \
    small printed bitmap in the bottom margin.";

#[derive(Debug)]
struct HandwritingRequest {
    inspect: bool,
    output: Option<String>,
    profile: HandwritingStatisticsProfile,
    replace: bool,
}

#[derive(Debug)]
struct NotebookRequest {
    dpi: u32,
    export_paper: Option<NotebookPaperFormat>,
    inspect: bool,
    output: Option<String>,
    pages: usize,
    profile: NotebookProfile,
    replace: bool,
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
}

fn read_handwriting_profile(
    path: &str,
) -> Result<HandwritingStatisticsProfile, String> {
    // Avoid a blocking FIFO open for ordinary explicit CLI path selection.
    // This is preflight, not a substitute for opened-handle verification.
    let source = metadata(path).map_err(|error| {
        format!("cannot inspect handwriting profile: {error}")
    })?;
    if !source.is_file() {
        return Err("handwriting file refused: NotRegularFile".into());
    }
    let file = File::open(path)
        .map_err(|error| format!("cannot open handwriting profile: {error}"))?;
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

fn parse_handwriting_options(
    args: &[String],
) -> Result<HandwritingRequest, String> {
    let boundaries = UnicodeGraphemeSegmentation;
    let mut profile = HandwritingStatisticsProfile::baseline(0);
    let mut seen_profile = false;
    let mut seen_edit = false;
    let mut inspect = false;
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
            "--inspect" => {
                if inspect {
                    return Err("--inspect may appear only once".into());
                }
                inspect = true;
            }
            "--output" => {
                let selected =
                    iter.next().ok_or("--output requires a filename")?;
                if output.is_some()
                    || selected.starts_with('-')
                    || selected.is_empty()
                {
                    return Err(
                        "--output requires one explicit filename".into()
                    );
                }
                output = Some(selected.clone());
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
    if replace && output.is_none() {
        return Err("--replace requires --output".into());
    }
    if inspect && output.is_some() {
        return Err("--inspect is stdout only; omit --output".into());
    }
    profile
        .validate(&boundaries)
        .map_err(|error| format!("invalid handwriting profile: {error:?}"))?;
    Ok(HandwritingRequest {
        inspect,
        output,
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

fn execute_handwriting(args: &[String]) -> Result<Option<Vec<u8>>, String> {
    let request = parse_handwriting_options(args)?;
    let boundaries = UnicodeGraphemeSegmentation;
    let canonical = encode_statistics_json(&request.profile, &boundaries)
        .map_err(|error| format!("statistics encoding refused: {error:?}"))?;
    if request.inspect {
        return Ok(Some(inspect_handwriting(&request.profile)));
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
    let source = metadata(path)
        .map_err(|error| format!("cannot inspect notebook input: {error}"))?;
    if !source.is_file() {
        return Err("notebook input refused: NotRegularFile".into());
    }
    let file = File::open(path)
        .map_err(|error| format!("cannot open notebook input: {error}"))?;
    let bytes = read_bounded_profile_file(file, limit)
        .map_err(|error| format!("notebook input refused: {error:?}"))?;
    Ok(bytes.bytes().to_vec())
}

fn apply_notebook_edit(
    profile: &mut NotebookProfile,
    option: &str,
    value: &str,
) -> Result<(), String> {
    match option {
        "--date-style" => profile.date_style = DateStyle::parse(value)?,
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

fn parse_notebook_export_option(
    option: &str,
    iter: &mut Iter<'_, String>,
    export: &mut NotebookExportOptions,
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
        _ => return Err(format!("unknown notebook export option: {option}")),
    }
    Ok(())
}

fn parse_notebook_options(args: &[String]) -> Result<NotebookRequest, String> {
    let mut profile = NotebookProfile::default();
    let mut inspect = false;
    let mut output = None;
    let mut replace = false;
    let mut export = NotebookExportOptions::default();
    let mut loaded = false;
    let mut edited = false;
    let mut iter = args.iter();
    while let Some(option) = iter.next() {
        match option.as_str() {
            "--profile" => {
                if loaded || edited {
                    return Err(
                        "--profile must appear once, before edits".into()
                    );
                }
                let path = iter.next().ok_or("--profile requires a file")?;
                profile = NotebookProfile::from_json(&read_notebook_input(
                    path,
                    MAX_NOTEBOOK_JSON_BYTES,
                )?)?;
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
            "--date-style" | "--date-placement" | "--border-radius-um"
            | "--page-numbers" | "--printed-sheet" => {
                let value = iter
                    .next()
                    .ok_or_else(|| format!("{option} requires a value"))?;
                apply_notebook_edit(&mut profile, option, value)?;
                edited = true;
            }
            "--inspect" => {
                if inspect {
                    return Err("--inspect may appear only once".into());
                }
                inspect = true;
            }
            "--export-paper" | "--pages" | "--dpi" => {
                parse_notebook_export_option(option, &mut iter, &mut export)?;
            }
            "--output" => {
                let path = iter.next().ok_or("--output requires a filename")?;
                if output.is_some() || path.is_empty() || path.starts_with('-')
                {
                    return Err(
                        "--output requires one explicit filename".into()
                    );
                }
                output = Some(path.clone());
            }
            "--replace" => {
                if replace {
                    return Err("--replace may appear only once".into());
                }
                replace = true;
            }
            _ => return Err(format!("unknown notebook option: {option}")),
        }
    }
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
    if export.dpi.is_some() && export.format != Some(NotebookPaperFormat::Png) {
        return Err("--dpi requires --export-paper png".into());
    }
    profile.validate()?;
    Ok(NotebookRequest {
        dpi: export.dpi.unwrap_or(150),
        export_paper: export.format,
        inspect,
        output,
        pages: export.pages.unwrap_or(1),
        profile,
        replace,
    })
}

fn execute_notebook(args: &[String]) -> Result<Option<Vec<u8>>, String> {
    let request = parse_notebook_options(args)?;
    let canonical = request.profile.canonical_json()?;
    if request.inspect {
        return Ok(Some(
            format!(
                concat!(
                    "Version: atrament.notebook.v1\n",
                    "Date: {} / {}\n",
                    "Border radius: {} um\n",
                    "Page numbers: {}\n",
                    "Printed sheet: {}\n"
                ),
                request.profile.date_style.key(),
                request.profile.date_placement.key(),
                request.profile.border_radius_um,
                request.profile.page_numbers,
                request.profile.printed_sheet,
            )
            .into_bytes(),
        ));
    }
    let (bytes, maximum_bytes) = match request.export_paper {
        Some(NotebookPaperFormat::Pdf) => (
            request.profile.paper_layer_pdf(request.pages)?,
            MAX_PAPER_OUTPUT_BYTES,
        ),
        Some(NotebookPaperFormat::Png) => (
            request.profile.paper_layer_png(request.dpi)?,
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
