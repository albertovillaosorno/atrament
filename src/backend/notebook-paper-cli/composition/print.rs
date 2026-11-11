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
use std::thread;
use std::time::{Duration, Instant};

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

// An external PDF can claim a sparse multi-billion-object table in a few
// bytes. Bound downstream reader allocation exposure against the same
// 24-MB file admission budget (minimum traditional record: 18 bytes).
const MAX_SUPPORTED_XREF_OBJECTS: usize = MAX_PAPER_OUTPUT_BYTES.div_ceil(18);
const CUPS_SUBMISSION_TIMEOUT: Duration = Duration::from_secs(60);

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
            "--pdf" if pdf.is_none() && !value.is_empty()
                && !value.starts_with('-') => {
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

// PDF 1.7 whitespace is NUL, HT, LF, FF, CR or space. Rust's ASCII
// whitespace excludes NUL and includes vertical tab, so it cannot be used
// for PDF tokens, trailer metadata or terminal EOF trivia.
const fn pdf_whitespace(byte: u8) -> bool {
    matches!(byte, b'\0' | b'\t' | b'\n' | b'\x0c' | b'\r' | b' ')
}

fn trim_pdf_end(bytes: &[u8]) -> &[u8] {
    let end = bytes.iter().rposition(|byte| !pdf_whitespace(*byte))
        .map_or(0, |last| last.saturating_add(1));
    bytes.get(..end).unwrap_or_default()
}

// A numeric locator can have PDF whitespace on both sides of its digits.
fn trim_pdf(bytes: &[u8]) -> &[u8] {
    let start = bytes.iter().position(|byte| !pdf_whitespace(*byte))
        .unwrap_or(bytes.len());
    trim_pdf_end(bytes.get(start..).unwrap_or_default())
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
    let Some(before_eof) = trim_pdf_end(bytes).strip_suffix(b"%%EOF")
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
    let number = trim_pdf(number_line);
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
        && target.get(4).is_some_and(|byte| pdf_whitespace(*byte))
    {
        return supported_xref_table(target, bytes, offset);
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
    xref_stream_header_end(prefix, Some((bytes, offset)), offset)
        .is_some_and(|(start, length, _)| {
            supported_xref_stream_tail(target, start, length)
        })
}

// Traditional xref tables have fixed-width decimal entries and a trailer
// dictionary. Do not spool a bare `xref` token or a forged subsection stub.
// This confirms only the terminal table's framing, not offset correctness.
// PDF delimiters terminate a keyword without requiring intervening space:
// a direct `trailer<<` dictionary is still a legal token boundary.
fn xref_word<'bytes>(bytes: &'bytes [u8], at: &mut usize)
    -> Option<&'bytes [u8]> {
    *at = pdf_trivia_end(bytes, *at);
    let start = *at;
    while bytes.get(*at).is_some_and(|byte| {
        !pdf_whitespace(*byte) && !b"()<>[]{}/%".contains(byte)
    }) {
        *at = at.checked_add(1)?;
    }
    bytes.get(start..*at).filter(|word| !word.is_empty())
}

// The xref keyword starts its own table line. It cannot share a row with
// an index/count, even if the later tokens are individually well-formed.
fn xref_table_keyword(bytes: &[u8], at: &mut usize) -> bool {
    if xref_word(bytes, at) != Some(b"xref".as_slice()) {
        return false;
    }
    let end = pdf_trivia_end(bytes, *at);
    bytes.get(*at..end).is_some_and(|gap| gap.iter()
        .any(|byte| matches!(byte, b'\n' | b'\r')))
}

// Subsection index and count form one physical line with horizontal-space
// separation. Token-by-token trivia skipping would accept a forged two-line
// header, including when validating the immediate /Prev table predecessor.
fn xref_subsection_header(
    bytes: &[u8], at: &mut usize, first: &[u8],
) -> Option<(usize, usize)> {
    let decimal = |word: &[u8]| {
        str::from_utf8(word).ok()
            .filter(|value| !value.is_empty()
                && value.bytes().all(|byte| byte.is_ascii_digit()))
            .and_then(|value| value.parse::<usize>().ok())
    };
    let first_index = decimal(first)?;
    let after_first = *at;
    let count_word = xref_word(bytes, at)?;
    let count = decimal(count_word)?;
    let count_start = at.checked_sub(count_word.len())?;
    let separation = bytes.get(after_first..count_start)?;
    if separation.is_empty()
        || !separation.iter().all(|byte| matches!(byte, b' ' | b'\t')) {
        return None;
    }
    Some((first_index, count))
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

// Table records have three fields on one terminated physical line.
// Word-by-word trivia skipping would otherwise splice a forged record from
// multiple rows or admit unrelated tokens after its status byte.
fn xref_table_entry<'bytes>(
    bytes: &'bytes [u8], at: &mut usize,
) -> Option<(&'bytes [u8], &'bytes [u8], &'bytes [u8])> {
    let start = pdf_trivia_end(bytes, *at);
    // A record must begin on a new line, not share its subsection header.
    if !bytes.get(*at..start)?.iter()
        .any(|byte| matches!(byte, b'\r' | b'\n')) {
        return None;
    }
    let tail = bytes.get(start..)?;
    let end = tail.iter().position(|byte| matches!(byte, b'\r' | b'\n'))?;
    let line = tail.get(..end)?;
    let mut words = line.split(|byte| matches!(byte, b' ' | b'\t'))
        .filter(|word| !word.is_empty());
    let offset = words.next()?;
    let generation = words.next()?;
    let status = words.next()?;
    if words.next().is_some() {
        return None;
    }
    *at = start.checked_add(end)?;
    Some((offset, generation, status))
}

