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
//   - Complete profile ZIP bytes tied to a typed manifest and payloads.
// - Must-Not:
//   - Select paths, write files, mutate a session, or claim Export authority.
// - Allows:
//   - Inputs: Typed profile declarations, supplied payloads and byte budgets.
//   - Outputs: Payload-verified canonical ZIP bytes or one typed refusal.
//   - Side effects: Bounded in-memory allocation and digest computation only.
// - Split-When:
//   - Trusted selected-file commits or section encoders are implemented.
// - Merge-When:
//   - The application owns complete validated profile import/export commands.
// - Summary:
//   - Ensures each payload matches exactly one manifest declaration before ZIP.
// - Description:
//   - Independent manifest JSON and stored-ZIP codecs retain byte authority.
// - Usage:
//   - Supply already selected semantic profile data, not untrusted paths.
// - Defaults:
//   - No archive bytes escape on count, path, length or digest failure.
//

//! Complete canonical portable-profile framing without file or session effects.

use atrament_handwriting_profile_container::{
    PROFILE_MANIFEST_PATH, ProfileManifest, ProfileManifestEntry, Sha256Digest,
    ProfileEntryVerificationError,
};
use atrament_profile_entry_sha256::{
    sha256_profile_bytes, verify_profile_entry_bytes,
};
use atrament_profile_json_syntax::{
    ProfileJsonSyntaxError, validate_profile_json_object,
};
pub use atrament_profile_json_syntax::ProfileJsonSyntaxLimits;
use atrament_profile_manifest_json_writer::{
    ProfileManifestJsonWriteError, ProfileManifestJsonWriteLimits,
    write_canonical_profile_manifest_json,
};
use atrament_profile_stored_zip_writer::{
    ProfileStoredZipMember, ProfileStoredZipWriteError,
    ProfileStoredZipWriteLimits, planned_stored_profile_zip_bytes,
    write_stored_profile_zip, write_stored_profile_zip_into,
};

/// Caller-supplied non-manifest payload. Paths are exact, not normalized.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProfileExportPayload<'bytes> {
    /// Exact payload bytes whose declared length and SHA-256 must match.
    pub bytes: &'bytes [u8],
    /// Exact declared ZIP entry path (never a filesystem target).
    pub path: &'bytes str,
}

/// Independent JSON and complete-ZIP resource ceilings.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProfileExportAssemblyLimits {
    /// Strict limits before allocating canonical manifest JSON.
    pub manifest_json: ProfileManifestJsonWriteLimits,
    /// Independent object syntax limits for each SHA-verified JSON section.
    pub section_json: ProfileJsonSyntaxLimits,
    /// Strict limits before allocating final stored ZIP framing.
    pub zip: ProfileStoredZipWriteLimits,
}

/// Failure before any complete profile artifact is exposed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProfileExportAssemblyError {
    /// The bounded list of borrowed metadata slots could not be reserved.
    AllocationFailed,
    /// Typed manifest could not be encoded within declared JSON limits.
    ManifestJson(ProfileManifestJsonWriteError),
    /// Supplied ZIP member count disagrees with the manifest declaration count.
    PayloadCountMismatch {
        /// Declared non-root member count.
        declared: usize,
        /// Number of supplied non-root payloads.
        supplied: usize,
    },
    /// Sorted caller payload path differs from the exact declared path.
    PayloadPathMismatch {
        /// Zero-based index in canonical manifest entry ordering.
        index: usize,
    },
    /// A declared length or SHA-256 digest disagrees with actual payload bytes.
    PayloadVerification {
        /// Zero-based index in canonical manifest entry ordering.
        index: usize,
        /// Verified failure of one exact declared member.
        reason: ProfileEntryVerificationError,
    },
    /// A fully digest-verified section is not a bounded JSON object.
    SectionJson {
        /// Zero-based index in canonical manifest entry ordering.
        index: usize,
        /// JSON grammar or caller-owned structural budget failure.
        reason: ProfileJsonSyntaxError,
    },
    /// A declared JSON section cannot be read by the first-release Import.
    SectionMediaType {
        /// Zero-based index in canonical manifest entry ordering.
        index: usize,
    },
    /// The canonical ZIP framing limits or invariants refuse output.
    Zip(ProfileStoredZipWriteError),
}

/// Evidence from caller-owned ZIP storage; not a file or session receipt.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProfileExportBufferEvidence {
    /// SHA-256 of exactly the emitted prefix, not unused destination capacity.
    pub archive_sha256: Sha256Digest,
    /// Number of bytes written into caller-owned storage.
    pub bytes_written: usize,
    /// SHA-256 of the exact canonical manifest JSON payload.
    pub root_sha256: Sha256Digest,
}

