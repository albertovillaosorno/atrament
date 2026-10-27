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
//   - Inputs: Classified path, parsed manifest, selected resource budget,
//     supported features, observed names, and measured encoding evidence.
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

#[derive(Clone, Copy)]
enum ProfileImportInventoryMode {
    General,
    Sorted,
}

/// Caller-selected upper bounds for an admitted portable-profile archive.
///
/// These apply to manifest declarations, not yet verified ZIP payload bytes.
/// The adapter must independently bound actual compressed/uncompressed bytes,
/// expansion, and total resources while reading and parsing each member.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProfileImportBudget {
    /// Maximum cumulative declared uncompressed member bytes.
    pub maximum_declared_bytes: u64,
    /// Maximum number of declared non-manifest archive members.
    pub maximum_entries: usize,
    /// Maximum declared uncompressed bytes in one archive member.
    pub maximum_entry_bytes: u64,
}

/// Explicit resource-budget failure before allocating an entry inventory.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileImportBudgetError {
    /// A member's declared size exceeds the configured individual limit.
    EntryBytesExceeded {
        /// Member's declared uncompressed byte length.
        declared: u64,
        /// Zero-based manifest entry index (no path strings are echoed).
        index: usize,
        /// Maximum declared bytes for any member.
        limit: u64,
    },
    /// Too many archive members are declared by the manifest.
    EntryCountExceeded {
        /// Maximum permitted number of non-manifest members.
        limit: usize,
        /// Actual count declared by the manifest.
        observed: usize,
    },
    /// The observed archive includes more member names than the budget allows.
    ObservedCountExceeded {
        /// Maximum permitted entry-name count including root manifest.
        limit: usize,
        /// Number of entry names observed by the archive adapter.
        observed: usize,
    },
    /// Aggregate member sizes exceed the caller's declared-byte budget.
    TotalBytesExceeded {
        /// Sum of member byte lengths, when representable.
        declared: u64,
        /// Maximum cumulative declared byte count.
        limit: u64,
    },
    /// Aggregate declared lengths overflow the `u64` counting authority.
    TotalOverflow,
}

/// Typed Import preflight failure before any profile section can be decoded.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProfileImportPreflightError {
    /// Canonical ZIP encoding validation rejected measured archive metadata.
    ArchiveEncoding(ProfileArchiveEncodingError),
    /// Caller-supplied resource bounds reject unsafe declared member sizes.
    Budget(ProfileImportBudgetError),
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
    budget: ProfileImportBudget,
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

    /// Exact caller-selected budget admitted for this manifest declaration.
    #[must_use]
    pub const fn budget(&self) -> ProfileImportBudget {
        self.budget
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

/// Validate manifest-declared sizes before inventory and content work.
///
/// The budget is explicitly chosen by the caller, and zero bounds are valid
/// fail-closed values. This does not inspect real file bytes or ZIP records.
///
/// # Errors
///
/// Returns count limit, then first oversized entry or running total failure.
/// A sum that cannot be represented in `u64` is rejected without wrapping.
pub fn validate_profile_import_budget(
    manifest: &ProfileManifest,
    budget: ProfileImportBudget,
) -> Result<(), ProfileImportBudgetError> {
    if manifest.entries.len() > budget.maximum_entries {
        return Err(ProfileImportBudgetError::EntryCountExceeded {
            limit: budget.maximum_entries,
            observed: manifest.entries.len(),
        });
    }
    let mut total: u64 = 0;
    for (index, entry) in manifest.entries.iter().enumerate() {
        if entry.byte_length > budget.maximum_entry_bytes {
            return Err(ProfileImportBudgetError::EntryBytesExceeded {
                declared: entry.byte_length,
                index,
                limit: budget.maximum_entry_bytes,
            });
        }
        total = total.checked_add(entry.byte_length)
            .ok_or(ProfileImportBudgetError::TotalOverflow)?;
        if total > budget.maximum_declared_bytes {
            return Err(ProfileImportBudgetError::TotalBytesExceeded {
                declared: total,
                limit: budget.maximum_declared_bytes,
            });
        }
    }
    Ok(())
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
/// Returns the first failure in path-purpose, encoding, resource budget,
/// manifest, then archive-entry-inventory order. Later phases do not execute.
pub fn preflight_explicit_profile_import<'path, 'manifest, 'observed>(
    path: &'path ExplicitFilePathIntent,
    manifest: &'manifest ProfileManifest,
    supported_required_features: &[&str],
    observed_paths: &'observed [&'observed str],
    encoding: ProfileArchiveEncodingEvidence,
    budget: ProfileImportBudget,
) -> Result<
    ProfileImportPreflight<'path, 'manifest, 'observed>,
    ProfileImportPreflightError,
