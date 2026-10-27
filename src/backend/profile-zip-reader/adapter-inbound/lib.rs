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
//   - Bounded, exact, read-only parsing of ordinary stored-only ZIP framing
//     and local-versus-central-directory entry consistency.
// - Must-Not:
//   - Open files, accept compression or ZIP64, decode profile JSON, infer
//     trust in a source path, or authorize importing or persisting a profile.
// - Allows:
//   - Inputs: One already bounded byte slice and explicit resource limits.
//   - Outputs: Borrowed entry names and checked stored member byte slices.
//   - Side effects: Small bounded index allocations only.
// - Split-When:
//   - ZIP64 or streaming resource admission needs a separate implementation.
// - Merge-When:
//   - A complete verified portable-profile archive adapter replaces this one.
// - Summary:
//   - Rejects malformed or ambiguous ZIP frames before manifest admission.
// - Description:
//   - Supports ordinary ZIP only; rejects features it cannot safely prove.
// - Usage:
//   - Pair with typed manifest, SHA-256, and path admission before import.
// - Defaults:
//   - Unsupported ZIP syntax is a typed failure, not a fallback parse.
//

//! Read-only, bounded ordinary stored ZIP framing for portable profiles.

use std::num::NonZeroUsize;
use std::slice::Chunks;
use std::str::from_utf8;

use atrament_handwriting_profile_container::{
    PROFILE_MANIFEST_PATH, ProfileEntryVerificationError,
    ProfileManifest, Sha256Digest, ValidatedProfileEntryInventory,
    profile_entry_kind,
};
use atrament_profile_entry_sha256::{
    sha256_profile_bytes, verify_profile_entry_bytes,
};
use atrament_profile_json_syntax::{
    ProfileJsonSyntaxError, ProfileJsonSyntaxLimits,
    ValidatedProfileJsonObject, validate_profile_json_object,
};

const END_SIGNATURE: u32 = 0x0605_4b50;
const CENTRAL_SIGNATURE: u32 = 0x0201_4b50;
const LOCAL_SIGNATURE: u32 = 0x0403_4b50;
const END_SIZE: usize = 22;
const CENTRAL_SIZE: usize = 46;
const LOCAL_SIZE: usize = 30;

/// Explicit limits on the archive byte slice and its stored entries.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProfileZipReadLimits {
    /// Maximum complete ZIP slice size, including all headers.
    pub maximum_archive_bytes: usize,
    /// Maximum number of entries, including root `manifest.json`.
    pub maximum_entries: usize,
    /// Maximum individual stored-entry bytes.
    pub maximum_entry_bytes: u64,
    /// Maximum sum of stored-entry bytes.
    pub maximum_total_bytes: u64,
}

/// One typed rejection of an unsupported or inconsistent ZIP framing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileZipReadError {
    /// The bounded member index could not be allocated.
    AllocationFailed,
    /// Complete supplied ZIP byte slice exceeds a caller-set bound.
    ArchiveTooLarge,
    /// Stored CRC-32 disagrees with independently checked member bytes.
    BadCrc32,
    /// Root manifest member bytes cannot be interpreted as UTF-8.
    InvalidManifestUtf8,
    /// Archive member name is empty, a directory, or not UTF-8.
    InvalidName,
    /// End record, central directory, or local header is inconsistent.
    InvalidStructure,
    /// The root manifest exceeds its independently specified byte budget.
    ManifestTooLarge,
    /// No exact root `manifest.json` member exists in the ZIP.
    MissingManifest,
    /// File ordering, trailing bytes, or foreign platform metadata is present.
    NonCanonical,
    /// Declared entries or member sizes exceed an explicit caller limit.
    ResourceLimit,
    /// A header or member payload terminates outside the ZIP slice.
    Truncated,
    /// Encrypted, compressed, descriptor or other unsupported ZIP feature.
    UnsupportedFeature,
    /// ZIP64 syntax or sentinel values require another, bounded adapter.
    UnsupportedZip64,
}

