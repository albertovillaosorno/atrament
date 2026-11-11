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
//   - CLI process arguments, stdout, and explicitly selected file writes.
// - Must-Not:
//   - Duplicate the paper-profile authority or write without user selection.
// - Allows:
//   - Inputs: Parsed caller arguments and rendered native paper bytes.
//   - Outputs: Exact PDF/PNG/profile bytes or explicit refusal.
//   - Side effects: stdout or atomic caller-selected file commit.
// - Split-When:
//   - Paper export gains a distinct transport or output device adapter.
// - Merge-When:
//   - A unified native export composition owns all CLI effects.
// - Summary:
//   - Keeps external CLI effects out of the pure paper application module.
// - Description:
//   - Preserves the shared Rust paper CLI entrypoints and write safeguards.
// - Usage:
//   - atrament-paper --format png --output selected.png
// - Defaults:
//   - stdout only; no implicit writes or printing.
//

//! Native paper CLI transport around the application renderer.

use std::env;
use std::error::Error;
use std::io::{self, Write as _};
use std::path::Path;

use atrament_profile_file_commit::{
    ProfileFileCommitDisposition, commit_profile_file,
};
use atrament_profile_file_reader::read_bounded_profile_file;
use atrament_profile_selected_file_open::open_selected_profile_file;

use super::application::{
    CliRequest, DEFAULT_PNG_DPI, MAX_PAPER_OUTPUT_BYTES, MAX_PROFILE_BYTES,
    Options, OutputFormat, USAGE, apply_paper_edit, decode_profile,
    encode_profile, orient_page, parse_image_export_option, parse_page_count,
    render_pages, render_png, resolve_png_quality, validate_export_flags,
};

#[expect(clippy::redundant_pub_crate, reason = "Sibling CLI regression tests")]
pub(super) fn read_profile(path: &str) -> Result<Options, String> {
    // The selected file opener checks file type on its opened descriptor,
    // refusing symlink components and path swaps rather than trusting a
    // separate metadata preflight before File::open.
    let file = open_selected_profile_file(Path::new(path))
        .map_err(|error| format!("paper profile file refused: {error:?}"))?;
    let bytes = read_bounded_profile_file(file, MAX_PROFILE_BYTES)
        .map_err(|error| format!("paper profile file refused: {error:?}"))?;
    decode_profile(bytes.bytes())
}

fn select_output_path(
    output: &mut Option<String>,
    value: &str,
) -> Result<(), String> {
    if output.is_some() || value.starts_with('-') || value.is_empty() {
        return Err("--output requires one explicit file path".into());
    }
    *output = Some(value.to_owned());
    Ok(())
}

#[expect(clippy::redundant_pub_crate, reason = "Sibling CLI regression tests")]
pub(super) fn parse_options(
    args: &[String],
) -> Result<Option<CliRequest>, String> {
    if args == ["--help"] {
        return Ok(None);
    }
    // Check every source-independent flag before opening a selected profile.
    // This uses the same typed parsers as the final pass, so a malformed late
    // option cannot consume a source or obscure the argument refusal.
    let preflight = parse_options_with_profile(args, false)?;
    if preflight.is_none() {
        return Ok(None);
    }
    parse_options_with_profile(args, true)
}

fn select_once(seen: &mut bool, error: &str) -> Result<(), String> {
    if *seen {
        return Err(error.into());
    }
    *seen = true;
    Ok(())
}

fn source_profile_options(
    path: &str,
    read_source: bool,
) -> Result<Options, String> {
    // Missing operands must not consume the following option and turn
    // it into an unintended local source. Prefix an actual dash-named
    // file with ./ to select it unambiguously.
    if path.is_empty() || path.starts_with('-') {
        return Err("--profile requires one explicit file path".into());
    }
    if read_source {
        read_profile(path)
    } else {
        Ok(Options::default())
    }
}

