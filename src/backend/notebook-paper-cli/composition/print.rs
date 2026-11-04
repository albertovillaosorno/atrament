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
use std::fmt::{self, Write as _};
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::str;

use atrament_notebook_paper_cli::MAX_PAPER_OUTPUT_BYTES;
use atrament_paper_mark_geometry as _;
use atrament_pdf_composition_plan as _;
use atrament_pdf_vector_serializer as _;
use atrament_physical_page_profile as _;
use atrament_profile_entry_sha256::sha256_profile_bytes;
use atrament_profile_file_commit as _;
use atrament_profile_file_reader::read_bounded_profile_file;
use atrament_profile_json_syntax as _;
use atrament_profile_selected_file_open::open_selected_profile_file;
#[cfg(all(test, unix))]
use nix as _;

const USAGE: &str = concat!(
    "Usage: atrament-print --pdf FILE --printer QUEUE",
    " [--dry-run] [--expect-sha256 64_HEX]\n",
    "Submits one existing PDF to an explicitly named local CUPS printer.\n",
    "Exporting a PDF or PNG never sends a print job.\n",
    "--dry-run displays the exact PDF SHA-256 without contacting CUPS.\n",
    "--expect-sha256 refuses any PDF bytes different from the reviewed file.",
);

#[derive(Clone, Debug, Eq, PartialEq)]
struct PrintRequest {
    dry_run: bool,
    expected_sha256: Option<[u8; 32]>,
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

// Parse an exact 256-bit digest without accepting prefixes, whitespace or
// truncated byte sequences. ASCII hex is case-insensitive on input.
fn parse_sha256(value: &str) -> Result<[u8; 32], String> {
    if value.len() != 64
        || !value.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(
            "--expect-sha256 requires exactly 64 hexadecimal digits".into()
        );
    }
    let mut digest = [0u8; 32];
    for (slot, pair) in digest.iter_mut()
        .zip(value.as_bytes().as_chunks::<2>().0.iter())
    {
        let text = str::from_utf8(pair)
            .map_err(|error| format!("invalid SHA-256 bytes: {error}"))?;
        *slot = u8::from_str_radix(text, 16)
            .map_err(|error| format!("invalid SHA-256 digit: {error}"))?;
    }
    Ok(digest)
}

fn sha256_hex(digest: [u8; 32]) -> Result<String, fmt::Error> {
    let mut text = String::with_capacity(64);
    for byte in digest {
        write!(&mut text, "{byte:02x}")?;
    }
    Ok(text)
}

fn parse_args(args: &[String]) -> Result<Option<PrintRequest>, String> {
    if args == ["--help"] {
        return Ok(None);
    }
    let mut pdf = None;
    let mut printer = None;
    let mut dry_run = false;
    let mut expected_sha256 = None;
    let mut items = args.iter();
    while let Some(flag) = items.next() {
        if flag == "--dry-run" {
            if dry_run {
                return Err("duplicate --dry-run flag".into());
            }
            dry_run = true;
            continue;
        }
        let value = items
            .next()
            .ok_or_else(|| format!("{flag} requires a value; use --help"))?;
        match flag.as_str() {
            "--pdf" if pdf.is_none() && !value.is_empty() => {
                pdf = Some(PathBuf::from(value));
            }
            "--expect-sha256" if expected_sha256.is_none() => {
                expected_sha256 = Some(parse_sha256(value)?);
            }
            "--printer" if printer.is_none() && valid_printer(value) => {
                printer = Some(value.clone());
            }
            _ => return Err(format!("invalid or repeated option: {flag}")),
        }
    }
    Ok(Some(PrintRequest {
        dry_run,
        expected_sha256,
        pdf: pdf.ok_or("--pdf requires an existing PDF file")?,
        printer: printer.ok_or("--printer requires an explicit CUPS queue")?,
    }))
}

// This is deliberately a framing check, not a PDF document parser. Refuse
// ambiguous version headers before any printer process can be contacted.
fn supported_pdf_header(bytes: &[u8]) -> bool {
    bytes.starts_with(b"%PDF-")
        && matches!(bytes.get(5..8),
            Some([b'1', b'.', b'0'..=b'7'] | [b'2', b'.', b'0']))
        && matches!(bytes.get(8), Some(b'\r' | b'\n'))
}