/// Root JSON syntax failure after ordinary stored-ZIP framing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileZipManifestSyntaxError {
    /// Root JSON is not one complete bounded UTF-8 object.
    Json(ProfileJsonSyntaxError),
    /// ZIP root manifest is absent, not UTF-8, or exceeds its own byte bound.
    Zip(ProfileZipReadError),
}

/// One borrowed member with verified local/central size and CRC evidence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProfileZipEntry<'bytes> {
    bytes: &'bytes [u8],
    crc32: u32,
    name: &'bytes str,
}

impl<'bytes> ProfileZipEntry<'bytes> {
    /// Exact borrowed uncompressed stored payload bytes.
    #[must_use]
    pub const fn bytes(self) -> &'bytes [u8] {
        self.bytes
    }

    /// CRC-32 verified against the archive header's declared checksum.
    #[must_use]
    pub const fn crc32(self) -> u32 {
        self.crc32
    }

    /// Exact UTF-8 archive entry name, without URI or path interpretation.
    #[must_use]
    pub const fn name(self) -> &'bytes str {
        self.name
    }
}

/// Constructor-sealed ordinary stored ZIP index, not an imported profile.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProfileZipArchive<'bytes> {
    entries: Vec<ProfileZipEntry<'bytes>>,
    source: &'bytes [u8],
}

/// Constructor-sealed pairing of CRC-checked ZIP framing and its exact,
/// resource-bounded, syntax-validated `manifest.json` root object.
///
/// The archive borrow prevents substituting an unrelated archive while
/// downstream code inspects this root. It does not prove that an independently
/// constructed typed manifest was decoded from these bytes; that separate
/// admission and authorization gate remains unimplemented.
#[derive(Debug)]
pub struct ValidatedProfileZipRootJson<'archive, 'bytes> {
    archive: &'archive ProfileZipArchive<'bytes>,
    root: ValidatedProfileJsonObject<'bytes>,
}

impl<'archive, 'bytes> ValidatedProfileZipRootJson<'archive, 'bytes> {
    /// Borrow the same ZIP index that supplied these admitted root bytes.
    #[must_use]
    pub const fn archive(&self) -> &'archive ProfileZipArchive<'bytes> {
        self.archive
    }

    /// Borrow the constructor-sealed root JSON syntax view from that ZIP.
    #[must_use]
    pub const fn root(&self) -> ValidatedProfileJsonObject<'bytes> {
        self.root
    }

    /// SHA-256 fingerprint of this archive's exact syntax-admitted root bytes.
    ///
    /// Unlike a CRC-32 framing check, this digest identifies the admitted
    /// source bytes without JSON reserialization or name normalization.
    /// It does **not** verify a declared root hash, decode a typed manifest,
    /// bind an imported file path, or grant Import/Export authorization.
    #[must_use]
    pub fn source_sha256(&self) -> Sha256Digest {
        sha256_profile_bytes(self.root.bytes())
    }
}

impl<'bytes> ProfileZipArchive<'bytes> {
    /// Pair this checked ZIP archive with exact admitted root JSON syntax.
    ///
    /// The returned view keeps the archive association in one sealed value,
    /// not just an independent typed manifest or detached JSON byte range.
    /// No decoded manifest fields, entry hashes or file authority are proved.
    ///
    /// # Errors
    ///
    /// Reports the existing root size, UTF-8, JSON grammar and budget errors
    /// without exposing a partial paired view.
    pub fn admitted_root_json(
        &self,
        limits: ProfileJsonSyntaxLimits,
    ) -> Result<ValidatedProfileZipRootJson<'_, 'bytes>,
        ProfileZipManifestSyntaxError> {
        let root = self.manifest_json_object(limits)?;
        Ok(ValidatedProfileZipRootJson { archive: self, root })
    }

    /// Borrow entries in the canonical byte order already validated.
    #[must_use]
    pub fn entries(&self) -> &[ProfileZipEntry<'bytes>] {
        &self.entries
    }