fn supported_xref_table(
    bytes: &[u8], document: &[u8], table_start: usize,
) -> bool {
    let mut at = 0usize;
    if !xref_table_keyword(bytes, &mut at) {
        return false;
    }
    let mut seen_entry = false;
    let mut minimum_size = 0usize;
    let mut previous_subsection_end = 0usize;
    loop {
        let Some(first) = xref_word(bytes, &mut at) else {
            return false;
        };
        if first == b"trailer" {
            return seen_entry && supported_xref_table_trailer(
                bytes.get(at..).unwrap_or_default(), bytes, minimum_size,
                document, table_start,
            );
        }
        let Some((first_index, count)) =
            xref_subsection_header(bytes, &mut at, first)
        else { return false; };
        if count.checked_mul(18)
            .is_none_or(|minimum| minimum > bytes.len()) {
            return false;
        }
        // One terminal table cannot redefine an earlier object's record.
        // Require disjoint increasing subsection ranges, without allocation;
        // sparse gaps remain supported.
        if count == 0 || first_index < previous_subsection_end {
            return false;
        }
        let Some(last_exclusive) = first_index.checked_add(count) else {
            return false;
        };
        previous_subsection_end = last_exclusive;
        minimum_size = minimum_size.max(last_exclusive);
        for index in first_index..last_exclusive {
            let Some((offset, generation, status)) =
                xref_table_entry(bytes, &mut at)
            else { return false; };
            if offset.len() != 10 || !offset.iter().all(u8::is_ascii_digit)
                || generation.len() != 5
                || !generation.iter().all(u8::is_ascii_digit)
                || !matches!(status, b"f" | b"n")
            {
                return false;
            }
            // The five-digit generation is a PDF 16-bit counter.
            let Some(generation_number) = str::from_utf8(generation).ok()
                .and_then(|digits| digits.parse::<u16>().ok())
            else { return false; };
            // Object zero is the permanent free-list head in PDF tables.
            // Its generation is reserved and must always be 65535.
            if index == 0 && (status != b"f"
                || generation_number != u16::MAX) {
                return false;
            }
            if status == b"f" {
                // Free-table offsets name the next free object, not a byte.
                // The trailer's /Size must also cover this reference.
                let Some(next_free) = str::from_utf8(offset).ok()
                    .and_then(|digits| digits.parse::<usize>().ok())
                    .and_then(|value| value.checked_add(1))
                else { return false; };
                minimum_size = minimum_size.max(next_free);
            } else {
                // An in-use record must name its own indirect object.
                let Some(object_offset) = str::from_utf8(offset).ok()
                    .and_then(|digits| digits.parse::<usize>().ok())
                else { return false; };
                // Terminal xref records cannot refer forward into their
                // own subsection, trailer or its metadata strings.
                if object_offset < 9 || object_offset >= table_start
                    || !in_use_object_header(
                        document, object_offset, index,
                        usize::from(generation_number),
                    ) {
                    return false;
                }
            }
            seen_entry = true;
        }
    }
}

// A declared cross-reference object count must fit the bounded local PDF
// input policy. Both tables and streams use this lexical numeric boundary.
fn bounded_xref_size(
    bytes: &[u8], at: &mut usize, minimum_size: usize,
) -> Option<usize> {
    *at = pdf_trivia_end(bytes, *at);
    let start = *at;
    while bytes.get(*at).is_some_and(u8::is_ascii_digit) {
        *at = at.saturating_add(1);
    }
    let value = bytes.get(start..*at)?;
    let number = str::from_utf8(value).ok()?.parse::<usize>().ok()?;
    if number == 0 || number < minimum_size
        || number > MAX_SUPPORTED_XREF_OBJECTS
        || !bytes.get(*at).is_some_and(|byte| {
            pdf_whitespace(*byte)
                || matches!(byte, b'>' | b'/' | b'%')
        }) {
        return None;
    }
    Some(number)
}

