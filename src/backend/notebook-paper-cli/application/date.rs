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
//   - Bounded date-text validation and printed vector glyph projection.
// - Must-Not:
//   - Synthesize handwriting, interpret locale, render other notebook text,
//     read assets, or write output files.
// - Allows:
//   - Inputs: Explicit numeric date text and admitted paper placeholder.
//   - Outputs: Filled vector rectangles shared by native PDF/PNG exporters.
//   - Side effects: In-memory bounded vector construction only.
// - Split-When:
//   - General notebook text layout becomes executable.
// - Merge-When:
//   - Full text shaping takes ownership of printed notebook annotations.
// - Summary:
//   - Renders an honest printed numeric date in an existing reserved box.
// - Description:
//   - Reuses the existing 3x5 printed digits with fixed punctuation masks.
// - Usage:
//   - Called after the notebook's date placement passes Rust admission.
// - Defaults:
//   - Empty text yields an empty placeholder with no glyph ink.
//

//! Printed date annotations, not sampled handwriting or searchable PDF text.

use super::{Options, PaperDatePlaceholder, filled_square};
use atrament_pdf_vector_serializer::{PdfRgb, PdfVectorPath};

const DATE_BOX_WIDTH_UM: u64 = 36_000;
const CELL_UM: u64 = 450;
const STEP_UM: u64 = 600;
const GLYPH_WIDTH_UM: u64 = 2 * STEP_UM + CELL_UM;
const GLYPH_ADVANCE_UM: u64 = GLYPH_WIDTH_UM + STEP_UM;
const TOTAL_WIDTH_UM: u64 = 10 * GLYPH_WIDTH_UM + 9 * STEP_UM;
const INK: PdfRgb = PdfRgb {
    red: 48,
    green: 66,
    blue: 88,
};

fn digits(bytes: &[u8]) -> Option<u32> {
    bytes.iter().try_fold(0u32, |value, ch| {
        if !ch.is_ascii_digit() {
            return None;
        }
        value
            .checked_mul(10)?
            .checked_add(u32::from(ch.checked_sub(b'0')?))
    })
}

const fn valid_gregorian_date(year: u32, month: u32, day: u32) -> bool {
    let leap = year.is_multiple_of(4)
        && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let maximum_day = match month {
        2 => {
            if leap {
                29
            } else {
                28
            }
        }
        4 | 6 | 9 | 11 => 30,
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        _ => 0,
    };
    year != 0 && day != 0 && day <= maximum_day
}

/// Validate one optional printed numeric calendar date.
///
/// The supported labels are ISO `YYYY-MM-DD`, and `NN/NN/YYYY` or
/// `NN.NN.YYYY` with either day-first or month-first order. Neither ambiguous
/// order is silently interpreted as a locale: it is printed exactly as typed.
///
/// # Errors
/// Refuses impossible calendar dates and unsupported or mixed separators.
pub fn validate_printed_date_text(value: &str) -> Result<(), String> {
    if value.is_empty() {
        return Ok(());
    }
    let bytes = value.as_bytes();
    if bytes.len() != 10 {
        return Err("date text must use YYYY-MM-DD or NN/NN/YYYY".into());
    }
    let same_separator = |first: usize, second: usize| {
        bytes.get(first).is_some_and(|ch| b"-/.".contains(ch))
            && bytes.get(first) == bytes.get(second)
    };
    let valid = if same_separator(4, 7) && bytes.get(4) == Some(&b'-') {
        bytes
            .get(0..4)
            .and_then(digits)
            .zip(bytes.get(5..7).and_then(digits))
            .zip(bytes.get(8..10).and_then(digits))
            .is_some_and(|((year, month), day)| {
                valid_gregorian_date(year, month, day)
            })
    } else if same_separator(2, 5) && bytes.get(2) != Some(&b'-') {
        bytes
            .get(0..2)
            .and_then(digits)
            .zip(bytes.get(3..5).and_then(digits))
            .zip(bytes.get(6..10).and_then(digits))
            .is_some_and(|((first, second), year)| {
                valid_gregorian_date(year, second, first)
                    || valid_gregorian_date(year, first, second)
            })
    } else {
        false
    };
    if valid {
        Ok(())
    } else {
        Err("invalid printed Gregorian date".into())
    }
}

/// Append 3x5 printed date symbols within an admitted 36x8 mm placeholder.
///
/// # Errors
/// Refuses invalid text or insufficient physical date placement clearance.
pub(super) fn append_date_text(
    paths: &mut Vec<PdfVectorPath>,
    options: Options,
    placeholder: PaperDatePlaceholder,
    text: &str,
) -> Result<(), String> {
    validate_printed_date_text(text)?;
    if text.is_empty() {
        return Ok(());
    }
    let right = matches!(
        placeholder,
        PaperDatePlaceholder::RightLine | PaperDatePlaceholder::RightBox
    );
    let x = if right {
        u64::from(options.width_um)
            .checked_sub(u64::from(options.margin_right_um))
            .and_then(|width| width.checked_sub(DATE_BOX_WIDTH_UM))
            .ok_or("date stamp exceeds right margin")?
    } else {
        u64::from(options.margin_left_um)
    };
    let y = u64::from(
        options
            .margin_top_um
            .checked_sub(8_000)
            .ok_or("date stamp needs top clearance")?,
    )
    .div_ceil(2);
    // Ten 3-column glyphs plus nine inter-glyph gaps fit in 36 mm.
    let label_left_in_box = DATE_BOX_WIDTH_UM
        .checked_sub(TOTAL_WIDTH_UM)
        .ok_or("date label exceeds box")?
        .checked_div(2)
        .ok_or("date label center unavailable")?;
    let left = x
        .checked_add(label_left_in_box)
        .ok_or("date label position overflow")?;
    let top = y.checked_add(2_000).ok_or("date label vertical overflow")?;
    for (index, ch) in text.bytes().enumerate() {
        let pattern = match ch {
            b'0'..=b'9' => *super::PAGE_NUMBER_DIGITS
                .get(usize::from(ch.saturating_sub(b'0')))
                .ok_or("printed digit index unavailable")?,
            b'-' => [0, 0, 0b111, 0, 0],
            b'/' => [0b001, 0b001, 0b010, 0b100, 0b100],
            b'.' => [0, 0, 0, 0, 0b010],
            _ => return Err("unsupported printed date glyph".into()),
        };
        let glyph_x = u64::try_from(index)
            .map_err(|error| format!("date index refused: {error}"))?
            .checked_mul(GLYPH_ADVANCE_UM)
            .and_then(|offset| left.checked_add(offset))
            .ok_or("date glyph position overflow")?;
        for (row, bits) in pattern.iter().enumerate() {
            let row_y = u64::try_from(row)
                .map_err(|error| format!("date row refused: {error}"))?
                .checked_mul(STEP_UM)
                .and_then(|offset| top.checked_add(offset))
                .ok_or("date row overflow")?;
            for column in 0..3u64 {
                if bits & (0b100 >> column) != 0 {
                    let column_x = column
                        .checked_mul(STEP_UM)
                        .and_then(|offset| glyph_x.checked_add(offset))
                        .ok_or("date column overflow")?;
                    paths.push(filled_square(column_x, row_y, CELL_UM, INK)?);
                }
            }
        }
    }
    Ok(())
}