    /// Admit exact root manifest JSON object syntax from this ZIP's bytes.
    ///
    /// Checks the independent manifest byte budget first, then strict JSON
    /// grammar, unique decoded keys, nesting, and value/member ceilings
    /// before any typed field decoding.
    /// This does not bind arbitrary typed manifest data to the root JSON.
    ///
    /// # Errors
    ///
    /// Rejects missing, excessive or invalid UTF-8 root bytes and malformed
    /// or over-budget JSON syntax without exposing any typed manifest.
    pub fn manifest_json_object(
        &self,
        limits: ProfileJsonSyntaxLimits,
    ) -> Result<
        ValidatedProfileJsonObject<'bytes>,
        ProfileZipManifestSyntaxError,
    > {
        let root = self.manifest_utf8(limits.maximum_bytes)
            .map_err(ProfileZipManifestSyntaxError::Zip)?;
        validate_profile_json_object(root.as_bytes(), limits)
            .map_err(ProfileZipManifestSyntaxError::Json)
    }

    /// Borrow exact root manifest UTF-8 without attempting JSON decoding.
    ///
    /// This limit is independent of the complete archive resource budget:
    /// downstream JSON parsing must still implement its own depth and
    /// allocation ceilings and validate typed manifest schema and digests.
    ///
    /// # Errors
    ///
    /// Fails if the root is absent, too large, or not valid UTF-8.
    pub fn manifest_utf8(
        &self,
        maximum_bytes: usize,
    ) -> Result<&'bytes str, ProfileZipReadError> {
        // The admitted index is strictly sorted by exact UTF-8 name bytes.
        // Use that same ordering as verified payload lookup; near matches
        // must not supply the root manifest's source-byte provenance.
        let index = self.entries.binary_search_by(
            |entry| entry.name.cmp(PROFILE_MANIFEST_PATH)
        ).map_err(|_insertion_index| ProfileZipReadError::MissingManifest)?;
        let entry = self.entries.get(index)
            .ok_or(ProfileZipReadError::MissingManifest)?;
        if entry.bytes.len() > maximum_bytes {
            return Err(ProfileZipReadError::ManifestTooLarge);
        }
        from_utf8(entry.bytes).ok()
            .ok_or(ProfileZipReadError::InvalidManifestUtf8)
    }

    /// Extract borrowed exact archive names for manifest inventory preflight.
    #[must_use]
    pub fn observed_paths(&self) -> Vec<&'bytes str> {
        self.entries.iter().map(|entry| entry.name).collect()
    }

    /// Borrow the complete original source bytes of this checked ZIP frame.
    ///
    /// The slice is exactly the caller-provided archive without any copy or
    /// normalization; only stored-ZIP framing/CRC checks have succeeded.
    /// No typed manifest or selected filesystem identity is established.
    #[must_use]
    pub const fn source_bytes(&self) -> &'bytes [u8] {
        self.source
    }

    /// SHA-256 fingerprint of the full checked ZIP byte slice, not its root.
    ///
    /// Unlike `ValidatedProfileZipRootJson::source_sha256`, this includes
    /// local headers, CRC fields, every member, the directory and end record.
    /// It is not an authenticity signature, typed manifest acceptance, or
    /// permission to read or write a selected path.
    #[must_use]
    pub fn source_sha256(&self) -> Sha256Digest {
        sha256_profile_bytes(self.source)
    }
}

/// Failed binding of admitted names or exact member bytes to a stored ZIP.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileZipPayloadVerificationError {
    /// One non-manifest member's bytes fail their declared size or SHA-256.
    Entry {
        /// Zero-based index in the admitted manifest entry sequence.
        index: usize,
        /// Byte-length checks precede digest checks.
        reason: ProfileEntryVerificationError,
    },
    /// The admitted inventory is not the exact member-name sequence here.
    InventoryMismatch,
    /// A declared member cannot be found after exact inventory comparison.
    MissingAfterAdmission {
        /// Zero-based index in the admitted manifest.
        index: usize,
    },
}