// A prior traditional xref pointer must identify a complete bounded first
// subsection, not a bare `xref` token or one record from a truncated range.
// This remains a nonrecursive probe: it does not validate older trailers
// or the remaining sections of an earlier revision.
fn previous_xref_table_header(
    bytes: &[u8], document: &[u8], table_start: usize,
) -> bool {
    let mut at = 0usize;
    if !xref_table_keyword(bytes, &mut at) {
        return false;
    }
    let Some(first_word) = xref_word(bytes, &mut at) else {
        return false;
    };
    let Some((first, count)) =
        xref_subsection_header(bytes, &mut at, first_word)
    else { return false; };
    let Some(last) = first.checked_add(count) else {
        return false;
    };
    if count == 0 || last > MAX_SUPPORTED_XREF_OBJECTS
        || count.checked_mul(18).is_none_or(|size| size > bytes.len()) {
        return false;
    }
    for index in first..last {
        let Some((offset, generation, status)) =
            xref_table_entry(bytes, &mut at)
        else { return false; };
        if offset.len() != 10 || !offset.iter().all(u8::is_ascii_digit)
            || generation.len() != 5
            || !matches!(status, b"f" | b"n") {
            return false;
        }
        let Some(generation_number) = str::from_utf8(generation).ok()
            .and_then(|digits| digits.parse::<u16>().ok())
        else { return false; };
        if index == 0 && (status != b"f"
            || generation_number != u16::MAX) {
            return false;
        }
        if status == b"f" {
            // Earlier free-list heads must obey the same global object
            // budget as terminal xref tables; the older /Size is not read.
            let Some(next_free_exclusive) = str::from_utf8(offset).ok()
                .and_then(|digits| digits.parse::<usize>().ok())
                .and_then(|next| next.checked_add(1))
            else { return false; };
            if next_free_exclusive > MAX_SUPPORTED_XREF_OBJECTS {
                return false;
            }
        } else {
            let Some(object_offset) = str::from_utf8(offset).ok()
                .and_then(|digits| digits.parse::<usize>().ok())
            else { return false; };
            if object_offset < 9 || object_offset >= table_start
                || !in_use_object_header(document, object_offset, index,
                    usize::from(generation_number)) {
                return false;
            }
        }
    }
    true
}

// Both revision and hybrid references must contain a direct decimal
// byte offset to an earlier section, never a forward or self reference.
fn bounded_xref_pointer(
    bytes: &[u8], at: &mut usize, table_start: usize,
) -> Option<usize> {
    *at = pdf_trivia_end(bytes, *at);
    let start = *at;
    while bytes.get(*at).is_some_and(u8::is_ascii_digit) {
        *at = at.saturating_add(1);
    }
    let offset = bytes.get(start..*at)
        .and_then(|digits| str::from_utf8(digits).ok())
        .and_then(|digits| digits.parse::<usize>().ok())?;
    if offset < 9 || offset >= table_start
        || !bytes.get(*at).is_some_and(|byte| {
            pdf_whitespace(*byte)
                || matches!(byte, b'>' | b'/' | b'%')
        }) {
        return None;
    }
    Some(offset)
}

// When a hybrid stream's explicit /Index excludes its own object, the
// enclosing traditional table must own that exact indirect-object offset.
// The caller already validated each table row against the object header;
// this second bounded scan avoids retaining potentially millions of rows.
fn xref_table_owns_stream(table: &[u8], stream_offset: usize) -> bool {
    let mut at = 0usize;
    if !xref_table_keyword(table, &mut at) {
        return false;
    }
    loop {
        let Some(first) = xref_word(table, &mut at) else {
            return false;
        };
        if first == b"trailer" {
            return false;
        }
        let Some((_, count)) = xref_subsection_header(table, &mut at, first)
        else { return false; };
        for _ in 0..count {
            let Some((offset, _, status)) = xref_table_entry(table, &mut at)
            else { return false; };
            if status == b"n" && str::from_utf8(offset).ok()
                .and_then(|digits| digits.parse::<usize>().ok())
                == Some(stream_offset) {
                return true;
            }
        }
    }
}

