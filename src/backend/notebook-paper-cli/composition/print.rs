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
//   - Explicit local CUPS spool submission for a selected PDF and printer.
// - Must-Not:
//   - Print on export, invoke a shell, or select the default printer.
// - Allows:
//   - Inputs: One selected regular PDF file and one named local print queue.
//   - Outputs: Refusal or acknowledgement of CUPS job admission.
//   - Side effects: Bounded source read and one explicitly requested lp job.
// - Split-When:
//   - Additional platform print providers need their own transport adapters.
// - Merge-When:
//   - A dedicated physical-output transport owns local print effects.
// - Summary:
//   - Keeps local printing separate from all Rust PDF/PNG export commands.
// - Description:
//   - Submits exact bounded PDF bytes to a named CUPS queue via stdin.
// - Usage:
//   - atrament-print --pdf notebook.pdf --printer selected-queue
// - Defaults:
//   - No implicit print, default queue, or shell execution.
//

//! Opt-in local PDF printing, separate from profile export and notebook ink.

use std::env;
use std::error::Error;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use atrament_notebook_paper_cli::MAX_PAPER_OUTPUT_BYTES;
use atrament_paper_mark_geometry as _;
use atrament_pdf_composition_plan as _;
use atrament_pdf_vector_serializer as _;
use atrament_physical_page_profile as _;
use atrament_profile_file_commit as _;
use atrament_profile_file_reader::read_bounded_profile_file;
use atrament_profile_json_syntax as _;
use atrament_profile_selected_file_open::open_selected_profile_file;
#[cfg(all(test, unix))]
use nix as _;

const USAGE: &str = "Usage: atrament-print --pdf FILE --printer QUEUE\n\
    Submits one existing PDF to an explicitly named local CUPS printer.\n\
    Exporting a PDF or PNG never sends a print job.";

#[derive(Debug, Eq, PartialEq)]
struct PrintRequest {
    pdf: PathBuf,
    printer: String,
}

fn valid_printer(printer: &str) -> bool {
    printer.len() <= 80
        && printer
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
        && printer.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.')
        })
}

fn parse_args(args: &[String]) -> Result<Option<PrintRequest>, String> {
    if args == ["--help"] {
        return Ok(None);
    }
    let mut pdf = None;
    let mut printer = None;
    let mut items = args.iter();
    while let Some(flag) = items.next() {
        let value = items
            .next()
            .ok_or_else(|| format!("{flag} requires a value; use --help"))?;
        match flag.as_str() {
            "--pdf" if pdf.is_none() && !value.is_empty() => {
                pdf = Some(PathBuf::from(value));
            }
            "--printer" if printer.is_none() && valid_printer(value) => {
                printer = Some(value.clone());
            }
            _ => return Err(format!("invalid or repeated option: {flag}")),
        }
    }
    Ok(Some(PrintRequest {
        pdf: pdf.ok_or("--pdf requires an existing PDF file")?,
        printer: printer.ok_or("--printer requires an explicit CUPS queue")?,
    }))
}

fn selected_pdf(path: &Path) -> Result<Vec<u8>, String> {
    let file = open_selected_profile_file(path)
        .map_err(|error| format!("cannot open selected PDF: {error:?}"))?;
    let source = read_bounded_profile_file(file, MAX_PAPER_OUTPUT_BYTES)
        .map_err(|error| format!("cannot read bounded PDF: {error:?}"))?;
    let bytes = source.bytes();
    let end = bytes.iter().rposition(|byte| !byte.is_ascii_whitespace());
    let has_eof = end
        .and_then(|index| bytes.get(..=index))
        .is_some_and(|prefix| prefix.ends_with(b"%%EOF"));
    if !has_eof
        || !(bytes.starts_with(b"%PDF-1.") || bytes.starts_with(b"%PDF-2."))
    {
        return Err("selected file is not a complete PDF".into());
    }
    Ok(bytes.to_vec())
}

// No shell or arbitrary command options. The exact already-read PDF goes to
// stdin, so a selected path cannot be swapped between validation and spooling.
fn submit_with(
    executable: &Path,
    printer: &str,
    pdf: &[u8],
) -> Result<(), String> {
    let mut child = Command::new(executable)
        .args(["-d", printer, "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("cannot launch local CUPS lp: {error}"))?;
    let written = child
        .stdin
        .take()
        .ok_or("lp stdin unavailable")?
        .write_all(pdf)
        .map_err(|error| format!("cannot supply PDF to lp: {error}"));
    let result = child
        .wait()
        .map_err(|error| format!("cannot await local lp: {error}"))?;
    written?;
    if !result.success() {
        return Err(format!("CUPS refused print submission: {result}"));
    }
    Ok(())
}

fn submit(request: &PrintRequest) -> Result<(), String> {
    let bytes = selected_pdf(&request.pdf)?;
    submit_with(Path::new("/usr/bin/lp"), &request.printer, &bytes)?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    match parse_args(&args)? {
        None => writeln!(io::stdout().lock(), "{USAGE}")?,
        Some(request) => {
            submit(&request)?;
            writeln!(
                io::stdout().lock(),
                "CUPS accepted a job for printer {}",
                request.printer
            )?;
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "../../../../tests/backend/notebook-paper-cli/composition/print.rs"]
mod tests;