// This is a bounded cross-reference *framing* check, not a PDF parser.
// A valid PDF trailer has a numeric startxref pointer to either an xref
// table or an indirect xref-stream object. A header plus a forged %%EOF
// alone must not reach a printer, even when it looks like a PDF file.
fn supported_pdf_cross_reference(bytes: &[u8]) -> bool {
    let Some(before_eof) = bytes.trim_ascii_end().strip_suffix(b"%%EOF")
    else {
        return false;
    };
    if !matches!(before_eof.last(), Some(b'\r' | b'\n')) {
        return false;
    }
    let Some(marker) = before_eof.windows(b"startxref".len())
        .rposition(|part| part == b"startxref")
    else {
        return false;
    };
    let Some(before_marker) = marker.checked_sub(1) else {
        return false;
    };
    if !matches!(before_eof.get(before_marker), Some(b'\r' | b'\n')) {
        return false;
    }
    let Some(number_start) = marker.checked_add(b"startxref".len())
    else {
        return false;
    };
    let Some(number_line) = before_eof.get(number_start..)
        .and_then(|tail| tail.strip_prefix(b"\r\n")
            .or_else(|| tail.strip_prefix(b"\n"))
            .or_else(|| tail.strip_prefix(b"\r")))
    else {
        return false;
    };
    let number = number_line.trim_ascii();
    if number.is_empty() || !number.iter().all(u8::is_ascii_digit) {
        return false;
    }
    let Some(offset) = str::from_utf8(number).ok()
        .and_then(|digits| digits.parse::<usize>().ok())
    else {
        return false;
    };
    let Some(target) = bytes.get(offset..marker) else {
        return false;
    };
    if target.starts_with(b"xref")
        && target.get(4).is_some_and(u8::is_ascii_whitespace)
    {
        return supported_xref_table(target, bytes);
    }
    // Cross-reference streams entered the PDF standard in version 1.5.
    // Old-format headers cannot authorize a stream even if it appears to
    // have a plausible indirect-object dictionary and terminal locator.
    if !matches!(bytes.get(5..8),
        Some([b'1', b'.', b'5'..=b'7'] | [b'2', b'.', b'0']))
    {
        return false;
    }
    // PDF 1.5+ can point at an xref-stream indirect object instead.
    // Check its nearby dictionary type before the stream body, without
    // assuming that every indirect object is a cross-reference object.
    let Some(prefix) = target.get(..target.len().min(16_384)) else {
        return false;
    };
    xref_stream_header_end(prefix).is_some_and(|start| {
        supported_xref_stream_tail(target, start)
    })
}

// Traditional xref tables have fixed-width decimal entries and a trailer
// dictionary. Do not spool a bare `xref` token or a forged subsection stub.
// This confirms only the terminal table's framing, not offset correctness.
fn xref_word<'bytes>(bytes: &'bytes [u8], at: &mut usize)
    -> Option<&'bytes [u8]> {
    *at = pdf_trivia_end(bytes, *at);
    let start = *at;
    while bytes.get(*at).is_some_and(|byte| {
        !byte.is_ascii_whitespace() && *byte != b'%'
    }) {
        *at = at.checked_add(1)?;
    }
    bytes.get(start..*at).filter(|word| !word.is_empty())
}

// An in-use table entry must identify the numbered indirect object at its
// exact absolute byte position, not just some byte within the PDF. This is
// a narrow header check, not a scan of an object's contents or stream data.
fn in_use_object_header(
    document: &[u8], offset: usize, index: usize, generation: usize,
) -> bool {
    let Some(tail) = document.get(offset..) else { return false; };
    if !tail.first().is_some_and(u8::is_ascii_digit) {
        return false;
    }
    let mut at = 0usize;
    let decimal = |word: &[u8]| {
        str::from_utf8(word).ok()
            .filter(|digits| digits.bytes().all(|byte| byte.is_ascii_digit()))
            .and_then(|digits| digits.parse::<usize>().ok())
    };
    xref_word(tail, &mut at).and_then(decimal) == Some(index)
        && xref_word(tail, &mut at).and_then(decimal) == Some(generation)
        && xref_word(tail, &mut at) == Some(b"obj".as_slice())
}