/// Constructor-sealed, borrowed verified ZIP member bytes.
///
/// This does not prove the supplied manifest was parsed from this ZIP root,
/// nor grant path authority, decode sections or commit an imported profile.
#[derive(Debug)]
pub struct VerifiedProfileZipPayloads<'manifest, 'archive, 'bytes> {
    archive: &'archive ProfileZipArchive<'bytes>,
    manifest: &'manifest ProfileManifest,
}

impl<'manifest, 'bytes> VerifiedProfileZipPayloads<'manifest, '_, 'bytes> {
    /// Borrow the exact manifest whose entry declarations were checked.
    #[must_use]
    pub const fn manifest(&self) -> &'manifest ProfileManifest {
        self.manifest
    }

    /// Return a declared, SHA-256-checked non-manifest entry's exact bytes.
    ///
    /// No name normalization, percent decoding or filesystem lookup occurs.
    #[must_use]
    pub fn verified_entry_bytes(&self, path: &str) -> Option<&'bytes [u8]> {
        // Admission proved that every non-manifest archive member is
        // declared, and verification checked all those exact bytes.
        // The ZIP index is strictly sorted, so no second linear name scan
        // or independently mutable declaration cache is required.
        if path == PROFILE_MANIFEST_PATH {
            return None;
        }
        let index = self.archive.entries.binary_search_by(
            |entry| entry.name().cmp(path)
        ).ok()?;
        self.archive.entries.get(index).map(|entry| entry.bytes())
    }

    /// Borrow fixed-size chunks of one completely SHA-256-verified member.
    ///
    /// The chunk size is nonzero by type, so no unchecked `chunks(0)` panic
    /// can occur. Slices remain borrowed from the same stored ZIP payload as
    /// `verified_entry_bytes`, with no new allocation or decode. Missing,
    /// undeclared and root-manifest names never yield verified chunks.
    /// This does not create a streaming file transport or typed section.
    #[must_use]
    pub fn verified_entry_chunks(
        &self,
        path: &str,
        chunk_size: NonZeroUsize,
    ) -> Option<Chunks<'bytes, u8>> {
        Some(self.verified_entry_bytes(path)?.chunks(chunk_size.get()))
    }

    /// Number of non-manifest member payloads verified.
    #[must_use]
    pub const fn verified_entry_count(&self) -> usize {
        self.manifest.entries.len()
    }
}

#[derive(Clone, Copy)]
struct CentralEntry<'bytes> {
    crc32: u32,
    flags: u16,
    length: u32,
    local_offset: u32,
    method: u16,
    modified_date: u16,
    modified_time: u16,
    name: &'bytes str,
    name_bytes: &'bytes [u8],
    next_cursor: usize,
    version: u16,
}

fn bytes_at(bytes: &[u8], offset: usize, count: usize) -> Option<&[u8]> {
    bytes.get(offset..offset.checked_add(count)?)
}

fn u16_at(bytes: &[u8], offset: usize) -> Option<u16> {
    let value: [u8; 2] = bytes_at(bytes, offset, 2)?.try_into().ok()?;
    Some(u16::from_le_bytes(value))
}

fn u32_at(bytes: &[u8], offset: usize) -> Option<u32> {
    let value: [u8; 4] = bytes_at(bytes, offset, 4)?.try_into().ok()?;
    Some(u32::from_le_bytes(value))
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0u8..8u8 {
            crc = if crc & 1u32 == 0 {
                crc >> 1u32
            } else {
                (crc >> 1u32) ^ 0xedb8_8320
            };
        }
    }
    !crc
}

