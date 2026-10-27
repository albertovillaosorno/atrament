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
//   - Deterministic, ordinary stored-only ZIP framing of caller-supplied
//     complete member bytes after explicit resource and ZIP32 preflight.
// - Must-Not:
//   - Choose manifest field names, parse profile JSON, accept arbitrary
//     archive paths, emit ZIP64, open files, or commit an exported profile.
// - Allows:
//   - Inputs: Exact sorted member names and bytes under caller-owned limits.
//   - Outputs: An in-memory byte vector of ordinary ZIP framing or a failure.
//   - Side effects: One bounded output allocation, no filesystem effect.
// - Split-When:
//   - ZIP64 and streaming writers need independent outbound adapters.
// - Merge-When:
//   - Full canonical portable-profile encoding owns the complete writer.
// - Summary:
//   - Emits fixed-metadata stored ZIP framing for already selected bytes.
// - Description:
//   - All size, name and order checks precede any output allocation.
// - Usage:
//   - Use only after future typed serialization and Export authorization.
// - Defaults:
//   - Missing roots, invalid names and ZIP64-size records fail closed.
//

//! Deterministic ordinary stored-ZIP frames, not a complete profile writer.

use atrament_handwriting_profile_container::{
    PROFILE_MANIFEST_PATH, profile_entry_kind,
};

const CENTRAL_HEADER_BYTES: usize = 46;
const END_BYTES: usize = 22;
const LOCAL_HEADER_BYTES: usize = 30;
const LOCAL_CRC32_OFFSET: usize = 14;
const MAX_ZIP32_VALUE: u32 = u32::MAX - 1;

/// Already serialized entry name and complete payload bytes, not a profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProfileStoredZipMember<'bytes> {
    /// Exact uncompressed bytes to store without interpreting their contents.
    pub bytes: &'bytes [u8],
    /// Exact archive path. Only the root manifest or valid profile paths pass.
    pub name: &'bytes str,
}

/// Independent bounds for serialized archive bytes and its member payloads.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProfileStoredZipWriteLimits {
    /// Maximum whole output archive, including fixed ZIP framing.
    pub maximum_archive_bytes: usize,
    /// Maximum member count, including the root manifest.
    pub maximum_entries: usize,
    /// Maximum payload bytes of any single member.
    pub maximum_entry_bytes: u64,
    /// Maximum cumulative uncompressed member bytes.
    pub maximum_total_bytes: u64,
}

/// Failure before allocating or writing any output member bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileStoredZipWriteError {
    /// The caller-bounded output allocation could not be reserved.
    AllocationFailed,
    /// The complete output would exceed the independent archive budget.
    ArchiveTooLarge,
    /// Caller-owned output storage cannot hold the entire ZIP frame.
    BufferTooSmall {
        /// Exact required output byte count before writing any destination.
        required: usize,
    },
    /// Internal local framing unexpectedly disagreed with checked offsets.
    InconsistentFraming,
    /// A member's UTF-8 path is unsafe or cannot fit a ZIP name field.
    InvalidName,
    /// No exact root `manifest.json` was supplied.
    MissingManifest,
    /// Entries are not in strictly increasing raw UTF-8 byte-name order.
    NonCanonicalOrder,
    /// One member or cumulative member bytes exceed a caller limit.
    ResourceLimit,
    /// Ordinary ZIP32 offsets, lengths or member counts cannot represent it.
    UnsupportedZip64,
}

// Shared in-memory emission for owned and caller-bounded ZIP byte storage.
// Both sinks expose already-written headers for CRC reuse in the directory.
trait ZipOutput {
    fn append(
        &mut self,
        bytes: &[u8],
    ) -> Result<(), ProfileStoredZipWriteError>;
    fn written(&self) -> &[u8];
}

impl ZipOutput for Vec<u8> {
    fn append(
        &mut self,
        bytes: &[u8],
    ) -> Result<(), ProfileStoredZipWriteError> {
        self.extend_from_slice(bytes);
        Ok(())
    }

    fn written(&self) -> &[u8] {
        self.as_slice()
    }
}

struct SliceZipOutput<'destination> {
    destination: &'destination mut [u8],
    position: usize,
}