/// Owned and fully byte-verified profile ZIP, without file-write authority.
///
/// The complete archive and root fingerprints identify bytes, not identity of
/// an external source or destination. This type accepts no session state.
#[derive(Debug)]
pub struct ProfileExportArtifact {
    archive_sha256: Sha256Digest,
    bytes: Vec<u8>,
    root_sha256: Sha256Digest,
}

impl ProfileExportArtifact {
    /// SHA-256 of the complete exact canonical ZIP byte sequence.
    #[must_use]
    pub const fn archive_sha256(&self) -> Sha256Digest {
        self.archive_sha256
    }

    /// Borrow the complete ZIP bytes; this is not a filesystem permission.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// SHA-256 of the exact canonical manifest root JSON bytes.
    #[must_use]
    pub const fn root_sha256(&self) -> Sha256Digest {
        self.root_sha256
    }
}

fn sorted_entries(
    entries: &[ProfileManifestEntry],
) -> Result<Vec<&ProfileManifestEntry>, ProfileExportAssemblyError> {
    let mut sorted = Vec::new();
    sorted
        .try_reserve_exact(entries.len())
        .map_err(|_error| ProfileExportAssemblyError::AllocationFailed)?;
    sorted.extend(entries);
    sorted.sort_unstable_by(|left, right| left.path.cmp(&right.path));
    Ok(sorted)
}

fn sorted_payloads<'bytes>(
    payloads: &'bytes [ProfileExportPayload<'bytes>],
) -> Result<
    Vec<&'bytes ProfileExportPayload<'bytes>>,
    ProfileExportAssemblyError,
> {
    let mut sorted: Vec<&ProfileExportPayload<'_>> = Vec::new();
    sorted
        .try_reserve_exact(payloads.len())
        .map_err(|_error| ProfileExportAssemblyError::AllocationFailed)?;
    sorted.extend(payloads);
    sorted.sort_unstable_by(|left, right| left.path.cmp(right.path));
    Ok(sorted)
}