fn parse_central_entry(
    bytes: &[u8],
    cursor: usize,
) -> Result<CentralEntry<'_>, ProfileZipReadError> {
    let _header = bytes_at(bytes, cursor, CENTRAL_SIZE)
        .ok_or(ProfileZipReadError::Truncated)?;
    if u32_at(bytes, cursor) != Some(CENTRAL_SIGNATURE) {
        return Err(ProfileZipReadError::InvalidStructure);
    }
    let get16 = |offset| u16_at(bytes, cursor.saturating_add(offset))
        .ok_or(ProfileZipReadError::Truncated);
    let get32 = |offset| u32_at(bytes, cursor.saturating_add(offset))
        .ok_or(ProfileZipReadError::Truncated);
    let creator = get16(4)?;
    let version = get16(6)?;
    let flags = get16(8)?;
    let method = get16(10)?;
    let modified_time = get16(12)?;
    let modified_date = get16(14)?;
    let crc32 = get32(16)?;
    let compressed = get32(20)?;
    let length = get32(24)?;
    let name_length = get16(28)?;
    let extras = get16(30)?;
    let comment = get16(32)?;
    let disk = get16(34)?;
    let internal_attr = get16(36)?;
    let external_attr = get32(38)?;
    let local_offset = get32(42)?;
    if [compressed, length, local_offset].contains(&u32::MAX) {
        return Err(ProfileZipReadError::UnsupportedZip64);
    }
    if flags & !0x0800 != 0 || method != 0 || !(10..=20).contains(&version) {
        return Err(ProfileZipReadError::UnsupportedFeature);
    }
    if extras != 0 || comment != 0 || creator >> 8u16 != 0
        || disk != 0 || internal_attr != 0 || external_attr != 0
    {
        return Err(ProfileZipReadError::NonCanonical);
    }
    if compressed != length {
        return Err(ProfileZipReadError::InvalidStructure);
    }
    let name_start = cursor.checked_add(CENTRAL_SIZE)
        .ok_or(ProfileZipReadError::ResourceLimit)?;
    let name_bytes = bytes_at(bytes, name_start, usize::from(name_length))
        .ok_or(ProfileZipReadError::Truncated)?;
    let name = from_utf8(name_bytes).ok()
        .ok_or(ProfileZipReadError::InvalidName)?;
    if name.is_empty() || name.ends_with('/') || name.contains('\0') {
        return Err(ProfileZipReadError::InvalidName);
    }
    if !name.is_ascii() && flags & 0x0800 == 0 {
        return Err(ProfileZipReadError::UnsupportedFeature);
    }
    if name != PROFILE_MANIFEST_PATH && profile_entry_kind(name).is_err() {
        return Err(ProfileZipReadError::InvalidName);
    }
    let next_cursor = name_start.checked_add(name_bytes.len())
        .ok_or(ProfileZipReadError::ResourceLimit)?;
    Ok(CentralEntry {
        crc32,
        flags,
        length,
        local_offset,
        method,
        modified_date,
        modified_time,
        name,
        name_bytes,
        next_cursor,
        version,
    })
}

fn read_local_entry<'bytes>(
    bytes: &'bytes [u8],
    central: &CentralEntry<'bytes>,
    local_end: usize,
    directory_start: usize,
) -> Result<&'bytes [u8], ProfileZipReadError> {
    let local = usize::try_from(central.local_offset).ok()
        .ok_or(ProfileZipReadError::ResourceLimit)?;
    if local != local_end {
        return Err(ProfileZipReadError::NonCanonical);
    }
    let _header = bytes_at(bytes, local, LOCAL_SIZE)
        .ok_or(ProfileZipReadError::Truncated)?;
    let get16 = |offset| u16_at(bytes, local.saturating_add(offset));
    let get32 = |offset| u32_at(bytes, local.saturating_add(offset));
    if u32_at(bytes, local) != Some(LOCAL_SIGNATURE)
        || get16(4) != Some(central.version)
        || get16(6) != Some(central.flags)
        || get16(8) != Some(central.method)
        || get16(10) != Some(central.modified_time)
        || get16(12) != Some(central.modified_date)
        || get32(14) != Some(central.crc32)
        || get32(18) != Some(central.length)
        || get32(22) != Some(central.length)
        || get16(26) != u16::try_from(central.name_bytes.len()).ok()
        || get16(28) != Some(0)
    {
        return Err(ProfileZipReadError::InvalidStructure);
    }
    let name_start = local.checked_add(LOCAL_SIZE)
        .ok_or(ProfileZipReadError::ResourceLimit)?;
    if bytes_at(bytes, name_start, central.name_bytes.len())
        != Some(central.name_bytes)
    {
        return Err(ProfileZipReadError::InvalidStructure);
    }
    let data_start = name_start.checked_add(central.name_bytes.len())
        .ok_or(ProfileZipReadError::ResourceLimit)?;
    let data_length = usize::try_from(central.length).ok()
        .ok_or(ProfileZipReadError::ResourceLimit)?;
    let payload = bytes_at(bytes, data_start, data_length)
        .ok_or(ProfileZipReadError::Truncated)?;
    let data_end = data_start.checked_add(data_length)
        .ok_or(ProfileZipReadError::ResourceLimit)?;
    if data_end > directory_start {
        return Err(ProfileZipReadError::InvalidStructure);
    }
    if crc32(payload) != central.crc32 {
        return Err(ProfileZipReadError::BadCrc32);
    }
    Ok(payload)
}

