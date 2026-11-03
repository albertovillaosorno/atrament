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
//   - Strict versioned JSON mapping for the native handwritten statistics.
// - Must-Not:
//   - Read or write user files, render handwriting, choose fonts, sample ink,
//     normalize glyphs, or bypass the domain and JSON syntax admission gates.
// - Allows:
//   - Inputs: Caller-provided bounded JSON bytes or validated typed profiles.
//   - Outputs: Validated Rust statistics or deterministic JSON bytes, errors.
//   - Side effects: Process-local memory allocation only.
// - Split-When:
//   - File I/O or CLI command handling gains independent application authority.
// - Merge-When:
//   - A notebook/handwriting profile codec subsumes these same fields.
// - Summary:
//   - Enables reusable, statically bounded Rust-only profile round-trips.
// - Description:
//   - Rejects duplicate and unknown keys before accepting any profile data.
// - Usage:
//   - Feed the same codec to native file commands and localhost endpoints.
// - Defaults:
//   - No silent defaults for missing serialized statistics or glyph fields.
//

//! Bounded Rust-only JSON codec for manually authored handwriting statistics.

use std::fmt::Write as _;
use std::str::from_utf8;

use atrament_handwriting_statistics::{
    Dimension, DimensionStatistics, GlyphStatistics,
    HandwritingStatisticsError, HandwritingStatisticsProfile,
    MAXIMUM_GLYPH_OVERRIDES, ScalarStatistics,
};
use atrament_profile_json_syntax::{
    ProfileJsonSyntaxError, ProfileJsonSyntaxLimits,
    ValidatedProfileJsonObject, ValidatedProfileJsonValue,
    validate_profile_json_object,
};
use atrament_unicode_grapheme_boundary_port::GraphemeBoundaryProvider;
#[cfg(test)]
use atrament_unicode_grapheme_segmentation as _;

/// Maximum bytes in an independently editable handwriting JSON file.
pub const MAXIMUM_STATISTICS_JSON_BYTES: usize = 1_048_576;

const ROOT_KEYS: [&str; 4] = ["version", "seed", "globals", "glyphs"];
const GLYPH_KEYS: [&str; 2] = ["grapheme", "parameters"];
const PARAMETER_KEYS: [&str; 6] = [
    "dimension",
    "minimum",
    "maximum",
    "mean",
    "median",
    "standard_deviation",
];

/// Exact reason for refusing a typed handwriting statistics JSON profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HandwritingStatisticsJsonError {
    /// An integer or dimension key is invalid.
    InvalidFieldValue,
    /// Typed domain validation refused the decoded profile.
    InvalidProfile(HandwritingStatisticsError),
    /// A required field is missing.
    MissingField,
    /// The generated JSON exceeds the same input budget.
    OutputTooLarge,
    /// Input JSON is not syntactically safe, bounded, or unambiguous.
    Syntax(ProfileJsonSyntaxError),
    /// A collection exceeds the supported element count.
    TooManyEntries,
    /// An unrecognized field was present in an admitted object.
    UnknownField,
    /// A field has the wrong JSON primitive or container shape.
    WrongFieldType,
}

fn check_fields(
    value: ValidatedProfileJsonValue<'_>,
    required: &[&str],
) -> Result<(), HandwritingStatisticsJsonError> {
    let fields = value
        .object_fields()
        .ok_or(HandwritingStatisticsJsonError::WrongFieldType)?;
    for field in fields {
        if !required.iter().any(|name| field.key_matches(name)) {
            return Err(HandwritingStatisticsJsonError::UnknownField);
        }
    }
    for name in required {
        if value.object_member(name).is_none() {
            return Err(HandwritingStatisticsJsonError::MissingField);
        }
    }
    Ok(())
}

fn string_field(
    value: ValidatedProfileJsonValue<'_>,
    key: &str,
    limit: usize,
) -> Result<String, HandwritingStatisticsJsonError> {
    let selected = value
        .object_member(key)
        .ok_or(HandwritingStatisticsJsonError::MissingField)?;
    let mut buffer = [0u8; 64];
    let amount = selected
        .copy_string_utf8(&mut buffer)
        .map_err(|_error| HandwritingStatisticsJsonError::WrongFieldType)?;
    if amount > limit {
        return Err(HandwritingStatisticsJsonError::InvalidFieldValue);
    }
    from_utf8(
        buffer
            .get(..amount)
            .ok_or(HandwritingStatisticsJsonError::InvalidFieldValue)?,
    )
    .map(str::to_owned)
    .map_err(|_error| HandwritingStatisticsJsonError::InvalidFieldValue)
}