impl ZipOutput for SliceZipOutput<'_> {
    fn append(
        &mut self,
        bytes: &[u8],
    ) -> Result<(), ProfileStoredZipWriteError> {
        let end = self
            .position
            .checked_add(bytes.len())
            .ok_or(ProfileStoredZipWriteError::InconsistentFraming)?;
        let slot = self
            .destination
            .get_mut(self.position..end)
            .ok_or(ProfileStoredZipWriteError::InconsistentFraming)?;
        slot.copy_from_slice(bytes);
        self.position = end;
        Ok(())
    }

    fn written(&self) -> &[u8] {
        self.destination.get(..self.position).unwrap_or_default()
    }
}

fn put16<O>(
    output: &mut O,
    value: u16,
) -> Result<(), ProfileStoredZipWriteError>
where
    O: ZipOutput,
{
    output.append(&value.to_le_bytes())
}

fn put32<O>(
    output: &mut O,
    value: u32,
) -> Result<(), ProfileStoredZipWriteError>
where
    O: ZipOutput,
{
    output.append(&value.to_le_bytes())
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0u8..8u8 {
            crc = if crc & 1 == 0 {
                crc >> 1u32
            } else {
                (crc >> 1u32) ^ 0xedb8_8320
            };
        }
    }
    !crc
}

