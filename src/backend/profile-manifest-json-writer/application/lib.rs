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
//   - Deterministic UTF-8 manifest JSON from admitted profile declarations.
// - Must-Not:
//   - Read or write files, choose payloads, emit ZIP framing, grant Export
//     authority, or claim that a serialized manifest has matching entry bytes.
// - Allows:
//   - Inputs: Typed manifest, supported feature set and resource ceilings.
//   - Outputs: Canonical JSON bytes or a typed refusal.
//   - Side effects: Explicitly bounded in-memory allocation only.
// - Split-When:
//   - Section encoders or a complete profile Export adapter are required.
// - Merge-When:
//   - A unified canonical profile writer replaces independent framing steps.
// - Summary:
//   - Stable key, number, string, feature and entry ordering for profile JSON.
// - Description:
//   - Shared counting/emission path prevents budget and byte-count drift.
// - Usage:
//   - Verify content/digests separately before storing with the ZIP writer.
// - Defaults:
//   - Invalid manifests and over-budget outputs fail before allocation.
//

//! Canonical, allocation-bounded JSON for typed portable profile metadata.

use std::cmp::Ordering;

use atrament_handwriting_profile_container::{
    ProfileManifest, ProfileManifestEntry, ProfileManifestError,
    validate_profile_manifest,
};

/// Independent limits for canonical manifest serialization, not ZIP bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProfileManifestJsonWriteLimits {
    /// Maximum complete UTF-8 manifest JSON byte count.
    pub maximum_bytes: usize,
    /// Maximum number of non-root declarations.
    pub maximum_entries: usize,
    /// Maximum number of required or optional feature identifiers per list.
    pub maximum_features: usize,
    /// Maximum unescaped UTF-8 bytes of any input string.
    pub maximum_string_bytes: usize,
}

/// Reject invalid declarations before producing canonical output bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProfileManifestJsonWriteError {
    /// The destination could not reserve its bounded output allocation.
    AllocationFailed,
    /// A feature or entry list exceeded the caller's count ceiling.
    ArrayLimitExceeded(&'static str),
    /// Caller-owned output is smaller than the exact planned JSON length.
    DestinationTooSmall {
        /// Exact canonical manifest size in bytes.
        required: usize,
    },
    /// One internal encoding invariant was unexpectedly violated.
    InternalEncoding,
    /// Typed metadata fails the existing container admission contract.
    Manifest(ProfileManifestError),
    /// Canonical JSON would exceed the output-byte ceiling.
    ManifestTooLarge,
    /// An input string exceeded its independent decoded-byte ceiling.
    StringLimitExceeded(&'static str),
}

trait JsonSink {
    fn emit(
        &mut self, bytes: &[u8],
    ) -> Result<(), ProfileManifestJsonWriteError>;
}

struct JsonCounter {
    count: usize,
    limit: usize,
}

impl JsonSink for JsonCounter {
    fn emit(
        &mut self, bytes: &[u8],
    ) -> Result<(), ProfileManifestJsonWriteError> {
        self.count = self
            .count
            .checked_add(bytes.len())
            .ok_or(ProfileManifestJsonWriteError::ManifestTooLarge)?;
        if self.count > self.limit {
            return Err(ProfileManifestJsonWriteError::ManifestTooLarge);
        }
        Ok(())
    }
}

struct JsonSliceSink<'buffer> {
    buffer: &'buffer mut [u8],
    position: usize,
}

impl JsonSink for JsonSliceSink<'_> {
    fn emit(
        &mut self, bytes: &[u8],
    ) -> Result<(), ProfileManifestJsonWriteError> {
        let end = self.position
            .checked_add(bytes.len())
            .ok_or(ProfileManifestJsonWriteError::InternalEncoding)?;
        let target = self.buffer
            .get_mut(self.position..end)
            .ok_or(ProfileManifestJsonWriteError::InternalEncoding)?;
        target.copy_from_slice(bytes);
        self.position = end;
        Ok(())
    }
}

impl JsonSink for Vec<u8> {
    fn emit(
        &mut self, bytes: &[u8],
    ) -> Result<(), ProfileManifestJsonWriteError> {
        self.extend_from_slice(bytes);
        Ok(())
    }
}