fn signed_field(
    value: ValidatedProfileJsonValue<'_>,
    key: &str,
) -> Result<i32, HandwritingStatisticsJsonError> {
    let raw = value
        .object_member(key)
        .ok_or(HandwritingStatisticsJsonError::MissingField)?;
    if raw.kind()
        != Some(atrament_profile_json_syntax::ProfileJsonValueKind::Number)
    {
        return Err(HandwritingStatisticsJsonError::WrongFieldType);
    }
    from_utf8(raw.bytes())
        .ok()
        .and_then(|text| text.parse::<i32>().ok())
        .ok_or(HandwritingStatisticsJsonError::InvalidFieldValue)
}

fn unsigned_field(
    value: ValidatedProfileJsonValue<'_>,
    key: &str,
) -> Result<u64, HandwritingStatisticsJsonError> {
    value
        .object_member(key)
        .ok_or(HandwritingStatisticsJsonError::MissingField)?
        .unsigned_integer()
        .ok_or(HandwritingStatisticsJsonError::InvalidFieldValue)
}

fn parse_parameter(
    value: ValidatedProfileJsonValue<'_>,
) -> Result<DimensionStatistics, HandwritingStatisticsJsonError> {
    check_fields(value, &PARAMETER_KEYS)?;
    let dimension = Dimension::from_key(&string_field(value, "dimension", 64)?)
        .ok_or(HandwritingStatisticsJsonError::InvalidFieldValue)?;
    let deviation = u32::try_from(unsigned_field(value, "standard_deviation")?)
        .map_err(|_error| HandwritingStatisticsJsonError::InvalidFieldValue)?;
    Ok(DimensionStatistics {
        dimension,
        statistics: ScalarStatistics {
            minimum: signed_field(value, "minimum")?,
            maximum: signed_field(value, "maximum")?,
            mean: signed_field(value, "mean")?,
            median: signed_field(value, "median")?,
            standard_deviation: deviation,
        },
    })
}

fn parse_parameters(
    array: ValidatedProfileJsonValue<'_>,
) -> Result<Vec<DimensionStatistics>, HandwritingStatisticsJsonError> {
    let elements = array
        .array_elements()
        .ok_or(HandwritingStatisticsJsonError::WrongFieldType)?;
    let mut parameters = Vec::new();
    for element in elements {
        if parameters.len() >= Dimension::ALL.len() {
            return Err(HandwritingStatisticsJsonError::TooManyEntries);
        }
        parameters.push(parse_parameter(element)?);
    }
    Ok(parameters)
}

fn parse_glyphs(
    value: ValidatedProfileJsonValue<'_>,
) -> Result<Vec<GlyphStatistics>, HandwritingStatisticsJsonError> {
    let elements = value
        .array_elements()
        .ok_or(HandwritingStatisticsJsonError::WrongFieldType)?;
    let mut glyphs = Vec::new();
    for element in elements {
        if glyphs.len() >= MAXIMUM_GLYPH_OVERRIDES {
            return Err(HandwritingStatisticsJsonError::TooManyEntries);
        }
        check_fields(element, &GLYPH_KEYS)?;
        glyphs.push(GlyphStatistics {
            grapheme: string_field(element, "grapheme", 64)?,
            parameters: parse_parameters(
                element
                    .object_member("parameters")
                    .ok_or(HandwritingStatisticsJsonError::MissingField)?,
            )?,
        });
    }
    Ok(glyphs)
}

