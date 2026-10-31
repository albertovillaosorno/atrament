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
//   - Source-bound profile Import preflight and complete member verification.
// - Must-Not:
//   - Open files, trust claimed source provenance, decode handwriting sections,
//     authorize filesystem effects, or accept a profile into session state.
// - Allows:
//   - Inputs: Source-sealed typed ZIP manifest, classified Import path,
//     supported features, resource budget and caller-owned name slots.
//   - Outputs: Sealed borrowed verified member bytes or typed failure.
//   - Side effects: Bounded validation allocations; no I/O or file mutation.
// - Split-When:
//   - Actual selected-file reading and session acceptance need a host adapter.
// - Merge-When:
//   - Complete profile Import owns these source and content checks.
// - Summary:
//   - Prevents detached manifest/ZIP combinations at the verification gate.
// - Description:
//   - All declared members must verify before any member bytes are exposed.
// - Usage:
//   - Supply only explicitly classified Import intent and a checked ZIP root.
// - Defaults:
//   - Fail closed on purpose, scratch capacity, manifest or payload mismatch.
//

//! Read-only completion of source-bound profile Import verification.

use std::fmt;

use atrament_explicit_file_path_intent::{
    ExplicitFilePathIntent, ExplicitFilePurpose,
};
use atrament_handwriting_profile_container::{ProfileManifest, Sha256Digest};
#[cfg(test)]
use atrament_profile_entry_sha256 as _;
#[cfg(test)]
use atrament_profile_json_syntax as _;
#[cfg(test)]
use atrament_profile_stored_zip_writer as _;
use atrament_profile_import_preflight::{
    ProfileImportBudget, ProfileImportPreflightError,
    preflight_explicit_profile_import_sorted,
};
use atrament_profile_manifest_json_codec::DecodedProfileZipManifest;
use atrament_profile_zip_reader::{
    ProfileZipPathCopyError, ProfileZipPayloadVerificationError,
    VerifiedProfileZipPayloads,
    verify_admitted_profile_zip_payloads,
};

/// One refusal before source-bound, complete payload evidence is exposed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProfileImportVerificationError {
    /// The caller's fixed storage cannot hold the ZIP's exact observed names.
    ObservedPathSlots(ProfileZipPathCopyError),
    /// At least one declared payload mismatches its bytes or digest.
    Payload(ProfileZipPayloadVerificationError),
    /// Invalid Import purpose, encoding, budget, manifest or inventory.
    Preflight(ProfileImportPreflightError),
}

/// Constructor-sealed evidence of every declared member in one decoded ZIP.
///
/// The supplied path remains an **intent**, not a permission to open its
/// target. This read-only evidence still lacks authenticated source selection,
/// typed section decoding, profile acceptance and any filesystem effect.
/// Debug output hides selected paths, typed identities and verified bytes.
pub struct VerifiedAttachedProfileImport<'path, 'manifest, 'archive, 'bytes> {
    path: &'path ExplicitFilePathIntent,
    payloads: VerifiedProfileZipPayloads<'manifest, 'archive, 'bytes>,
    source: &'manifest DecodedProfileZipManifest<'archive, 'bytes>,
}

impl fmt::Debug for VerifiedAttachedProfileImport<'_, '_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VerifiedAttachedProfileImport")
            .finish_non_exhaustive()
    }
}

impl<'path, 'manifest, 'bytes>
    VerifiedAttachedProfileImport<'path, 'manifest, '_, 'bytes>
{
    /// SHA-256 of the complete borrowed ZIP; not proof of file authorization.
    #[must_use]
    pub fn archive_sha256(&self) -> Sha256Digest {
        self.source.archive().source_sha256()
    }

    /// The source-decoded manifest whose entire entry set verified.
    #[must_use]
    pub const fn manifest(&self) -> &'manifest ProfileManifest {
        self.payloads.manifest()
    }

    /// The caller-classified source intent; actual file provenance is unproven.
    #[must_use]
    pub const fn path(&self) -> &'path ExplicitFilePathIntent {
        self.path
    }

    /// Source-byte fingerprint of the ZIP's exact admitted manifest root.
    #[must_use]
    pub const fn root_sha256(&self) -> Sha256Digest {
        self.source.root_sha256()
    }

    /// Borrow the complete exact source ZIP after all member integrity gates.
    ///
    /// This never normalizes or reserializes bytes. It supports read-only,
    /// byte-exact canonicality comparison against a separately produced ZIP,
    /// but does not prove the caller selected the file or authorize Import.
    /// Incidental `Debug` formatting still redacts this private source.
    #[must_use]
    pub const fn verified_archive_bytes(&self) -> &'bytes [u8] {
        self.source.archive().source_bytes()
    }

    /// Exact declared member bytes, available only after complete verification.
    #[must_use]
    pub fn verified_entry_bytes(&self, path: &str) -> Option<&'bytes [u8]> {
        self.payloads.verified_entry_bytes(path)
    }

    /// Number of non-manifest member payloads checked.
    #[must_use]
    pub const fn verified_entry_count(&self) -> usize {
        self.payloads.verified_entry_count()
    }
}

/// Bind ZIP-root-decoded metadata to its own checked archive and all digests.
///
/// Exact observed paths are copied from the attached ZIP into caller-owned
/// storage, not accepted as external claims. No member bytes are exposed
/// until path-purpose, encoding, bounded declarations, sorted inventory and
/// all member SHA-256 checks succeed. The source and manifest cannot be
/// swapped independently because the decoded input carries both in one
/// constructor-sealed value.
///
/// Storage requires one slot per ZIP entry, including `manifest.json`. This
/// method does not allocate an observed-name collection and does not mutate
/// the caller's slots on insufficient capacity. Callers must independently
/// prove the source path was actually selected and opened by a trusted host.
///
/// # Errors
///
/// Rejects incorrect path purpose before touching the scratch slots, then
/// insufficient slot capacity, preflight failure or first payload mismatch.
pub fn verify_attached_profile_import<'path, 'manifest, 'archive, 'bytes>(
    path: &'path ExplicitFilePathIntent,
    source: &'manifest DecodedProfileZipManifest<'archive, 'bytes>,
    supported_required_features: &[&str],
    budget: ProfileImportBudget,
    observed_path_slots: &mut [&'bytes str],
) -> Result<
    VerifiedAttachedProfileImport<'path, 'manifest, 'archive, 'bytes>,
    ProfileImportVerificationError,
> {
    if path.purpose() != ExplicitFilePurpose::ImportProfile {
        return Err(ProfileImportVerificationError::Preflight(
            ProfileImportPreflightError::WrongPathPurpose,
        ));
    }
    let archive = source.archive();
    let count = archive
        .copy_observed_paths(observed_path_slots)
        .map_err(ProfileImportVerificationError::ObservedPathSlots)?;
    let observed = observed_path_slots.get(..count).ok_or(
        ProfileImportVerificationError::ObservedPathSlots(
            ProfileZipPathCopyError::BufferTooSmall { required: count },
        ),
    )?;
    let preflight = preflight_explicit_profile_import_sorted(
        path,
        source.manifest(),
        supported_required_features,
        observed,
        archive.encoding_evidence(),
        budget,
    )
    .map_err(ProfileImportVerificationError::Preflight)?;
    let payloads = verify_admitted_profile_zip_payloads(
        archive, preflight.inventory(),
    ).map_err(ProfileImportVerificationError::Payload)?;
    Ok(VerifiedAttachedProfileImport {
        path,
        payloads,
        source,
    })
}