struct CanonicalManifestPlan<'manifest> {
    bytes: usize,
    entries: Vec<&'manifest ProfileManifestEntry>,
    manifest: &'manifest ProfileManifest,
    optional: Vec<&'manifest str>,
    required: Vec<&'manifest str>,
}

impl CanonicalManifestPlan<'_> {
    fn emit<S>(
        &self,
        sink: &mut S,
    ) -> Result<(), ProfileManifestJsonWriteError>
    where
        S: JsonSink,
    {
        append_manifest(
            sink, self.manifest, &self.entries, &self.required, &self.optional,
        )
    }
}

const fn checked_string(
    value: &str,
    location: &'static str,
    limits: ProfileManifestJsonWriteLimits,
) -> Result<(), ProfileManifestJsonWriteError> {
    if value.len() > limits.maximum_string_bytes {
        return Err(
            ProfileManifestJsonWriteError::StringLimitExceeded(location),
        );
    }
    Ok(())
}

fn append_json_string<S>(
    sink: &mut S,
    value: &str,
) -> Result<(), ProfileManifestJsonWriteError>
where
    S: JsonSink,
{
    sink.emit(b"\"")?;
    for character in value.chars() {
        match character {
            '"' => sink.emit(br#"\""#)?,
            '\\' => sink.emit(br"\\")?,
            '\u{0008}' => sink.emit(br"\b")?,
            '\u{000c}' => sink.emit(br"\f")?,
            '\n' => sink.emit(br"\n")?,
            '\r' => sink.emit(br"\r")?,
            '\t' => sink.emit(br"\t")?,
            '\u{0000}'..='\u{001f}' => {
                let code = u8::try_from(u32::from(character))
                    .map_err(|_scalar| {
                        ProfileManifestJsonWriteError::InternalEncoding
                    })?;
                sink.emit(&[
                    b'\\', b'u', b'0', b'0',
                    hex_digit(code >> 4u32)?, hex_digit(code & 15u8)?,
                ])?;
            }
            _ => {
                let mut buffer = [0u8; 4];
                sink.emit(character.encode_utf8(&mut buffer).as_bytes())?;
            }
        }
    }
    sink.emit(b"\"")
}

#[expect(
    clippy::integer_division_remainder_used,
    reason = "base-ten digit encoding requires exact quotient and remainder"
)]
fn append_decimal<S>(
    sink: &mut S,
    mut value: u64,
) -> Result<(), ProfileManifestJsonWriteError>
where
    S: JsonSink,
{
    // Every u64 fits in 20 decimal digits. Count and emission must not
    // allocate a temporary String per declared entry byte length.
    let mut digits = [0u8; 20];
    let mut start = digits.len();
    loop {
        start = start.checked_sub(1)
            .ok_or(ProfileManifestJsonWriteError::InternalEncoding)?;
        let digit = u8::try_from(value % 10u64)
            .map_err(|_value| ProfileManifestJsonWriteError::InternalEncoding)?;
        *digits.get_mut(start)
            .ok_or(ProfileManifestJsonWriteError::InternalEncoding)? =
            b'0'.checked_add(digit)
                .ok_or(ProfileManifestJsonWriteError::InternalEncoding)?;
        value /= 10u64;
        if value == 0 {
            break;
        }
    }
    sink.emit(digits.get(start..)
        .ok_or(ProfileManifestJsonWriteError::InternalEncoding)?)
}

fn hex_digit(value: u8) -> Result<u8, ProfileManifestJsonWriteError> {
    b"0123456789abcdef"
        .get(usize::from(value))
        .copied()
        .ok_or(ProfileManifestJsonWriteError::InternalEncoding)
}

fn append_digest<S>(
    sink: &mut S,
    digest: [u8; 32],
) -> Result<(), ProfileManifestJsonWriteError>
where
    S: JsonSink,
{
    let mut encoded = [0u8; 64];
    let (pairs, remainder) = encoded.as_chunks_mut::<2>();
    debug_assert!(remainder.is_empty(), "SHA-256 hex pairs must fit");
    for (pair, byte) in pairs.iter_mut().zip(digest) {
        let [high, low] = pair;
        *high = hex_digit(byte >> 4u32)?;
        *low = hex_digit(byte & 0x0fu8)?;
    }
    sink.emit(b"\"")?;
    sink.emit(&encoded)?;
    sink.emit(b"\"")
}