// A traditional table's /XRefStm hybrid reference must name a complete
// earlier xref stream, not arbitrary bytes or a traditional table. This
// bounds its exact stream frame without parsing its compressed contents.
fn bounded_hybrid_xref_stream(
    bytes: &[u8], at: &mut usize, table: &[u8], document: &[u8],
    table_start: usize,
) -> bool {
    if !matches!(document.get(5..8),
        Some([b'1', b'.', b'5'..=b'7'] | [b'2', b'.', b'0'])) {
        return false;
    }
    let Some(offset) = bounded_xref_pointer(bytes, at, table_start)
    else { return false; };
    let Some(stream) = document.get(offset..table_start) else {
        return false;
    };
    stream.get(..stream.len().min(16_384))
        .and_then(|prefix| xref_stream_header_end(prefix, None, offset))
        .is_some_and(|(start, length, needs_table_owner)| {
            (!needs_table_owner || xref_table_owns_stream(table, offset))
                && xref_stream_frame_end(stream, start, length).is_some()
        })
}

// /Prev, when present on a terminal traditional table, must lead back to
// an earlier cross-reference record rather than to arbitrary data, a self
// reference or a forged future byte offset. This is not recursive parsing
// of older revisions; it only authenticates the immediate target's header.
fn bounded_previous_xref(
    bytes: &[u8], at: &mut usize, document: &[u8], table_start: usize,
) -> bool {
    let Some(offset) = bounded_xref_pointer(bytes, at, table_start)
    else { return false; };
    let Some(earlier) = document.get(offset..table_start) else {
        return false;
    };
    if earlier.starts_with(b"xref")
        && earlier.get(4).is_some_and(|byte| pdf_whitespace(*byte)) {
        return previous_xref_table_header(earlier, document, offset);
    }
    // A PDF 1.5+ incremental revision may refer back to an xref stream.
    let stream_edition = matches!(document.get(5..8),
        Some([b'1', b'.', b'5'..=b'7'] | [b'2', b'.', b'0']));
    stream_edition && earlier.get(..earlier.len().min(16_384))
        .and_then(|prefix| xref_stream_header_end(prefix, None, offset))
        .is_some_and(|(frame_start, length, _)| {
            xref_stream_frame_end(earlier, frame_start, length).is_some()
        })
}

// The required trailer /Size is one positive top-level direct decimal.
// It must cover every cross-reference subsection's exclusive upper bound.
// Balance nested dictionaries, arrays and literal/hex strings without
// mistaking a nested /Size or >> text for the terminal dictionary close.
// After that close, only whitespace or PDF comments may precede startxref.
fn supported_xref_table_trailer(
    bytes: &[u8], table: &[u8], minimum_size: usize,
    document: &[u8], table_start: usize,
) -> bool {
    let mut at = pdf_trivia_end(bytes, 0);
    if !bytes.get(at..).is_some_and(|tail| tail.starts_with(b"<<")) {
        return false;
    }
    at = at.saturating_add(2);
    let mut depth = 1usize;
    let mut array_depth = 0usize;
    let mut seen_size = false;
    let mut seen_prev = false;
    let mut seen_hybrid = false;
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
                let Some(end) = pdf_hex_end(bytes, at) else {
                    return false;
                };
                at = end;
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
                if depth == 1 && array_depth == 0
                    && pdf_name_is(name, b"Size") {
                    if seen_size {
                        return false;
                    }
                    if bounded_xref_size(
                        bytes, &mut at, minimum_size,
                    ).is_none() {
                        return false;
                    }
                    seen_size = true;
                } else if depth == 1 && array_depth == 0
                    && pdf_name_is(name, b"Prev") {
                    if seen_prev || !bounded_previous_xref(
                        bytes, &mut at, document, table_start,
                    ) {
                        return false;
                    }
                    seen_prev = true;
                } else if depth == 1 && array_depth == 0
                    && pdf_name_is(name, b"XRefStm") {
                    if seen_hybrid || !bounded_hybrid_xref_stream(
                        bytes, &mut at, table, document, table_start,
                    ) {
                        return false;
                    }
                    seen_hybrid = true;
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
        !pdf_whitespace(*byte) && !b"()<>[]{}/%".contains(byte)
    }) {
        // Every # begins a two-digit hex escape, including in unknown
        // keys. Do not let malformed PDF names bypass lexical preflight.
        if bytes.get(end) == Some(&b'#')
            && !bytes.get(end.saturating_add(1)..end.saturating_add(3))
                .is_some_and(|pair| pair.len() == 2
                    && pair.iter().all(u8::is_ascii_hexdigit)) {
            return None;
        }
        end = end.saturating_add(1);
    }
    if end == first {
        return None;
    }
    bytes.get(first..end).map(|name| (name, end))
}