// Validate every byte count and member path before allocating output.
fn preflight_stored_profile_zip(
    members: &[ProfileStoredZipMember<'_>],
    limits: ProfileStoredZipWriteLimits,
) -> Result<(usize, usize), ProfileStoredZipWriteError> {
    if members.len() > limits.maximum_entries {
        return Err(ProfileStoredZipWriteError::ResourceLimit);
    }
    if members.len() >= usize::from(u16::MAX) {
        return Err(ProfileStoredZipWriteError::UnsupportedZip64);
    }
    let mut local_bytes = 0usize;
    let mut central_bytes = 0usize;
    let mut payload_bytes = 0u64;
    let mut previous: Option<&str> = None;
    let mut saw_root = false;
    for member in members {
        if previous.is_some_and(|name| name >= member.name) {
            return Err(ProfileStoredZipWriteError::NonCanonicalOrder);
        }
        previous = Some(member.name);
        let name_bytes = member.name.len();
        if name_bytes == 0
            || name_bytes > usize::from(u16::MAX)
            || member.name.ends_with('/')
        {
            return Err(ProfileStoredZipWriteError::InvalidName);
        }
        if member.name == PROFILE_MANIFEST_PATH {
            saw_root = true;
        } else if profile_entry_kind(member.name).is_err() {
            return Err(ProfileStoredZipWriteError::InvalidName);
        }
        let member_len = u64::try_from(member.bytes.len())
            .map_err(|_length| ProfileStoredZipWriteError::UnsupportedZip64)?;
        if member_len > limits.maximum_entry_bytes {
            return Err(ProfileStoredZipWriteError::ResourceLimit);
        }
        if member_len > u64::from(MAX_ZIP32_VALUE) {
            return Err(ProfileStoredZipWriteError::UnsupportedZip64);
        }
        payload_bytes = payload_bytes
            .checked_add(member_len)
            .ok_or(ProfileStoredZipWriteError::ResourceLimit)?;
        if payload_bytes > limits.maximum_total_bytes {
            return Err(ProfileStoredZipWriteError::ResourceLimit);
        }
        local_bytes = local_bytes
            .checked_add(LOCAL_HEADER_BYTES)
            .and_then(|value| value.checked_add(name_bytes))
            .and_then(|value| value.checked_add(member.bytes.len()))
            .ok_or(ProfileStoredZipWriteError::UnsupportedZip64)?;
        central_bytes = central_bytes
            .checked_add(CENTRAL_HEADER_BYTES)
            .and_then(|value| value.checked_add(name_bytes))
            .ok_or(ProfileStoredZipWriteError::UnsupportedZip64)?;
        if u64::try_from(local_bytes).unwrap_or(u64::MAX)
            > u64::from(MAX_ZIP32_VALUE)
            || u64::try_from(central_bytes).unwrap_or(u64::MAX)
                > u64::from(MAX_ZIP32_VALUE)
        {
            return Err(ProfileStoredZipWriteError::UnsupportedZip64);
        }
    }
    if !saw_root {
        return Err(ProfileStoredZipWriteError::MissingManifest);
    }
    let archive_bytes = local_bytes
        .checked_add(central_bytes)
        .and_then(|value| value.checked_add(END_BYTES))
        .ok_or(ProfileStoredZipWriteError::UnsupportedZip64)?;
    if archive_bytes > limits.maximum_archive_bytes {
        return Err(ProfileStoredZipWriteError::ArchiveTooLarge);
    }
    Ok((archive_bytes, central_bytes))
}

fn append_local_member<O>(
    output: &mut O,
    member: ProfileStoredZipMember<'_>,
) -> Result<(), ProfileStoredZipWriteError>
where
    O: ZipOutput,
{
    let name_len = u16::try_from(member.name.len())
        .map_err(|_len| ProfileStoredZipWriteError::InvalidName)?;
    let len = u32::try_from(member.bytes.len())
        .map_err(|_len| ProfileStoredZipWriteError::UnsupportedZip64)?;
    put32(output, 0x0403_4b50)?;
    put16(output, 20)?;
    put16(output, 0x0800)?;
    put16(output, 0)?;
    put16(output, 0)?;
    put16(output, 0)?;
    put32(output, crc32(member.bytes))?;
    put32(output, len)?;
    put32(output, len)?;
    put16(output, name_len)?;
    put16(output, 0)?;
    output.append(member.name.as_bytes())?;
    output.append(member.bytes)?;
    Ok(())
}

fn append_central_member<O>(
    output: &mut O,
    member: ProfileStoredZipMember<'_>,
    offset: u32,
    crc: u32,
) -> Result<(), ProfileStoredZipWriteError>
where
    O: ZipOutput,
{
    let name_len = u16::try_from(member.name.len())
        .map_err(|_len| ProfileStoredZipWriteError::InvalidName)?;
    let len = u32::try_from(member.bytes.len())
        .map_err(|_len| ProfileStoredZipWriteError::UnsupportedZip64)?;
    put32(output, 0x0201_4b50)?;
    put16(output, 20)?;
    put16(output, 20)?;
    put16(output, 0x0800)?;
    put16(output, 0)?;
    put16(output, 0)?;
    put16(output, 0)?;
    put32(output, crc)?;
    put32(output, len)?;
    put32(output, len)?;
    put16(output, name_len)?;
    for _ in 0u8..4u8 {
        put16(output, 0)?;
    }
    put32(output, 0)?;
    put32(output, offset)?;
    output.append(member.name.as_bytes())?;
    Ok(())
}

// The local record already contains CRC-32 of the exact member bytes.
// Read this checked offset rather than hashing every potentially large
// payload a second time when emitting the central directory.
fn stored_local_crc32(output: &[u8], local_offset: usize) -> Option<u32> {
    let start = local_offset.checked_add(LOCAL_CRC32_OFFSET)?;
    let end = start.checked_add(4)?;
    let bytes: [u8; 4] = output.get(start..end)?.try_into().ok()?;
    Some(u32::from_le_bytes(bytes))
}

/// Compute the exact ordinary stored-ZIP output byte count before allocating.
///
/// This shares all member-name, ordering, resource and ZIP32 constraints
/// with `write_stored_profile_zip`, but performs no CRC hashing or output
/// allocation. The plan refers only to the supplied byte slices at the time
/// of this call; it cannot reserve memory, bind a source path, establish an
/// accepted typed manifest, or authorize Export.
///
/// # Errors
///
/// Returns the same input preflight failure as the framing writer; the
/// distinct later `AllocationFailed` error applies only to actual writing.
pub fn planned_stored_profile_zip_bytes(
    members: &[ProfileStoredZipMember<'_>],
    limits: ProfileStoredZipWriteLimits,
) -> Result<usize, ProfileStoredZipWriteError> {
    let (whole_archive, _directory) =
        preflight_stored_profile_zip(members, limits)?;
    Ok(whole_archive)
}

/// Encode already selected bytes into deterministic ordinary stored-only ZIP.
///
/// This supplies fixed ZIP framing only; it never chooses a portable-profile
/// manifest schema, interprets member media types or grants Export authority.
/// All sizes, names, order and ZIP32 limits are checked before allocation.
///
/// # Errors
///
/// Refuses missing roots, invalid paths, unsorted inputs and limit overruns.
pub fn write_stored_profile_zip(
    members: &[ProfileStoredZipMember<'_>],
    limits: ProfileStoredZipWriteLimits,
) -> Result<Vec<u8>, ProfileStoredZipWriteError> {
    let (archive_bytes, central_bytes) =
        preflight_stored_profile_zip(members, limits)?;
    // One bounded output allocation: the central directory is appended
    // directly after local records, without a second temporary byte vector.
    let mut output = Vec::new();
    output
        .try_reserve_exact(archive_bytes)
        .map_err(|_allocation| ProfileStoredZipWriteError::AllocationFailed)?;
    encode_stored_profile_zip(
        &mut output,
        members,
        central_bytes,
        archive_bytes,
    )?;
    Ok(output)
}

/// Encode deterministic stored-only ZIP bytes into caller-owned storage.
///
/// This uses the same framing and CRC emission as the allocating writer, but
/// never allocates any output storage. All member, order, budget and ZIP32
/// checks run before considering destination size. An undersized destination
/// is rejected **before any byte is written**; a larger destination retains
/// its unused trailing bytes unchanged. On success, the returned length is
/// exactly the independently planned complete ZIP byte count.
///
/// This is pure ZIP framing for already selected and serialized byte slices,
/// not a file writer, canonical JSON serializer or authorization to Export.
///
/// # Errors
///
/// Returns identical input preflight failures as the allocating writer,
/// followed by an exact `BufferTooSmall` required-size error. Internal framing
/// failures are impossible after successful preflight but remain fail-closed.
pub fn write_stored_profile_zip_into(
    members: &[ProfileStoredZipMember<'_>],
    limits: ProfileStoredZipWriteLimits,
    destination: &mut [u8],
) -> Result<usize, ProfileStoredZipWriteError> {
    let (archive_bytes, central_bytes) =
        preflight_stored_profile_zip(members, limits)?;
    if destination.len() < archive_bytes {
        return Err(ProfileStoredZipWriteError::BufferTooSmall {
            required: archive_bytes,
        });
    }
    let mut output = SliceZipOutput {
        destination,
        position: 0,
    };
    encode_stored_profile_zip(
        &mut output,
        members,
        central_bytes,
        archive_bytes,
    )?;
    Ok(output.position)
}

// After successful common preflight, both sinks run one exact framing path.
fn encode_stored_profile_zip<O>(
    output: &mut O,
    members: &[ProfileStoredZipMember<'_>],
    central_bytes: usize,
    archive_bytes: usize,
) -> Result<(), ProfileStoredZipWriteError>
where
    O: ZipOutput,
{
    for member in members {
        append_local_member(output, *member)?;
    }
    let directory_start = u32::try_from(output.written().len())
        .map_err(|_offset| ProfileStoredZipWriteError::UnsupportedZip64)?;
    let mut local_offset = 0usize;
    for member in members {
        let offset = u32::try_from(local_offset)
            .map_err(|_offset| ProfileStoredZipWriteError::UnsupportedZip64)?;
        let crc = stored_local_crc32(output.written(), local_offset)
            .ok_or(ProfileStoredZipWriteError::InconsistentFraming)?;
        append_central_member(output, *member, offset, crc)?;
        local_offset = local_offset
            .checked_add(LOCAL_HEADER_BYTES)
            .and_then(|bytes| bytes.checked_add(member.name.len()))
            .and_then(|bytes| bytes.checked_add(member.bytes.len()))
            .ok_or(ProfileStoredZipWriteError::UnsupportedZip64)?;
    }
    debug_assert_eq!(
        usize::try_from(directory_start).ok(),
        Some(local_offset),
        "preflighted local offsets must end exactly at the central directory",
    );
    let directory_len = u32::try_from(central_bytes)
        .map_err(|_len| ProfileStoredZipWriteError::UnsupportedZip64)?;
    let count = u16::try_from(members.len())
        .map_err(|_len| ProfileStoredZipWriteError::UnsupportedZip64)?;
    put32(output, 0x0605_4b50)?;
    put16(output, 0)?;
    put16(output, 0)?;
    put16(output, count)?;
    put16(output, count)?;
    put32(output, directory_len)?;
    put32(output, directory_start)?;
    put16(output, 0)?;
    debug_assert_eq!(
        output.written().len(),
        archive_bytes,
        "emitted ZIP framing must match exact preflight byte count",
    );
    Ok(())
}
