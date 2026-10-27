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
//   - Strict bounded mapping of source-sealed JSON to typed profile metadata.
// - Must-Not:
//   - Read files or ZIP, trust unsealed JSON, verify entry bytes, authorize
//     source paths, admit unsupported features, or mutate a session profile.
// - Allows:
//   - Inputs: Constructor-sealed root JSON and caller-selected decode limits.
//   - Outputs: Typed metadata or a location-specific schema rejection.
//   - Side effects: Bounded process-local allocation only.
// - Split-When:
//   - Canonical profile JSON serialization gains a distinct writer boundary.
// - Merge-When:
//   - Explicit profile Import takes full schema decoding ownership.
// - Summary:
//   - Refuses missing, unknown, wrongly typed, or malformed manifest fields.
// - Description:
//   - Typed metadata is not verified against the containing ZIP or its bytes.
// - Usage:
//   - Decode only the root object returned by bounded ZIP JSON inspection.
// - Defaults:
//   - Reject unknown wire keys instead of silently losing their contents.
//

//! Schema-specific, read-only decoder of a syntax-admitted profile manifest.

use atrament_handwriting_profile_container::{
    ProfileManifest, ProfileManifestEntry, Sha256Digest,
};
use atrament_profile_json_syntax::{
    ValidatedProfileJsonObject, ValidatedProfileJsonValue,
};

/// Allocation ceilings independent of the upstream JSON grammar limits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProfileManifestJsonLimits {
    /// Maximum number of non-manifest member declarations.
    pub maximum_entries: usize,
    /// Maximum feature identifiers within either feature list.
    pub maximum_features: usize,
    /// Maximum decoded UTF-8 bytes for any string field.
    pub maximum_string_bytes: usize,
}

