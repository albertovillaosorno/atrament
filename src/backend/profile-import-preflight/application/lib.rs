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
//   - Read-only composition of explicitly selected profile Import paths,
//     canonical archive encoding, manifests, and observed entry inventory.
// - Must-Not:
//   - Read ZIP files, synthesize provider evidence, decode sections, admit
//     arbitrary path authority, mutate a profile, or persist imported bytes.
// - Allows:
//   - Inputs: A classified path, parsed manifest, supported feature names,
//     observed archive names, and separately measured encoding evidence.
//   - Outputs: Typed rejection or sealed entry-verification prerequisites.
//   - Side effects: None.
// - Split-When:
//   - A ZIP adapter or format decoder needs its own trust boundary.
// - Merge-When:
//   - A complete explicit profile Import application absorbs these gates.
// - Summary:
//   - Rejects mismatched file intents and unsafe archive declarations early.
// - Description:
//   - Does not mistake structural preflight for imported profile success.
// - Usage:
//   - Feed trusted observations from a separately authorized Import adapter.
// - Defaults:
//   - No file read or session mutation is granted by preflight readiness.
//

//! Pure explicit handwriting-profile Import prerequisite admission.

use atrament_explicit_file_path_intent::{
    ExplicitFilePathIntent, ExplicitFilePurpose,
};
use atrament_handwriting_profile_container::{
    ProfileArchiveEncodingError, ProfileArchiveEncodingEvidence,
    ProfileEntryEvidence, ProfileEntryInventoryError,
    ProfileEntryVerificationError, ProfileManifest, ProfileManifestError,
    ValidatedProfileArchiveEncoding, ValidatedProfileEntryInventory,
    VerifiedProfileEntry, validate_profile_archive_encoding_view,
    validate_profile_manifest_view,
};

/// Typed Import preflight failure before any profile section can be decoded.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProfileImportPreflightError {
    /// Canonical ZIP encoding validation rejected measured archive metadata.
    ArchiveEncoding(ProfileArchiveEncodingError),
    /// Archive names do not exactly match the admitted manifest declarations.
    EntryInventory(ProfileEntryInventoryError),
    /// Manifest metadata or required features are invalid for this reader.
    Manifest(ProfileManifestError),
    /// A notebook Import or any Export path cannot authorize profile Import.
    WrongPathPurpose,
}

/// Sealed, borrowed Import evidence, still awaiting bytes and section decode.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProfileImportPreflight<'path, 'manifest, 'observed> {
    archive_encoding: ValidatedProfileArchiveEncoding,
    inventory: ValidatedProfileEntryInventory<'manifest, 'observed>,
    path: &'path ExplicitFilePathIntent,
}

impl<'path, 'manifest, 'observed>
    ProfileImportPreflight<'path, 'manifest, 'observed>
{
    /// Exact admitted adapter-observed canonical ZIP encoding evidence.
    #[must_use]
    pub const fn archive_encoding(&self) -> ValidatedProfileArchiveEncoding {
        self.archive_encoding
    }

    /// Exact validated manifest and observed archive entry-name inventory.
    #[must_use]
    pub const fn inventory(
        &self,
    ) -> ValidatedProfileEntryInventory<'manifest, 'observed> {
        self.inventory
    }

    /// Exact selected file path intent, without granting permission to read.
    #[must_use]
    pub const fn path(&self) -> &'path ExplicitFilePathIntent {
        self.path
    }

    /// Verify one declared entry's independently observed size and digest.
    ///
    /// An absent entry is `None`; bytes and section semantics are not decoded.
    ///
    /// # Errors
    ///
    /// Returns a typed byte-length or digest failure, in that order.
    pub fn verify_entry(
        &self,
        path: &str,
        evidence: ProfileEntryEvidence,
    ) -> Result<
        Option<VerifiedProfileEntry<'manifest>>,
        ProfileEntryVerificationError,
    > {
        self.inventory.verify_entry(path, evidence)
    }
}

/// Compose path intent, encoding, manifest, and exact member-name admission.
///
/// This function does **not** read the file or prove that observations came
/// from the claimed path: a trusted caller must do so independently. An
/// admitted result has no filesystem effect capability, no parsed profile
/// sections, and no valid asset bytes until a later adapter verifies all entry
/// digests and decodes the typed section schema.
///
/// # Errors
///
/// Returns the first failure in path-purpose, encoding, manifest, then
/// archive-entry-inventory order. No later phase executes after rejection.
pub fn preflight_explicit_profile_import<'path, 'manifest, 'observed>(
    path: &'path ExplicitFilePathIntent,
    manifest: &'manifest ProfileManifest,
    supported_required_features: &[&str],
    observed_paths: &'observed [&'observed str],
    encoding: ProfileArchiveEncodingEvidence,
) -> Result<
    ProfileImportPreflight<'path, 'manifest, 'observed>,
    ProfileImportPreflightError,
> {
    if path.purpose() != ExplicitFilePurpose::ImportProfile {
        return Err(ProfileImportPreflightError::WrongPathPurpose);
    }
    let archive_encoding = validate_profile_archive_encoding_view(encoding)
        .map_err(ProfileImportPreflightError::ArchiveEncoding)?;
    let validated = validate_profile_manifest_view(
        manifest, supported_required_features,
    )
    .map_err(ProfileImportPreflightError::Manifest)?;
    let inventory = validated
        .admit_entry_inventory(observed_paths)
        .map_err(ProfileImportPreflightError::EntryInventory)?;
    Ok(ProfileImportPreflight {
        archive_encoding,
        inventory,
        path,
    })
}