fn supported_xref_table(bytes: &[u8], document: &[u8]) -> bool {
    let mut at = 0usize;
    if xref_word(bytes, &mut at) != Some(b"xref".as_slice()) {
        return false;
    }
    let mut seen_entry = false;
    let mut minimum_size = 0usize;
    loop {
        let Some(first) = xref_word(bytes, &mut at) else {
            return false;
        };
        if first == b"trailer" {
            return seen_entry && supported_xref_table_trailer(
                bytes.get(at..).unwrap_or_default(), minimum_size,
            );
        }
        let Some(first_index) = str::from_utf8(first).ok()
            .filter(|digits| !digits.is_empty()
                && digits.bytes().all(|byte| byte.is_ascii_digit()))
            .and_then(|digits| digits.parse::<usize>().ok())
        else { return false; };
        let Some(count) = xref_word(bytes, &mut at)
            .and_then(|value| str::from_utf8(value).ok())
            .and_then(|digits| digits.parse::<usize>().ok())
        else {
            return false;
        };
        if count.checked_mul(18)
            .is_none_or(|minimum| minimum > bytes.len()) {
            return false;
        }
        let Some(last_exclusive) = first_index.checked_add(count) else {
            return false;
        };
        minimum_size = minimum_size.max(last_exclusive);
        for index in first_index..last_exclusive {
            let Some(offset) = xref_word(bytes, &mut at) else {
                return false;
            };
            let Some(generation) = xref_word(bytes, &mut at) else {
                return false;
            };
            let Some(status) = xref_word(bytes, &mut at) else {
                return false;
            };
            if offset.len() != 10 || !offset.iter().all(u8::is_ascii_digit)
                || generation.len() != 5
                || !generation.iter().all(u8::is_ascii_digit)
                || !matches!(status, b"f" | b"n")
            {
                return false;
            }
            // Free-table values are object indices rather than byte offsets.
            // An in-use record must name its own indirect object header.
            if status == b"n" {
                let Some(object_offset) = str::from_utf8(offset).ok()
                    .and_then(|digits| digits.parse::<usize>().ok())
                else { return false; };
                let Some(generation_number) = str::from_utf8(generation).ok()
                    .and_then(|digits| digits.parse::<usize>().ok())
                else { return false; };
                if object_offset < 9 || object_offset >= document.len()
                    || !in_use_object_header(
                        document, object_offset, index, generation_number,
                    ) {
                    return false;
                }
            }
            seen_entry = true;
        }
    }
}

// The required trailer /Size is one positive top-level direct decimal.
// It must cover every cross-reference subsection's exclusive upper bound.
// Balance nested dictionaries, arrays and literal/hex strings without
// mistaking a nested /Size or >> text for the terminal dictionary close.
// After that close, only whitespace or PDF comments may precede startxref.
fn supported_xref_table_trailer(bytes: &[u8], minimum_size: usize) -> bool {
    let mut at = pdf_trivia_end(bytes, 0);
    if !bytes.get(at..).is_some_and(|tail| tail.starts_with(b"<<")) {
        return false;
    }
    at = at.saturating_add(2);
    let mut depth = 1usize;
    let mut array_depth = 0usize;
    let mut seen_size = false;
    while at < bytes.len() {
        at = pdf_trivia_end(bytes, at);
        match bytes.get(at) {
            Some(&b'(') => {
                let Some(end) = pdf_literal_end(bytes, at) else {
                    return false;
                };
                at = end;
            }
            Some(&b'<') if bytes.get(at.saturating_add(1)) == Some(&b'<') => {
                depth = depth.saturating_add(1);
                at = at.saturating_add(2);
            }
            Some(&b'<') => {
                let Some(end) = bytes.get(at.saturating_add(1)..)
                    .and_then(|tail| tail.iter().position(|byte| *byte == b'>'))
                else { return false; };
                at = at.saturating_add(end).saturating_add(2);
            }
            Some(&b'>') if bytes.get(at.saturating_add(1)) == Some(&b'>') => {
                depth = depth.saturating_sub(1);
                at = at.saturating_add(2);
                if depth == 0 {
                    return array_depth == 0 && seen_size
                        && pdf_trivia_end(bytes, at) == bytes.len();
                }
            }
            Some(&b'>') => return false,
            Some(&b'[') => {
                array_depth = array_depth.saturating_add(1);
                at = at.saturating_add(1);
            }
            Some(&b']') => {
                if array_depth == 0 {
                    return false;
                }
                array_depth = array_depth.saturating_sub(1);
                at = at.saturating_add(1);
            }
            Some(&b'/') => {
                let Some((name, end)) = pdf_name(bytes, at) else {
                    return false;
                };
                at = end;
                if depth == 1 && array_depth == 0 && name == b"Size" {
                    if seen_size {
                        return false;
                    }
                    at = pdf_trivia_end(bytes, at);
                    let start = at;
                    while bytes.get(at).is_some_and(u8::is_ascii_digit) {
                        at = at.saturating_add(1);
                    }
                    let value = bytes.get(start..at).unwrap_or_default();
                    if value.is_empty() || str::from_utf8(value).ok()
                        .and_then(|digits| digits.parse::<usize>().ok())
                        .is_none_or(|number| {
                            number == 0 || number < minimum_size
                        })
                        || !bytes.get(at).is_some_and(|byte| {
                            byte.is_ascii_whitespace()
                                || matches!(byte, b'>' | b'/' | b'%')
                        }) {
                        return false;
                    }
                    seen_size = true;
                }
            }
            Some(_) => at = at.saturating_add(1),
            None => break,
        }
    }
    false
}