/// A manifest schema violation, with stable structural field location.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileManifestJsonError {
    /// An array has more members than the caller permits.
    ArrayLimitExceeded(&'static str),
    /// SHA-256 must be exactly 64 lowercase hexadecimal characters.
    InvalidSha256,
    /// An integer field is negative, fractional, exponent, or out of `u64`.
    InvalidUnsignedInteger(&'static str),
    /// A mandatory member is absent.
    MissingField(&'static str),
    /// A decoded string's byte length exceeds the independent caller limit.
    StringLimitExceeded(&'static str),
    /// An unrecognized key could not be preserved by the typed model.
    UnknownField(&'static str),
    /// A present field has the wrong JSON kind.
    WrongType(&'static str),
}

/// Decode every required first-release wire field, refusing unknown keys.
///
/// The field names are `container_version`, `profile_identity`,
/// `required_features`, `optional_features`, and `entries`. Each entry requires
/// `path`, `media_type`, `byte_length`, and lowercase `sha256` hex. This codec
/// does not enforce ZIP source authority or admit the resulting typed manifest:
/// callers must still use manifest validation, exact inventory checks and
/// independently verified entry bytes before profile acceptance.
///
/// # Errors
///
/// Rejects incompatible object shape or caller resource limits without
/// returning a partially decoded profile.
pub fn decode_profile_manifest_json(
    root: ValidatedProfileJsonObject<'_>,
    limits: ProfileManifestJsonLimits,
) -> Result<ProfileManifest, ProfileManifestJsonError> {
    for field in root.root_fields() {
        if ![
            "container_version",
            "profile_identity",
            "required_features",
            "optional_features",
            "entries",
        ]
        .iter()
        .any(|key| field.key_matches(key))
        {
            return Err(ProfileManifestJsonError::UnknownField("manifest"));
        }
    }
    let read = |key| {
        root.root_member_value(key)
            .ok_or(ProfileManifestJsonError::MissingField(key))
    };
    let container_version = read_string(
        read("container_version")?, "container_version", limits,
    )?;
    let profile_identity = read_string(
        read("profile_identity")?, "profile_identity", limits,
    )?;
    let required_features = read_strings(
        read("required_features")?, "required_features", limits,
    )?;
    let optional_features = read_strings(
        read("optional_features")?, "optional_features", limits,
    )?;
    let entries_value = read("entries")?;
    let entries_array = entries_value
        .array_elements()
        .ok_or(ProfileManifestJsonError::WrongType("entries"))?;
    let mut entries = Vec::new();
    for entry in entries_array {
        if entries.len() >= limits.maximum_entries {
            return Err(ProfileManifestJsonError::ArrayLimitExceeded("entries"));
        }
        entries.push(decode_entry(entry, limits)?);
    }
    Ok(ProfileManifest {
        container_version,
        entries,
        optional_features,
        profile_identity,
        required_features,
    })
}

fn decode_entry(
    value: ValidatedProfileJsonValue<'_>,
    limits: ProfileManifestJsonLimits,
) -> Result<ProfileManifestEntry, ProfileManifestJsonError> {
    let fields = value
        .object_fields()
        .ok_or(ProfileManifestJsonError::WrongType("entries[]"))?;
    for field in fields {
        if !["path", "media_type", "byte_length", "sha256"]
            .iter()
            .any(|key| field.key_matches(key))
        {
            return Err(ProfileManifestJsonError::UnknownField("entries[]"));
        }
    }
    let read = |key| {
        value
            .object_member(key)
            .ok_or(ProfileManifestJsonError::MissingField(key))
    };
    let path = read_string(read("path")?, "entries[].path", limits)?;
    let media_type = read_string(
        read("media_type")?, "entries[].media_type", limits,
    )?;
    let byte_length = read("byte_length")?.unsigned_integer().ok_or(
        ProfileManifestJsonError::InvalidUnsignedInteger(
            "entries[].byte_length",
        ),
    )?;
    let digest = parse_sha256(read("sha256")?)?;
    Ok(ProfileManifestEntry {
        byte_length,
        digest,
        media_type,
        path,
    })
}

fn read_strings(
    value: ValidatedProfileJsonValue<'_>,
    location: &'static str,
    limits: ProfileManifestJsonLimits,
) -> Result<Vec<String>, ProfileManifestJsonError> {
    let array = value
        .array_elements()
        .ok_or(ProfileManifestJsonError::WrongType(location))?;
    let mut strings = Vec::new();
    for item in array {
        if strings.len() >= limits.maximum_features {
            return Err(ProfileManifestJsonError::ArrayLimitExceeded(location));
        }
        strings.push(read_string(item, location, limits)?);
    }
    Ok(strings)
}

fn read_string(
    value: ValidatedProfileJsonValue<'_>,
    location: &'static str,
    limits: ProfileManifestJsonLimits,
) -> Result<String, ProfileManifestJsonError> {
    let chars = value
        .string_chars()
        .ok_or(ProfileManifestJsonError::WrongType(location))?;
    let mut length = 0usize;
    for character in chars.clone() {
        length = length.saturating_add(character.len_utf8());
        if length > limits.maximum_string_bytes {
            return Err(ProfileManifestJsonError::StringLimitExceeded(location));
        }
    }
    let mut decoded = String::with_capacity(length);
    decoded.extend(chars);
    Ok(decoded)
}

fn parse_sha256(
    value: ValidatedProfileJsonValue<'_>,
) -> Result<Sha256Digest, ProfileManifestJsonError> {
    let mut chars = value.string_chars()
        .ok_or(ProfileManifestJsonError::WrongType("entries[].sha256"))?;
    let mut digest = [0u8; 32];
    for byte in &mut digest {
        let high = chars.next().and_then(lower_hex_digit)
            .ok_or(ProfileManifestJsonError::InvalidSha256)?;
        let low = chars.next().and_then(lower_hex_digit)
            .ok_or(ProfileManifestJsonError::InvalidSha256)?;
        *byte = (high << 4u32) | low;
    }
    if chars.next().is_some() {
        return Err(ProfileManifestJsonError::InvalidSha256);
    }
    Ok(Sha256Digest::from_bytes(digest))
}

fn lower_hex_digit(character: char) -> Option<u8> {
    match character {
        '0'..='9' | 'a'..='f' => {
            u8::try_from(character.to_digit(16u32)?).ok()
        }
        _ => None,
    }
}