> {
    preflight_profile_import_with_inventory_mode(
        path, manifest, supported_required_features, observed_paths,
        encoding, budget, ProfileImportInventoryMode::General,
    )
}

fn preflight_profile_import_with_inventory_mode<'path, 'manifest, 'observed>(
    path: &'path ExplicitFilePathIntent,
    manifest: &'manifest ProfileManifest,
    supported_required_features: &[&str],
    observed_paths: &'observed [&'observed str],
    encoding: ProfileArchiveEncodingEvidence,
    budget: ProfileImportBudget,
    mode: ProfileImportInventoryMode,
) -> Result<
    ProfileImportPreflight<'path, 'manifest, 'observed>,
    ProfileImportPreflightError,
> {
    if path.purpose() != ExplicitFilePurpose::ImportProfile {
        return Err(ProfileImportPreflightError::WrongPathPurpose);
    }
    let archive_encoding = validate_profile_archive_encoding_view(encoding)
        .map_err(ProfileImportPreflightError::ArchiveEncoding)?;
    validate_profile_import_budget(manifest, budget)
        .map_err(ProfileImportPreflightError::Budget)?;
    let observed_limit = budget.maximum_entries.saturating_add(1);
    if observed_paths.len() > observed_limit {
        return Err(ProfileImportPreflightError::Budget(
            ProfileImportBudgetError::ObservedCountExceeded {
                limit: observed_limit,
                observed: observed_paths.len(),
            },
        ));
    }
    let validated = validate_profile_manifest_view(
        manifest, supported_required_features,
    )
    .map_err(ProfileImportPreflightError::Manifest)?;
    let inventory = match mode {
        ProfileImportInventoryMode::General => {
            validated.admit_entry_inventory(observed_paths)
        },
        ProfileImportInventoryMode::Sorted => {
            validated.admit_sorted_entry_inventory(observed_paths)
        },
    }.map_err(ProfileImportPreflightError::EntryInventory)?;
    Ok(ProfileImportPreflight {
        archive_encoding,
        budget,
        inventory,
        path,
    })
}

/// Compose explicit profile Import preflight with a sorted ZIP view.
///
/// This reuses the general Import path-purpose, encoding, resource and typed
/// manifest gates in the same order. Only the final observed-name inventory
/// uses `ValidatedProfileManifest::admit_sorted_entry_inventory`: it rechecks
/// strict UTF-8 lexicographic ordering and performs a bounded, allocation-free
/// search of the validated observed-name slice rather than allocating separate
/// inventory sets. The supplied names need not be trusted as already sorted.
/// The result is still read-only evidence, not filesystem or Import authority.
///
/// # Errors
///
/// Returns the same early failures as `preflight_explicit_profile_import`,
/// with additional strict-order/count failures for the sorted inventory path.
pub fn preflight_explicit_profile_import_sorted<'path, 'manifest, 'observed>(
    path: &'path ExplicitFilePathIntent,
    manifest: &'manifest ProfileManifest,
    supported_required_features: &[&str],
    observed_paths: &'observed [&'observed str],
    encoding: ProfileArchiveEncodingEvidence,
    budget: ProfileImportBudget,
) -> Result<
    ProfileImportPreflight<'path, 'manifest, 'observed>,
    ProfileImportPreflightError,
> {
    preflight_profile_import_with_inventory_mode(
        path, manifest, supported_required_features, observed_paths,
        encoding, budget, ProfileImportInventoryMode::Sorted,
    )
}