// These small lexical checks intentionally do not parse PDF object contents
// or validate stream data. They only keep comments, strings, nested maps and
// coincidental substrings from impersonating the top-level xref dictionary.
fn pdf_name(bytes: &[u8], start: usize) -> Option<(&[u8], usize)> {
    if bytes.get(start) != Some(&b'/') {
        return None;
    }
    let first = start.checked_add(1)?;
    let mut end = first;
    while bytes.get(end).is_some_and(|byte| {
        !byte.is_ascii_whitespace() && !b"()<>[]{}/%".contains(byte)
    }) {
        end = end.saturating_add(1);
    }
    if end == first {
        return None;
    }
    bytes.get(first..end).map(|name| (name, end))
}

fn pdf_trivia_end(bytes: &[u8], mut at: usize) -> usize {
    loop {
        while bytes.get(at).is_some_and(u8::is_ascii_whitespace) {
            at = at.saturating_add(1);
        }
        if bytes.get(at) != Some(&b'%') {
            return at;
        }
        while let Some(byte) = bytes.get(at) {
            at = at.saturating_add(1);
            if matches!(byte, b'\r' | b'\n') {
                break;
            }
        }
    }
}

fn pdf_literal_end(bytes: &[u8], start: usize) -> Option<usize> {
    let mut depth = 1usize;
    let mut at = start.checked_add(1)?;
    while let Some(byte) = bytes.get(at) {
        match byte {
            b'\\' => at = at.checked_add(2)?,
            b'(' => {
                depth = depth.saturating_add(1);
                at = at.saturating_add(1);
            }
            b')' => {
                depth = depth.saturating_sub(1);
                at = at.saturating_add(1);
                if depth == 0 {
                    return Some(at);
                }
            }
            _ => at = at.saturating_add(1),
        }
    }
    None
}

fn xref_stream_header_end(prefix: &[u8]) -> Option<usize> {
    let mut at = 0usize;
    for _ in 0u8..2 {
        at = pdf_trivia_end(prefix, at);
        let start = at;
        while prefix.get(at).is_some_and(u8::is_ascii_digit) {
            at = at.saturating_add(1);
        }
        if start == at || !prefix.get(at)
            .is_some_and(u8::is_ascii_whitespace)
        {
            return None;
        }
    }
    at = pdf_trivia_end(prefix, at);
    if !prefix.get(at..).is_some_and(|tail| tail.starts_with(b"obj"))
        || !prefix.get(at.saturating_add(3))
            .is_some_and(u8::is_ascii_whitespace)
    {
        return None;
    }
    at = pdf_trivia_end(prefix, at.saturating_add(3));
    if !prefix.get(at..).is_some_and(|tail| tail.starts_with(b"<<")) {
        return None;
    }
    at = at.saturating_add(2);
    let mut depth = 1usize;
    let mut array_depth = 0usize;
    let mut found_type = false;
    while at < prefix.len() {
        match prefix.get(at) {
            Some(&b'%') => at = pdf_trivia_end(prefix, at),
            Some(&b'(') => at = pdf_literal_end(prefix, at)?,
            Some(&b'<') if prefix.get(at.saturating_add(1)) == Some(&b'<') => {
                depth = depth.saturating_add(1);
                at = at.saturating_add(2);
            }
            Some(&b'<') => {
                let end = prefix.get(at.saturating_add(1)..)
                    .and_then(|tail| {
                        tail.iter().position(|byte| *byte == b'>')
                    })?;
                at = at.saturating_add(end).saturating_add(2);
            }
            Some(&b'>') if prefix.get(at.saturating_add(1)) == Some(&b'>') => {
                depth = depth.saturating_sub(1);
                at = at.saturating_add(2);
                if depth == 0 {
                    break;
                }
            }
            Some(&b'>') => return None,
            Some(&b'[') => {
                array_depth = array_depth.saturating_add(1);
                at = at.saturating_add(1);
            }
            Some(&b']') => {
                if array_depth == 0 {
                    return None;
                }
                array_depth = array_depth.saturating_sub(1);
                at = at.saturating_add(1);
            }
            Some(&b'/') => {
                let (name, end) = pdf_name(prefix, at)?;
                at = end;
                if depth == 1 && array_depth == 0 && name == b"Type" {
                    if found_type {
                        return None;
                    }
                    at = pdf_trivia_end(prefix, at);
                    let (value, value_end) = pdf_name(prefix, at)?;
                    if value != b"XRef" {
                        return None;
                    }
                    found_type = true;
                    at = value_end;
                }
            }
            _ => at = at.saturating_add(1),
        }
    }
    if depth != 0 || array_depth != 0 || !found_type {
        return None;
    }
    at = pdf_trivia_end(prefix, at);
    if prefix.get(at..).is_some_and(|tail| {
        tail.starts_with(b"stream")
            && matches!(tail.get(6), Some(b'\n' | b'\r'))
    }) {
        at.checked_add(6)
    } else {
        None
    }
}