// PDF names may spell bytes with # followed by two hex digits. Normalize
// comparisons without allocations so an escaped alias cannot evade a
// duplicate required-key check (for example /Size and /S#69ze).
fn pdf_name_is(name: &[u8], literal: &[u8]) -> bool {
    let mut at = 0usize;
    for expected in literal {
        let Some(&first) = name.get(at) else { return false; };
        let byte = if first == b'#' {
            let Some(pair) = name.get(at.saturating_add(1)
                ..at.saturating_add(3)) else { return false; };
            let Some(hex) = str::from_utf8(pair).ok() else {
                return false;
            };
            let Some(decoded) = u8::from_str_radix(hex, 16).ok() else {
                return false;
            };
            at = at.saturating_add(3);
            decoded
        } else {
            at = at.saturating_add(1);
            first
        };
        if byte != *expected {
            return false;
        }
    }
    at == name.len()
}

fn pdf_trivia_end(bytes: &[u8], mut at: usize) -> usize {
    loop {
        while bytes.get(at).is_some_and(|byte| pdf_whitespace(*byte)) {
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

// PDF hexadecimal string contents are hex digits and PDF whitespace.
// An odd last nibble is legal (implicitly padded with zero), but arbitrary
// punctuation or a second opening delimiter cannot be skipped as metadata.
// Use the same bounded lexical check in table trailers and xref streams.
fn pdf_hex_end(bytes: &[u8], start: usize) -> Option<usize> {
    if bytes.get(start) != Some(&b'<')
        || bytes.get(start.checked_add(1)?) == Some(&b'<') {
        return None;
    }
    let mut at = start.checked_add(1)?;
    while let Some(byte) = bytes.get(at) {
        if *byte == b'>' {
            return at.checked_add(1);
        }
        if !byte.is_ascii_hexdigit() && !pdf_whitespace(*byte) {
            return None;
        }
        at = at.checked_add(1)?;
    }
    None
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

// Stream contents have a byte length, not an unbounded endstream scan.
// Require one direct bounded integer in this conservative print preflight.
fn bounded_stream_length(bytes: &[u8], at: &mut usize) -> Option<usize> {
    *at = pdf_trivia_end(bytes, *at);
    let start = *at;
    while bytes.get(*at).is_some_and(u8::is_ascii_digit) {
        *at = at.saturating_add(1);
    }
    let length = str::from_utf8(bytes.get(start..*at)?)
        .ok()?.parse::<usize>().ok()?;
    if length == 0 || length > MAX_PAPER_OUTPUT_BYTES
        || !bytes.get(*at).is_some_and(|byte| {
            pdf_whitespace(*byte)
                || matches!(byte, b'>' | b'/' | b'%')
        }) {
        return None;
    }
    Some(length)
}

// A cross-reference stream cannot be decoded without its /W triplet.
// Require three bounded, direct, unsigned integer field widths. This is a
// structural preflight only: compressed data and /Index rows are not decoded.
fn xref_stream_field_widths(bytes: &[u8], at: &mut usize)
    -> Option<usize> {
    *at = pdf_trivia_end(bytes, *at);
    if bytes.get(*at) != Some(&b'[') {
        return None;
    }
    *at = at.checked_add(1)?;
    let mut widths = [0usize; 3];
    for width in &mut widths {
        *at = pdf_trivia_end(bytes, *at);
        let start = *at;
        while bytes.get(*at).is_some_and(u8::is_ascii_digit) {
            *at = at.checked_add(1)?;
        }
        let digits = bytes.get(start..*at)?;
        *width = str::from_utf8(digits).ok()?.parse::<usize>().ok()?;
        // The scanner admits at most eight bytes for any object field.
        // Larger values require a reviewed re-export under local policy.
        if *width > 8 || !bytes.get(*at).is_some_and(|byte| {
            pdf_whitespace(*byte) || matches!(byte, b']' | b'%')
        }) {
            return None;
        }
    }
    *at = pdf_trivia_end(bytes, *at);
    if bytes.get(*at) != Some(&b']')
        || widths.iter().all(|width| *width == 0) {
        return None;
    }
    *at = at.checked_add(1)?;
    Some(widths.iter().sum())
}

// /Filter on an encoded xref stream names a decoder, or a nonempty array
// of decoder names. Without it, /Length must describe complete raw rows.
fn xref_stream_filter(bytes: &[u8], at: &mut usize) -> Option<()> {
    *at = pdf_trivia_end(bytes, *at);
    if bytes.get(*at) == Some(&b'/') {
        *at = pdf_name(bytes, *at)?.1;
        return Some(());
    }
    if bytes.get(*at) != Some(&b'[') {
        return None;
    }
    *at = at.checked_add(1)?;
    let mut seen = false;
    loop {
        *at = pdf_trivia_end(bytes, *at);
        if bytes.get(*at) == Some(&b']') {
            if !seen {
                return None;
            }
            *at = at.checked_add(1)?;
            return Some(());
        }
        *at = pdf_name(bytes, *at)?.1;
        seen = true;
    }
}

// /Index is optional, but when present it must enumerate nonempty,
// increasing disjoint pairs of object start and count within /Size.
// A terminal standalone stream must index itself. A hybrid stream may
// instead have its own object entry in the surrounding traditional table.
// This checks coverage and row count without decoding the stream.
fn xref_stream_index_end(
    bytes: &[u8], at: &mut usize, object_number: usize,
) -> Option<(usize, bool, usize)> {
    *at = pdf_trivia_end(bytes, *at);
    if bytes.get(*at) != Some(&b'[') {
        return None;
    }
    *at = at.checked_add(1)?;
    let mut previous_end = 0usize;
    let mut seen_range = false;
    let mut contains_self = false;
    let mut row_count = 0usize;
    loop {
        *at = pdf_trivia_end(bytes, *at);
        if bytes.get(*at) == Some(&b']') {
            if !seen_range {
                return None;
            }
            *at = at.checked_add(1)?;
            return Some((previous_end, contains_self, row_count));
        }
        let first = xref_stream_index_integer(bytes, at)?;
        let count = xref_stream_index_integer(bytes, at)?;
        let end = first.checked_add(count)?;
        if count == 0 || first < previous_end
            || end > MAX_SUPPORTED_XREF_OBJECTS {
            return None;
        }
        contains_self |= (first..end).contains(&object_number);
        row_count = row_count.checked_add(count)?;
        previous_end = end;
        seen_range = true;
    }
}

fn xref_stream_index_integer(bytes: &[u8], at: &mut usize)
    -> Option<usize> {
    *at = pdf_trivia_end(bytes, *at);
    let start = *at;
    while bytes.get(*at).is_some_and(u8::is_ascii_digit) {
        *at = at.checked_add(1)?;
    }
    let digits = bytes.get(start..*at)?;
    let value = str::from_utf8(digits).ok()?.parse::<usize>().ok()?;
    if !bytes.get(*at).is_some_and(|byte| {
        pdf_whitespace(*byte) || matches!(byte, b']' | b'%')
    }) {
        return None;
    }
    Some(value)
}

// The indirect xref-stream object must use a real nonzero object number
// and a 16-bit generation; its number is compared against declared /Size.
fn xref_stream_object_header(prefix: &[u8]) -> Option<(usize, usize)> {
    let mut at = 0usize;
    let mut object_numbers = [0usize; 2];
    for number in &mut object_numbers {
        at = pdf_trivia_end(prefix, at);
        let start = at;
        while prefix.get(at).is_some_and(u8::is_ascii_digit) {
            at = at.saturating_add(1);
        }
        if start == at || !prefix.get(at)
            .is_some_and(|byte| pdf_whitespace(*byte))
        {
            return None;
        }
        *number = str::from_utf8(prefix.get(start..at)?)
            .ok()?.parse::<usize>().ok()?;
    }
    // Object zero is the free-list head. Generation is a PDF u16 value.
    if object_numbers[0] == 0 || object_numbers[1] > u16::MAX.into() {
        return None;
    }
    at = pdf_trivia_end(prefix, at);
    if !prefix.get(at..).is_some_and(|tail| tail.starts_with(b"obj"))
        || !prefix.get(at.saturating_add(3))
            .is_some_and(|byte| pdf_whitespace(*byte))
    {
        return None;
    }
    at = pdf_trivia_end(prefix, at.saturating_add(3));
    Some((at, object_numbers[0]))
}

// Earlier stream headers must carry a direct bounded /Prev value, but
// only a terminal stream follows that pointer one additional revision.
fn bounded_stream_previous(
    prefix: &[u8], at: &mut usize,
    previous: Option<(&[u8], usize)>, stream_start: usize,
) -> bool {
    match previous {
        Some((document, start)) => bounded_previous_xref(
            prefix, at, document, start,
        ),
        None => bounded_xref_pointer(
            prefix, at, stream_start,
        ).is_some(),
    }
}

// The stream keyword is followed by a physical line ending before its
// exact encoded bytes. Keep this framing probe separate from dictionary
// admission so both checks remain small and bounded.
fn xref_stream_data_start(prefix: &[u8], at: usize) -> Option<usize> {
    let start = pdf_trivia_end(prefix, at);
    let tail = prefix.get(start..)?;
    if !tail.starts_with(b"stream")
        || !matches!(tail.get(6), Some(b'\n' | b'\r')) {
        return None;
    }
    start.checked_add(6)
}

// Check the decoded-row shape of a stream without parsing its row values.
// A filtered stream may have a different encoded /Length and is only
// subject to exact stream frame validation after this metadata check.
fn xref_stream_payload_shape(
    metadata: (Option<usize>, Option<usize>, Option<usize>),
    index: Option<(usize, bool, usize)>, object_number: usize,
    require_self_index: bool, filtered: bool,
) -> Option<(usize, bool)> {
    let (Some(size), Some(length), Some(width)) = metadata else {
        return None;
    };
    if object_number >= size || index.is_some_and(|(end, contains_self, _)| {
        end > size || (require_self_index && !contains_self)
    }) {
        return None;
    }
    // Unfiltered streams contain one /W-sized entry per /Index object,
    // or per 0..Size object when /Index is absent. No row is decoded.
    let rows = index.map_or(size, |(_, _, count)| count);
    if !filtered && rows.checked_mul(width)? != length {
        return None;
    }
    Some((length, index.is_some_and(|(_, self_indexed, _)| {
        !self_indexed
    })))
}

// A terminal stream may carry /Prev just like a traditional trailer.
// Only its immediate earlier header is checked; older chains are outside
// this bounded lexical preflight and are never parsed recursively.
fn xref_stream_header_end(
    prefix: &[u8], previous: Option<(&[u8], usize)>,
    stream_start: usize,
) -> Option<(usize, usize, bool)> {
    let (mut at, object_number) = xref_stream_object_header(prefix)?;
    if !prefix.get(at..).is_some_and(|tail| tail.starts_with(b"<<")) {
        return None;
    }
    at = at.saturating_add(2);
    let mut depth = 1usize;
    let mut array_depth = 0usize;
    let mut found_type = false;
    let mut declared_size = None;
    let mut declared_length = None;
    let mut field_bytes = None;
    let mut seen_prev = false;
    let mut seen_filter = false;
    let mut index_end = None;
    while at < prefix.len() {
        match prefix.get(at) {
            Some(&b'%') => at = pdf_trivia_end(prefix, at),
            Some(&b'(') => at = pdf_literal_end(prefix, at)?,
            Some(&b'<') if prefix.get(at.saturating_add(1)) == Some(&b'<') => {
                depth = depth.saturating_add(1);
                at = at.saturating_add(2);
            }
            Some(&b'<') => at = pdf_hex_end(prefix, at)?,
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
                if depth == 1 && array_depth == 0
                    && pdf_name_is(name, b"Type") {
                    if found_type { return None; }
                    at = pdf_trivia_end(prefix, at);
                    let (value, value_end) = pdf_name(prefix, at)?;
                    if !pdf_name_is(value, b"XRef") { return None; }
                    found_type = true;
                    at = value_end;
                } else if depth == 1 && array_depth == 0
                    && pdf_name_is(name, b"Size") {
                    if declared_size.is_some() { return None; }
                    declared_size =
                        Some(bounded_xref_size(prefix, &mut at, 1)?);
                } else if depth == 1 && array_depth == 0
                    && pdf_name_is(name, b"W") {
                    if field_bytes.is_some() { return None; }
                    field_bytes = Some(xref_stream_field_widths(
                        prefix, &mut at)?);
                } else if depth == 1 && array_depth == 0
                    && pdf_name_is(name, b"Index") {
                    if index_end.is_some() { return None; }
                    index_end = Some(xref_stream_index_end(
                        prefix, &mut at, object_number,
                    )?);
                } else if depth == 1 && array_depth == 0
                    && pdf_name_is(name, b"Length") {
                    if declared_length.is_some() { return None; }
                    declared_length =
                        Some(bounded_stream_length(prefix, &mut at)?);
                } else if depth == 1 && array_depth == 0
                    && pdf_name_is(name, b"Filter") {
                    if seen_filter { return None; }
                    xref_stream_filter(prefix, &mut at)?;
                    seen_filter = true;
                } else if depth == 1 && array_depth == 0
                    && pdf_name_is(name, b"Prev") {
                    if seen_prev || !bounded_stream_previous(
                        prefix, &mut at, previous, stream_start,
                    ) {
                        return None;
                    }
                    seen_prev = true;
                }
            }
            _ => at = at.saturating_add(1),
        }
    }
    if depth != 0 || array_depth != 0 || !found_type { return None; }
    let (length, needs_table_owner) = xref_stream_payload_shape(
        (declared_size, declared_length, field_bytes), index_end,
        object_number, previous.is_some(), seen_filter,
    )?;
    Some((xref_stream_data_start(prefix, at)?, length,
        needs_table_owner))
}

// Validate exact encoded bytes and both closing delimiters for every xref
// stream header, including a prior /Prev target. This stops at the object
// boundary; only the terminal stream requires trivia up to startxref.
// It is lexical framing, not PDF stream decoding or object graph parsing.
fn xref_stream_frame_end(
    bytes: &[u8], after_stream: usize, length: usize,
) -> Option<usize> {
    let raw = bytes.get(after_stream..)?;
    let newline = if raw.starts_with(b"\r\n") { 2 }
        else if raw.starts_with(b"\n") || raw.starts_with(b"\r") {
            1
        } else {
            return None;
        };
    let data_end = after_stream.checked_add(newline)
        .and_then(|start| start.checked_add(length))?;
    // /Length counts encoded stream bytes, not apparent delimiters inside
    // a payload. Only one CR/LF ending may follow before `endstream`.
    let after_data = bytes.get(data_end..)?;
    let tail_newline = if after_data.starts_with(b"\r\n") { 2 }
        else if after_data.starts_with(b"\n")
            || after_data.starts_with(b"\r") { 1 }
        else { return None; };
    let endstream = data_end.checked_add(tail_newline)?;
    if !bytes.get(endstream..).is_some_and(|rest| {
        rest.starts_with(b"endstream")
            && rest.get(b"endstream".len())
                .is_some_and(|byte| pdf_whitespace(*byte))
    }) {
        return None;
    }
    let endstream_end = endstream.checked_add(b"endstream".len())?;
    let endobj = pdf_trivia_end(bytes, endstream_end);
    if !bytes.get(endobj..).is_some_and(|rest| rest.starts_with(b"endobj"))
        || !bytes.get(endobj.saturating_add(b"endobj".len()))
            .is_some_and(|byte| pdf_whitespace(*byte)) {
        return None;
    }
    endobj.checked_add(b"endobj".len())
}

// A terminal cross-reference object must end at the final startxref locator;
// prior revisions may legitimately have their own old locator and later data.
fn supported_xref_stream_tail(
    bytes: &[u8], after_stream: usize, length: usize,
) -> bool {
    xref_stream_frame_end(bytes, after_stream, length)
        .is_some_and(|end| pdf_trivia_end(bytes, end) == bytes.len())
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
// A stalled local spooler must not block the CLI indefinitely, including
// when its stdin pipe fills. Timeout is an uncertain submission outcome:
// the caller must inspect the queue before considering another print job.
fn submit_with_timeout(
    executable: &Path, printer: &str, pdf: Vec<u8>, timeout: Duration,
) -> Result<(), String> {
    let mut child = Command::new(executable)
        .args(["-d", printer, "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("cannot launch local CUPS lp: {error}"))?;
    let Some(mut input) = child.stdin.take() else {
        drop(child.kill());
        drop(child.wait());
        return Err("lp stdin unavailable".into());
    };
    // The writer owns the already-reviewed Vec; no second PDF read or
    // unbounded copy is needed. On timeout, killing lp closes its pipe.
    let writer = match thread::Builder::new()
        .name("atrament-cups-stdin".into())
        .spawn(move || input.write_all(&pdf)) {
        Ok(writer) => writer,
        Err(error) => {
            drop(child.kill());
            drop(child.wait());
            return Err(format!("cannot start lp stdin writer: {error}"));
        }
    };
    let start = Instant::now();
    let outcome = loop {
        match child.try_wait() {
            Ok(Some(status)) if writer.is_finished() => break Ok(status),
            Ok(_) if start.elapsed() >= timeout => break Err(concat!(
                "CUPS submission timed out; outcome uncertain; ",
                "inspect the queue before retrying",
            ).to_owned()),
            Ok(_) => thread::sleep(Duration::from_millis(10)),
            Err(error) => break Err(format!("cannot await local lp: {error}")),
        }
    };
    let status = match outcome {
        Ok(status) => status,
        Err(error) => {
            drop(child.kill());
            drop(child.wait());
            return Err(error);
        }
    };
    // An early lp exit can report success while closing stdin before the
    // reviewed PDF is transferred. A partial write is not proof of job
    // rejection: lp may already have queued an incomplete submission.
    writer.join()
        .map_err(|_panic| concat!(
            "CUPS submission outcome uncertain; ",
            "inspect the queue before retrying: lp writer stopped",
        ).to_owned())?
        .map_err(|error| format!(concat!(
            "CUPS submission outcome uncertain; ",
            "inspect the queue before retrying: ",
            "cannot supply PDF to lp: {}",
        ), error))?;
    if !status.success() {
        return Err(format!("CUPS refused print submission: {status}"));
    }
    Ok(())
}

fn submit_with(executable: &Path, printer: &str, pdf: Vec<u8>)
    -> Result<(), String> {
    submit_with_timeout(executable, printer, pdf, CUPS_SUBMISSION_TIMEOUT)
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
        submit_with(executable, &request.printer, bytes)?;
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