fn parse_options_with_profile(
    args: &[String],
    read_source: bool,
) -> Result<Option<CliRequest>, String> {
    let mut opts = Options::default();
    let mut dpi = DEFAULT_PNG_DPI;
    let mut seen_dpi = false;
    let mut quality_dpi = None;
    let mut format = OutputFormat::Pdf;
    let mut seen_format = false;
    let mut emit_profile = false;
    let mut seen_edit = false;
    let mut seen_profile = false;
    let mut seen_size_override = false;
    let mut seen_preset = false;
    let mut seen_orientation = false;
    let mut pages = 1;
    let mut seen_pages = false;
    let mut output = None;
    let mut replace = false;
    let mut items = args.iter();
    while let Some(name) = items.next() {
        if name == "--help" {
            return Err("--help must be the only argument".into());
        }
        if name == "--emit-profile" {
            select_once(&mut emit_profile,
                "--emit-profile may appear only once")?;
            continue;
        }
        if name == "--replace" {
            select_once(&mut replace, "--replace may appear only once")?;
            continue;
        }
        if name == "--landscape" || name == "--portrait" {
            if seen_orientation {
                return Err("choose only one page orientation".into());
            }
            orient_page(&mut opts, name);
            seen_orientation = true;
            seen_size_override = true;
            seen_edit = true;
            continue;
        }
        let value = items.next().ok_or_else(|| {
            format!("missing value for {name}; use --help for arguments")
        })?;
        if name == "--profile" {
            if seen_edit || seen_profile {
                return Err("--profile must appear once, before edits".into());
            }
            opts = source_profile_options(value, read_source)?;
            seen_profile = true;
            continue;
        }
        if name == "--output" {
            select_output_path(&mut output, value)?;
            continue;
        }
        if parse_image_export_option(
            name,
            value,
            &mut format,
            &mut dpi,
            &mut seen_format,
            &mut seen_dpi,
            &mut quality_dpi,
        )? {
            continue;
        }
        if name == "--pages" {
            // Page count controls this export, not the saved profile.
            // It may be selected before loading the source profile.
            if seen_pages {
                return Err("--pages may appear only once".into());
            }
            pages = parse_page_count(value)?;
            seen_pages = true;
        } else {
            seen_edit = true;
            apply_paper_edit(
                &mut opts,
                name,
                value,
                &mut seen_size_override,
                &mut seen_preset,
            )?;
        }
    }
    validate_export_flags(format, seen_pages, seen_dpi, emit_profile)?;
    dpi = resolve_png_quality(dpi, quality_dpi, format, seen_dpi)?;
    if replace && output.is_none() {
        return Err("--replace requires --output".into());
    }
    Ok(Some(CliRequest {
        dpi,
        emit_profile,
        format,
        options: opts,
        output,
        pages,
        replace,
    }))
}

#[expect(clippy::redundant_pub_crate, reason = "Sibling CLI regression tests")]
pub(super) fn execute(request: CliRequest) -> Result<Option<Vec<u8>>, String> {
    // Admission and rendering complete before any selected target can change.
    let bytes = if request.emit_profile {
        let _validated = render_pages(request.options, 1)?;
        encode_profile(request.options).into_bytes()
    } else {
        match request.format {
            OutputFormat::Pdf => render_pages(request.options, request.pages)?,
            OutputFormat::Png => render_png(request.options, request.dpi)?,
        }
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
        MAX_PAPER_OUTPUT_BYTES,
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

/// Run the shared Rust paper CLI with caller-provided arguments.
///
/// # Errors
///
/// Invalid parameters or output errors abort before reporting success.
pub fn run_cli_args(args: &[String]) -> Result<(), Box<dyn Error>> {
    let Some(request) = parse_options(args)? else {
        writeln!(io::stdout().lock(), "{USAGE}")?;
        return Ok(());
    };
    if let Some(bytes) = execute(request)? {
        io::stdout().lock().write_all(&bytes)?;
    }
    Ok(())
}

/// Run the standalone paper CLI from process arguments.
///
/// # Errors
///
/// Invalid parameters or output errors abort before reporting success.
pub fn run_cli() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = env::args().skip(1).collect();
    run_cli_args(&args)
}