fn parse_root(
    object: ValidatedProfileJsonObject<'_>,
    boundaries: &impl GraphemeBoundaryProvider,
) -> Result<HandwritingStatisticsProfile, HandwritingStatisticsJsonError> {
    for field in object.root_fields() {
        if !ROOT_KEYS.iter().any(|name| field.key_matches(name)) {
            return Err(HandwritingStatisticsJsonError::UnknownField);
        }
    }
    let root_value = |key| {
        object
            .root_member_value(key)
            .ok_or(HandwritingStatisticsJsonError::MissingField)
    };
    let version_value = root_value("version")?;
    let mut version_bytes = [0u8; 64];
    let len = version_value
        .copy_string_utf8(&mut version_bytes)
        .map_err(|_error| HandwritingStatisticsJsonError::WrongFieldType)?;
    let version = from_utf8(
        version_bytes
            .get(..len)
            .ok_or(HandwritingStatisticsJsonError::InvalidFieldValue)?,
    )
    .map_err(|_error| HandwritingStatisticsJsonError::InvalidFieldValue)?
    .to_owned();
    let seed = root_value("seed")?
        .unsigned_integer()
        .ok_or(HandwritingStatisticsJsonError::InvalidFieldValue)?;
    let profile = HandwritingStatisticsProfile {
        version,
        seed,
        globals: parse_parameters(root_value("globals")?)?,
        glyphs: parse_glyphs(root_value("glyphs")?)?,
    };
    profile
        .validate(boundaries)
        .map_err(HandwritingStatisticsJsonError::InvalidProfile)?;
    Ok(profile)
}

/// Decode bounded Rust statistics JSON after duplicate-key syntax validation.
///
/// # Errors
///
/// Refuses unknown, missing, conflicting, out-of-range, or oversized data.
pub fn decode_statistics_json<Boundary>(
    bytes: &[u8],
    boundaries: &Boundary,
) -> Result<HandwritingStatisticsProfile, HandwritingStatisticsJsonError>
where
    Boundary: GraphemeBoundaryProvider,
{
    let validated = validate_profile_json_object(
        bytes,
        ProfileJsonSyntaxLimits {
            maximum_bytes: MAXIMUM_STATISTICS_JSON_BYTES,
            maximum_nesting: 6,
            maximum_object_members: 16,
            maximum_values: 60_000,
        },
    )
    .map_err(HandwritingStatisticsJsonError::Syntax)?;
    parse_root(validated, boundaries)
}

fn json_string(output: &mut String, value: &str) {
    output.push('"');
    for scalar in value.chars() {
        match scalar {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character if character.is_control() => {
                let _write_result =
                    write!(output, "\\u{:04X}", u32::from(character));
            }
            character => output.push(character),
        }
    }
    output.push('"');
}

fn append_parameter(output: &mut String, entry: DimensionStatistics) {
    let statistics = entry.statistics;
    let _write_result = write!(
        output,
        concat!(
            "{{\"dimension\":\"{}\",",
            "\"minimum\":{},\"maximum\":{},",
            "\"mean\":{},\"median\":{},\"standard_deviation\":{}}}",
        ),
        entry.dimension.key(),
        statistics.minimum,
        statistics.maximum,
        statistics.mean,
        statistics.median,
        statistics.standard_deviation,
    );
}

fn append_parameters(output: &mut String, parameters: &[DimensionStatistics]) {
    output.push('[');
    for (index, parameter) in parameters.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        append_parameter(output, *parameter);
    }
    output.push(']');
}

/// Encode a validated profile without loss, clipping or reinterpretation.
///
/// The fixed field order and exact glyph order yield repeatable UTF-8 bytes.
/// The returned string can be validated and decoded by this same Rust codec.
///
/// # Errors
///
/// Refuses invalid typed profiles or output outside the same input budget.
pub fn encode_statistics_json<Boundary>(
    profile: &HandwritingStatisticsProfile,
    boundaries: &Boundary,
) -> Result<Vec<u8>, HandwritingStatisticsJsonError>
where
    Boundary: GraphemeBoundaryProvider,
{
    profile
        .validate(boundaries)
        .map_err(HandwritingStatisticsJsonError::InvalidProfile)?;
    let mut output = String::new();
    output.push_str("{\"version\":");
    json_string(&mut output, &profile.version);
    let _write_result =
        write!(output, ",\"seed\":{},\"globals\":", profile.seed);
    append_parameters(&mut output, &profile.globals);
    output.push_str(",\"glyphs\":[");
    for (index, glyph) in profile.glyphs.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"grapheme\":");
        json_string(&mut output, &glyph.grapheme);
        output.push_str(",\"parameters\":");
        append_parameters(&mut output, &glyph.parameters);
        output.push('}');
    }
    output.push_str("]}");
    if output.len() > MAXIMUM_STATISTICS_JSON_BYTES {
        return Err(HandwritingStatisticsJsonError::OutputTooLarge);
    }
    Ok(output.into_bytes())
}