fn append_strings<S>(
    sink: &mut S,
    strings: &[&str],
) -> Result<(), ProfileManifestJsonWriteError>
where
    S: JsonSink,
{
    sink.emit(b"[")?;
    for (index, value) in strings.iter().enumerate() {
        if index > 0 {
            sink.emit(b",")?;
        }
        append_json_string(sink, value)?;
    }
    sink.emit(b"]")
}

fn append_manifest<S>(
    sink: &mut S,
    manifest: &ProfileManifest,
    entries: &[&ProfileManifestEntry],
    required: &[&str],
    optional: &[&str],
) -> Result<(), ProfileManifestJsonWriteError>
where
    S: JsonSink,
{
    sink.emit(b"{\"container_version\":")?;
    append_json_string(sink, &manifest.container_version)?;
    sink.emit(b",\"entries\":[")?;
    for (index, entry) in entries.iter().enumerate() {
        if index > 0 {
            sink.emit(b",")?;
        }
        sink.emit(b"{\"byte_length\":")?;
        append_decimal(sink, entry.byte_length)?;
        sink.emit(b",\"media_type\":")?;
        append_json_string(sink, &entry.media_type)?;
        sink.emit(b",\"path\":")?;
        append_json_string(sink, &entry.path)?;
        sink.emit(b",\"sha256\":")?;
        append_digest(sink, entry.digest.bytes())?;
        sink.emit(b"}")?;
    }
    sink.emit(b"],\"optional_features\":")?;
    append_strings(sink, optional)?;
    sink.emit(b",\"profile_identity\":")?;
    append_json_string(sink, &manifest.profile_identity)?;
    sink.emit(b",\"required_features\":")?;
    append_strings(sink, required)?;
    sink.emit(b"}")
}

fn sorted_borrowed<'item, T>(
    source: impl ExactSizeIterator<Item = &'item T>,
    compare: impl Fn(&T, &T) -> Ordering,
) -> Result<Vec<&'item T>, ProfileManifestJsonWriteError>
where
    T: ?Sized,
{
    let mut sorted = Vec::new();
    sorted
        .try_reserve_exact(source.len())
        .map_err(|_allocation| {
            ProfileManifestJsonWriteError::AllocationFailed
        })?;
    sorted.extend(source);
    sorted.sort_unstable_by(|left, right| compare(left, right));
    Ok(sorted)
}

fn plan_canonical_manifest<'manifest>(
    manifest: &'manifest ProfileManifest,
    supported_required_features: &[&str],
    limits: ProfileManifestJsonWriteLimits,
) -> Result<CanonicalManifestPlan<'manifest>, ProfileManifestJsonWriteError> {
    if manifest.entries.len() > limits.maximum_entries {
        return Err(ProfileManifestJsonWriteError::ArrayLimitExceeded(
            "entries",
        ));
    }
    if manifest.required_features.len() > limits.maximum_features {
        return Err(ProfileManifestJsonWriteError::ArrayLimitExceeded(
            "required_features",
        ));
    }
    if manifest.optional_features.len() > limits.maximum_features {
        return Err(ProfileManifestJsonWriteError::ArrayLimitExceeded(
            "optional_features",
        ));
    }
    checked_string(&manifest.container_version, "container_version", limits)?;
    checked_string(&manifest.profile_identity, "profile_identity", limits)?;
    for feature in &manifest.required_features {
        checked_string(feature, "required_features", limits)?;
    }
    for feature in &manifest.optional_features {
        checked_string(feature, "optional_features", limits)?;
    }
    for entry in &manifest.entries {
        checked_string(&entry.path, "entries[].path", limits)?;
        checked_string(&entry.media_type, "entries[].media_type", limits)?;
    }
    validate_profile_manifest(manifest, supported_required_features)
        .map_err(ProfileManifestJsonWriteError::Manifest)?;
    let entries = sorted_borrowed(manifest.entries.iter(), |left, right| {
        left.path.cmp(&right.path)
    })?;
    let required = sorted_borrowed(
        manifest.required_features.iter().map(String::as_str),
        str::cmp,
    )?;
    let optional = sorted_borrowed(
        manifest.optional_features.iter().map(String::as_str),
        str::cmp,
    )?;
    let mut counter = JsonCounter {
        count: 0,
        limit: limits.maximum_bytes,
    };
    append_manifest(&mut counter, manifest, &entries, &required, &optional)?;
    Ok(CanonicalManifestPlan {
        bytes: counter.count,
        entries,
        manifest,
        optional,
        required,
    })
}