/// Assemble complete verified profile ZIP bytes without importing or exporting.
///
/// First checks typed manifest JSON, the exact supplied payload inventory,
/// and JSON media types for every declared `sections/` member, then
/// length-before-SHA-256 verifies **all** non-root bytes. Every verified
/// section must also pass an independently bounded JSON object syntax gate.
/// Only after all checks does the stored-ZIP writer allocate final output.
/// The independent JSON and ZIP limits include the root bytes. Caller payload
/// order and manifest declaration order never change the resulting archive.
///
/// Nothing here attests the origin of semantic profile data, authorizes a file
/// path, writes any target, or accepts a profile into the live session.
///
/// # Errors
///
/// Rejects invalid declarations, path/count mismatch, digest mismatch or
/// archive limits, without returning partial artifacts.
pub fn assemble_profile_export_artifact(
    manifest: &ProfileManifest,
    payloads: &[ProfileExportPayload<'_>],
    supported_required_features: &[&str],
    limits: ProfileExportAssemblyLimits,
) -> Result<ProfileExportArtifact, ProfileExportAssemblyError> {
    let (bytes, root_sha256) = with_verified_members(
        manifest, payloads, supported_required_features, limits,
        write_stored_profile_zip,
    )?;
    Ok(ProfileExportArtifact {
        archive_sha256: sha256_profile_bytes(&bytes),
        bytes,
        root_sha256,
    })
}

/// Preview the exact complete ZIP size after verifying all declared payloads.
///
/// Unlike the owned writer, this allocates no complete archive output and
/// performs no ZIP CRC work. It does allocate bounded JSON root bytes and
/// member references to validate the same SHA-256 and ZIP32 prerequisites.
/// The result is a byte count, not Export intent or filesystem authority.
///
/// # Errors
///
/// Refuses the same manifest, inventory, digest and ZIP preflight failures.
pub fn planned_profile_export_artifact_bytes(
    manifest: &ProfileManifest,
    payloads: &[ProfileExportPayload<'_>],
    supported_required_features: &[&str],
    limits: ProfileExportAssemblyLimits,
) -> Result<usize, ProfileExportAssemblyError> {
    let (size, _root_sha256) = with_verified_members(
        manifest, payloads, supported_required_features, limits,
        planned_stored_profile_zip_bytes,
    )?;
    Ok(size)
}

/// Assemble a complete verified profile ZIP into already-owned storage.
///
/// This does not allocate the final archive buffer. Every manifest, payload,
/// ZIP resource and destination-size check occurs before any destination write.
/// Short destinations report exact required capacity without mutation; extra
/// bytes remain untouched. Returned SHA-256 evidence covers only written bytes.
/// This is not Export file authorization, atomic commit or session acceptance.
///
/// # Errors
///
/// Refuses the same invalid inputs as owned assembly and rejects short buffers
/// atomically through the underlying ZIP writer's `BufferTooSmall` error.
pub fn write_profile_export_artifact_into(
    manifest: &ProfileManifest,
    payloads: &[ProfileExportPayload<'_>],
    supported_required_features: &[&str],
    limits: ProfileExportAssemblyLimits,
    destination: &mut [u8],
) -> Result<ProfileExportBufferEvidence, ProfileExportAssemblyError> {
    let (bytes_written, root_sha256) = with_verified_members(
        manifest, payloads, supported_required_features, limits,
        |members, zip_limits| {
            write_stored_profile_zip_into(members, zip_limits, destination)
        },
    )?;
    let written = destination.get(..bytes_written).ok_or(
        ProfileExportAssemblyError::Zip(
            ProfileStoredZipWriteError::InconsistentFraming,
        ),
    )?;
    Ok(ProfileExportBufferEvidence {
        archive_sha256: sha256_profile_bytes(written),
        bytes_written,
        root_sha256,
    })
}

// This common gate checks content and assembles exact member references before
// handing the output effect to one of the three independent ZIP writers.
fn with_verified_members<T>(
    manifest: &ProfileManifest,
    payloads: &[ProfileExportPayload<'_>],
    supported_required_features: &[&str],
    limits: ProfileExportAssemblyLimits,
    write: impl FnOnce(
        &[ProfileStoredZipMember<'_>], ProfileStoredZipWriteLimits,
    ) -> Result<T, ProfileStoredZipWriteError>,
) -> Result<(T, Sha256Digest), ProfileExportAssemblyError> {
    let root = write_canonical_profile_manifest_json(
        manifest, supported_required_features, limits.manifest_json,
    )
    .map_err(ProfileExportAssemblyError::ManifestJson)?;
    if payloads.len() != manifest.entries.len() {
        return Err(ProfileExportAssemblyError::PayloadCountMismatch {
            declared: manifest.entries.len(),
            supplied: payloads.len(),
        });
    }
    let declared = sorted_entries(&manifest.entries)?;
    let supplied = sorted_payloads(payloads)?;
    for (index, (entry, payload)) in
        declared.iter().zip(&supplied).enumerate()
    {
        if entry.path != payload.path {
            return Err(ProfileExportAssemblyError::PayloadPathMismatch {
                index,
            });
        }
    }
    // The file Import gate requires exact JSON media types for sections.
    // Avoid emitting a canonical Export artifact that it must reject.
    for (index, entry) in declared.iter().enumerate() {
        if entry.path.starts_with("sections/")
            && entry.media_type != "application/json"
        {
            return Err(ProfileExportAssemblyError::SectionMediaType { index });
        }
    }

    let count = declared.len()
        .checked_add(1)
        .ok_or(ProfileExportAssemblyError::Zip(
            ProfileStoredZipWriteError::UnsupportedZip64,
        ))?;
    let mut members = Vec::new();
    members
        .try_reserve_exact(count)
        .map_err(|_error| ProfileExportAssemblyError::AllocationFailed)?;
    let mut wrote_root = false;
    for (entry, payload) in declared.iter().zip(&supplied) {
        if !wrote_root && entry.path.as_str() > PROFILE_MANIFEST_PATH {
            members.push(ProfileStoredZipMember {
                bytes: &root,
                name: PROFILE_MANIFEST_PATH,
            });
            wrote_root = true;
        }
        members.push(ProfileStoredZipMember {
            bytes: payload.bytes,
            name: &entry.path,
        });
    }
    if !wrote_root {
        members.push(ProfileStoredZipMember {
            bytes: &root,
            name: PROFILE_MANIFEST_PATH,
        });
    }
    // Refuse ZIP count, payload budgets and ZIP32 overflow before hashing
    // potentially large member bodies. The output writer checks again.
    let _planned_size = planned_stored_profile_zip_bytes(
        &members, limits.zip,
    )
    .map_err(ProfileExportAssemblyError::Zip)?;
    for (index, (entry, payload)) in
        declared.iter().zip(&supplied).enumerate()
    {
        let _evidence = verify_profile_entry_bytes(entry, payload.bytes)
            .map_err(|reason| {
                ProfileExportAssemblyError::PayloadVerification {
                    index, reason,
                }
            })?;
    }
    // Preserve the Import gate's all-digests-before-any-section semantics.
    // No output writer is invoked unless every JSON section is admissible.
    for (index, (entry, payload)) in
        declared.iter().zip(&supplied).enumerate()
    {
        if entry.path.starts_with("sections/") {
            let _syntax = validate_profile_json_object(
                payload.bytes, limits.section_json,
            )
            .map_err(|reason| {
                ProfileExportAssemblyError::SectionJson { index, reason }
            })?;
        }
    }
    let result = write(&members, limits.zip)
        .map_err(ProfileExportAssemblyError::Zip)?;
    Ok((result, sha256_profile_bytes(&root)))
}
