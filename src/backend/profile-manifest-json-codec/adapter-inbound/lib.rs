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

use std::fmt;

use atrament_handwriting_profile_container::{
    ProfileManifest, ProfileManifestEntry, Sha256Digest,
};
use atrament_profile_json_syntax::{
    ValidatedProfileJsonObject, ValidatedProfileJsonValue,
};
use atrament_profile_zip_reader::{
    ProfileZipArchive, ValidatedProfileZipRootJson,
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
    /// The bounded typed metadata could not be allocated.
    AllocationFailed,
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

/// The typed manifest paired with the exact CRC-checked ZIP that supplied it.
///
/// This association cannot be constructed from unrelated manifest values by
/// callers. The underlying source bytes, root syntax, and typed metadata stay
/// tied together, but the declarations have not yet been checked against the
/// ZIP member inventory and SHA-256 bytes. No filesystem source is authorized.
/// Debug output excludes private decoded manifest and archive contents.
pub struct DecodedProfileZipManifest<'archive, 'bytes> {
    archive: &'archive ProfileZipArchive<'bytes>,
    manifest: ProfileManifest,
    root_sha256: Sha256Digest,
}

impl fmt::Debug for DecodedProfileZipManifest<'_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DecodedProfileZipManifest")
            .finish_non_exhaustive()
    }
}

impl<'archive, 'bytes> DecodedProfileZipManifest<'archive, 'bytes> {
    /// Borrow the checked source archive for subsequent inventory verification.
    #[must_use]
    pub const fn archive(&self) -> &'archive ProfileZipArchive<'bytes> {
        self.archive
    }

    /// Borrow the schema-decoded declarations, without Import authority.
    #[must_use]
    pub const fn manifest(&self) -> &ProfileManifest {
        &self.manifest
    }

    /// Return the hash of the exact root member bytes admitted from this ZIP.
    ///
    /// This is **not** a cryptographic proof of the other archive members.
    #[must_use]
    pub const fn root_sha256(&self) -> Sha256Digest {
        self.root_sha256
    }
}

/// Decode only root JSON evidence sealed to the checked source ZIP archive.
///
/// Unlike decoding a standalone syntax-admitted object, this function retains
/// the exact supplying archive borrow and root-only SHA-256 fingerprint in a
/// private constructor result. This is not entry-hash verification or Import.
///
/// # Errors
///
/// Propagates exact typed schema and allocation-limit errors without exposing
/// a partially decoded manifest or independent archive relationship.
pub fn decode_attached_profile_manifest_json<'archive, 'bytes>(
    source: &ValidatedProfileZipRootJson<'archive, 'bytes>,
    limits: ProfileManifestJsonLimits,
) -> Result<DecodedProfileZipManifest<'archive, 'bytes>,
    ProfileManifestJsonError> {
    let manifest = decode_profile_manifest_json(source.root(), limits)?;
    Ok(DecodedProfileZipManifest {
        archive: source.archive(),
        manifest,
        root_sha256: source.source_sha256(),
    })
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
        let parsed = decode_entry(entry, limits)?;
        entries.try_reserve(1)
            .map_err(|_error| ProfileManifestJsonError::AllocationFailed)?;
        entries.push(parsed);
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
        let parsed = read_string(item, location, limits)?;
        strings.try_reserve(1)
            .map_err(|_error| ProfileManifestJsonError::AllocationFailed)?;
        strings.push(parsed);
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
    let mut decoded = String::new();
    decoded.try_reserve_exact(length)
        .map_err(|_error| ProfileManifestJsonError::AllocationFailed)?;
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