/// Return the exact canonical JSON byte length without allocating output.
///
/// The same typed admission, list, string, and byte-budget checks as the
/// allocating writer run before producing a count. Sorting may reserve
/// bounded metadata references but never allocates serialized output.
///
/// # Errors
///
/// Refuses the same invalid inputs as the two canonical JSON writers.
pub fn planned_canonical_profile_manifest_json_bytes(
    manifest: &ProfileManifest,
    supported_required_features: &[&str],
    limits: ProfileManifestJsonWriteLimits,
) -> Result<usize, ProfileManifestJsonWriteError> {
    Ok(plan_canonical_manifest(
        manifest, supported_required_features, limits,
    )?.bytes)
}

/// Write canonical JSON into caller-owned memory with no output allocation.
///
/// A short destination is rejected with the exact required byte count before
/// touching any slot. Extra capacity is not changed. All admission, limits,
/// and sorting checks precede emission; this function cannot create an
/// authorized Export or verify accompanying section payloads.
///
/// # Errors
///
/// Refuses invalid manifests or inadequate destination capacity before writing.
pub fn write_canonical_profile_manifest_json_into(
    manifest: &ProfileManifest,
    supported_required_features: &[&str],
    limits: ProfileManifestJsonWriteLimits,
    destination: &mut [u8],
) -> Result<usize, ProfileManifestJsonWriteError> {
    let plan = plan_canonical_manifest(
        manifest, supported_required_features, limits,
    )?;
    if destination.len() < plan.bytes {
        return Err(ProfileManifestJsonWriteError::DestinationTooSmall {
            required: plan.bytes,
        });
    }
    let mut sink = JsonSliceSink {
        buffer: destination,
        position: 0,
    };
    plan.emit(&mut sink)?;
    debug_assert_eq!(
        sink.position, plan.bytes,
        "caller-owned emission must match exact bounded JSON plan",
    );
    Ok(sink.position)
}

/// Serialize a validated first-release manifest as deterministic UTF-8 JSON.
///
/// Object keys use ASCII lexical order, non-manifest entries sort by exact
/// path bytes, feature lists sort by UTF-8 bytes, and unsigned sizes use plain
/// decimal digits. Strings retain literal UTF-8 except JSON-required escapes,
/// encoded without optional whitespace or a trailing newline. This is a
/// first-release writer policy: section payloads and ZIP are separate.
///
/// All declaration and size checks, including an exact JSON byte count, run
/// before allocating the final output vector. An invalid manifest or
/// insufficient budget yields no partial serialized output. Required feature
/// support is independently supplied by the caller; source values confer no
/// filesystem Export intent and do not verify declared payload hashes.
///
/// # Errors
///
/// Refuses list/string/output limits and existing typed manifest failures.
pub fn write_canonical_profile_manifest_json(
    manifest: &ProfileManifest,
    supported_required_features: &[&str],
    limits: ProfileManifestJsonWriteLimits,
) -> Result<Vec<u8>, ProfileManifestJsonWriteError> {
    let plan = plan_canonical_manifest(
        manifest, supported_required_features, limits,
    )?;
    let mut output = Vec::new();
    output
        .try_reserve_exact(plan.bytes)
        .map_err(|_allocation| {
            ProfileManifestJsonWriteError::AllocationFailed
        })?;
    plan.emit(&mut output)?;
    debug_assert_eq!(
        output.len(), plan.bytes,
        "emission must match exact bounded JSON plan",
    );
    Ok(output)
}