// A syntactically plausible xref-stream header is not sufficient: a selected
// document must also frame its terminal stream and indirect object before
// startxref. This remains lexical preflight, not PDF stream decoding.
fn supported_xref_stream_tail(bytes: &[u8], start: usize) -> bool {
    let Some(tail) = bytes.get(start..) else { return false; };
    let Some(endstream) = tail.windows(b"endstream".len())
        .enumerate().rfind(|(at, token)| {
            *token == b"endstream"
                && at.checked_sub(1).and_then(|before| tail.get(before))
                    .is_some_and(|byte| matches!(byte, b'\r' | b'\n'))
                && tail.get(at.saturating_add(b"endstream".len()))
                    .is_some_and(u8::is_ascii_whitespace)
        }).map(|(at, _)| at)
    else {
        return false;
    };
    let Some(endstream_end) = start.checked_add(endstream)
        .and_then(|at| at.checked_add(b"endstream".len()))
    else {
        return false;
    };
    let endobj = pdf_trivia_end(bytes, endstream_end);
    if !bytes.get(endobj..).is_some_and(|rest| rest.starts_with(b"endobj"))
        || !bytes.get(endobj.saturating_add(b"endobj".len()))
            .is_some_and(u8::is_ascii_whitespace) {
        return false;
    }
    pdf_trivia_end(bytes, endobj.saturating_add(b"endobj".len()))
        == bytes.len()
}

fn selected_pdf(path: &Path) -> Result<Vec<u8>, String> {
    let file = open_selected_profile_file(path)
        .map_err(|error| format!("cannot open selected PDF: {error:?}"))?;
    let source = read_bounded_profile_file(file, MAX_PAPER_OUTPUT_BYTES)
        .map_err(|error| format!("cannot read bounded PDF: {error:?}"))?;
    let bytes = source.bytes();
    if !supported_pdf_header(bytes) || !supported_pdf_cross_reference(bytes) {
        return Err(concat!(
            "selected PDF needs a supported version header, ",
            "cross-reference startxref and terminal %%EOF",
        ).into());
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

fn submit_selected(
    request: &PrintRequest,
    executable: &Path,
) -> Result<[u8; 32], String> {
    let bytes = selected_pdf(&request.pdf)?;
    let digest = sha256_profile_bytes(&bytes).bytes();
    if request.expected_sha256.is_some_and(|expected| expected != digest) {
        return Err("selected PDF SHA-256 differs from reviewed bytes".into());
    }
    if !request.dry_run {
        submit_with(executable, &request.printer, &bytes)?;
    }
    Ok(digest)
}

fn submit(request: &PrintRequest) -> Result<[u8; 32], String> {
    submit_selected(request, Path::new("/usr/bin/lp"))
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    match parse_args(&args)? {
        None => writeln!(io::stdout().lock(), "{USAGE}")?,
        Some(request) => {
            let digest = submit(&request)?;
            if request.dry_run {
                writeln!(
                    io::stdout().lock(),
                    "PDF validated for {}; SHA-256 {}; no job submitted",
                    request.printer, sha256_hex(digest)?
                )?;
            } else {
                writeln!(
                    io::stdout().lock(),
                    "CUPS accepted a job for printer {}",
                    request.printer
                )?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "../../../../tests/backend/notebook-paper-cli/composition/print.rs"]
mod tests;