/// Parse one complete bounded ordinary ZIP archive with stored-only members.
///
/// Fails closed on ZIP64, multi-disk, encryption, descriptors, compression,
/// extras, comments, unknown flags, platform-specific attributes, unordered
/// members, overlapping entries, unexpected prefixes, and unclaimed trailing
/// bytes. Local headers must exactly agree with central metadata. Each stored
/// member's CRC is recomputed from its bounded borrowed byte range.
///
/// This reader does not parse `manifest.json`, compute SHA-256, decode typed
/// sections, or imply that any file on disk has been imported. Its input bytes
/// must already have come from a trusted, resource-bounded caller.
///
/// # Errors
///
/// Returns a typed framing, integrity, or resource-limit error. Unsupported
/// syntax is never guessed or silently normalized.
pub fn read_stored_profile_zip(
    bytes: &[u8],
    limits: ProfileZipReadLimits,
) -> Result<ProfileZipArchive<'_>, ProfileZipReadError> {
    if bytes.len() > limits.maximum_archive_bytes {
        return Err(ProfileZipReadError::ArchiveTooLarge);
    }
    let trailer = bytes.len().checked_sub(END_SIZE)
        .ok_or(ProfileZipReadError::Truncated)?;
    if u32_at(bytes, trailer) != Some(END_SIGNATURE)
        || u16_at(bytes, trailer.saturating_add(20)) != Some(0)
    {
        return Err(ProfileZipReadError::InvalidStructure);
    }
    let get16 = |offset| u16_at(bytes, trailer.saturating_add(offset))
        .ok_or(ProfileZipReadError::Truncated);
    let get32 = |offset| u32_at(bytes, trailer.saturating_add(offset))
        .ok_or(ProfileZipReadError::Truncated);
    if get16(4)? != 0 || get16(6)? != 0 {
        return Err(ProfileZipReadError::UnsupportedFeature);
    }
    let disk_entries = get16(8)?;
    let all_entries = get16(10)?;
    let directory_size = get32(12)?;
    let directory_offset = get32(16)?;
    if [disk_entries, all_entries].contains(&u16::MAX)
        || [directory_size, directory_offset].contains(&u32::MAX)
    {
        return Err(ProfileZipReadError::UnsupportedZip64);
    }
    if disk_entries != all_entries {
        return Err(ProfileZipReadError::UnsupportedFeature);
    }
    if usize::from(all_entries) > limits.maximum_entries {
        return Err(ProfileZipReadError::ResourceLimit);
    }
    let directory_start = usize::try_from(directory_offset).ok()
        .ok_or(ProfileZipReadError::ResourceLimit)?;
    let central_size = usize::try_from(directory_size).ok()
        .ok_or(ProfileZipReadError::ResourceLimit)?;
    if directory_start.checked_add(central_size) != Some(trailer) {
        return Err(ProfileZipReadError::InvalidStructure);
    }
    // Do not let a tiny forged EOCD allocate thousands of phantom entries.
    let minimum_directory_bytes = usize::from(all_entries)
        .checked_mul(CENTRAL_SIZE)
        .ok_or(ProfileZipReadError::ResourceLimit)?;
    if minimum_directory_bytes > central_size {
        return Err(ProfileZipReadError::InvalidStructure);
    }
    let mut cursor = directory_start;
    let mut local_end = 0usize;
    let mut total_bytes = 0u64;
    // Fallibly reserve exactly the validated member count. Never let an
    // attacker-selected count enter an infallible allocation path.
    let mut entries = Vec::new();
    entries.try_reserve_exact(usize::from(all_entries))
        .map_err(|_allocation| ProfileZipReadError::AllocationFailed)?;
    let mut previous_name: Option<&str> = None;
    for _ in 0..usize::from(all_entries) {
        let central = parse_central_entry(bytes, cursor)?;
        if previous_name.is_some_and(|name| name >= central.name) {
            return Err(ProfileZipReadError::NonCanonical);
        }
        if u64::from(central.length) > limits.maximum_entry_bytes {
            return Err(ProfileZipReadError::ResourceLimit);
        }
        total_bytes = total_bytes.checked_add(u64::from(central.length))
            .ok_or(ProfileZipReadError::ResourceLimit)?;
        if total_bytes > limits.maximum_total_bytes {
            return Err(ProfileZipReadError::ResourceLimit);
        }
        let payload = read_local_entry(bytes, &central, local_end,
            directory_start)?;
        local_end = usize::try_from(central.local_offset).ok()
            .and_then(|local| local.checked_add(LOCAL_SIZE))
            .and_then(|start| start.checked_add(central.name_bytes.len()))
            .and_then(|start| start.checked_add(payload.len()))
            .ok_or(ProfileZipReadError::ResourceLimit)?;
        entries.push(ProfileZipEntry {
            bytes: payload,
            crc32: central.crc32,
            name: central.name,
        });
        previous_name = Some(central.name);
        cursor = central.next_cursor;
    }
    if cursor != trailer || local_end != directory_start {
        return Err(ProfileZipReadError::InvalidStructure);
    }
    Ok(ProfileZipArchive { entries, source: bytes })
}

/// Verify all admitted member bytes against one exact ZIP archive.
///
/// The already sealed inventory comes from independent manifest admission.
/// Names must match this archive again before any digest is computed, so an
/// admission from a different ZIP cannot be substituted. All entries must
/// verify before the result exposes even one byte to a future decoder.
///
/// The source of the typed manifest and file path remains unproven here.
///
/// # Errors
///
/// Returns an inventory mismatch before any member verification, followed by
/// the first declaration-order member byte-length or SHA-256 failure.
pub fn verify_admitted_profile_zip_payloads<'manifest, 'archive, 'bytes>(
    archive: &'archive ProfileZipArchive<'bytes>,
    inventory: ValidatedProfileEntryInventory<'manifest, '_>,
) -> Result<
    VerifiedProfileZipPayloads<'manifest, 'archive, 'bytes>,
    ProfileZipPayloadVerificationError,
> {
    if archive.entries.len() != inventory.observed_paths().len() {
        return Err(ProfileZipPayloadVerificationError::InventoryMismatch);
    }
    for (entry, name) in
        archive.entries.iter().zip(inventory.observed_paths())
    {
        if entry.name() != *name {
            return Err(ProfileZipPayloadVerificationError::InventoryMismatch);
        }
    }
    for (index, declaration) in
        inventory.manifest().entries.iter().enumerate()
    {
        let member_index = archive.entries.binary_search_by(
            |entry| entry.name().cmp(declaration.path.as_str())
        ).map_err(|_position| {
            ProfileZipPayloadVerificationError::MissingAfterAdmission { index }
        })?;
        let member = archive.entries.get(member_index)
            .ok_or(
                ProfileZipPayloadVerificationError::MissingAfterAdmission {
                    index,
                },
            )?;
        let _evidence = verify_profile_entry_bytes(declaration, member.bytes())
            .map_err(|reason| ProfileZipPayloadVerificationError::Entry {
                index, reason,
            })?;
    }
    Ok(VerifiedProfileZipPayloads {
        archive,
        manifest: inventory.manifest(),
    })
}
