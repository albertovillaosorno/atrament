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
//   - Transport-neutral portable handwriting-profile container invariants.
//   - Portable profile format identity and non-manifest entry validation.
// - Must-Not:
//   - Read or write files, parse ZIP or JSON, choose resource limits, or decode
//     typed handwriting sections and opaque assets.
// - Allows:
//   - Inputs: Typed manifest metadata and independently observed entry
//     evidence.
//   - Outputs: Typed validation failures and deterministic entry ordering.
//   - Side effects: Process-local result allocation only.
// - Split-When:
//   - ZIP parsing, canonical JSON, migration, or inspection gains independent
//     application or adapter authority.
// - Merge-When:
//   - Portable profile validation becomes inseparable from another pure domain.
// - Summary:
//   - Validates frozen `.atrament` container invariants without choosing wire
//     field names or file effects.
// - Description:
//   - Rejects unsafe or duplicate paths and unsupported required features
//     before
//     section decoding while preserving optional metadata as opaque values.
// - Usage:
//   - Validate parsed manifest values and entry evidence before interpretation.
// - Defaults:
//   - Non-manifest entries live only below `sections/` or `assets/`.
//

//! Portable `.atrament` handwriting-profile container invariants.

use std::collections::BTreeSet;

/// First-release portable handwriting-profile format identity.
pub const PROFILE_CONTAINER_VERSION: &str = "atrament.profile/1";

/// Canonical root manifest path in one `.atrament` archive.
pub const PROFILE_MANIFEST_PATH: &str = "manifest.json";

/// One SHA-256 digest supplied or observed at a container boundary.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Sha256Digest([u8; 32]);

impl Sha256Digest {
    /// Return the exact digest bytes.
    #[must_use]
    pub const fn bytes(self) -> [u8; 32] {
        self.0
    }

    /// Construct one exact 256-bit digest value.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

/// One non-manifest entry declared by a portable profile manifest.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProfileManifestEntry {
    /// Declared uncompressed byte length.
    pub byte_length: u64,
    /// Declared SHA-256 digest of the exact entry bytes.
    pub digest: Sha256Digest,
    /// Declared media type retained without adapter reinterpretation.
    pub media_type: String,
    /// Canonical archive-relative path below `sections/` or `assets/`.
    pub path: String,
}

/// Transport-neutral manifest values required by the accepted container ADR.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProfileManifest {
    /// Portable profile container behavior identity.
    pub container_version: String,
    /// Every non-manifest archive entry and its verification metadata.
    pub entries: Vec<ProfileManifestEntry>,
    /// Feature identifiers old readers may preserve without interpretation.
    pub optional_features: Vec<String>,
    /// Stable profile identity whose syntax belongs to the eventual wire
    /// schema.
    pub profile_identity: String,
    /// Feature identifiers that must be understood before profile admission.
    pub required_features: Vec<String>,
}

/// Why one declared profile entry path is not a canonical safe file path.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileEntryPathError {
    /// Backslash separators are not canonical portable archive separators.
    BackslashSeparator,
    /// One path segment is empty.
    EmptySegment,
    /// The root manifest cannot also appear as a declared non-manifest entry.
    ManifestReserved,
    /// A `.` or `..` traversal segment is present.
    TraversalSegment,
    /// The entry is outside the admitted `sections/` and `assets/` roots.
    UnsupportedRoot,
}

/// Typed manifest validation failure before any section is decoded.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProfileManifestError {
    /// Two declared non-manifest entries use the same canonical path.
    DuplicateEntryPath {
        /// Duplicate path retained exactly for diagnostics.
        path: String,
    },
    /// One entry has no media-type identity.
    EmptyMediaType {
        /// Entry path carrying the empty media type.
        path: String,
    },
    /// One entry path is unsafe or outside the frozen archive roots.
    InvalidEntryPath {
        /// Declared path retained exactly for diagnostics.
        path: String,
        /// Exact canonical-path rule that rejected the path.
        reason: ProfileEntryPathError,
    },
    /// The manifest uses another portable container behavior version.
    UnsupportedContainerVersion {
        /// Version supplied by the parsed manifest.
        observed: String,
    },
    /// A required feature is not admitted by this reader.
    UnsupportedRequiredFeature {
        /// Required feature retained exactly for diagnostics.
        feature: String,
    },
}

/// Typed archive-inventory failure before any entry bytes are decoded.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProfileEntryInventoryError {
    /// The archive contains the same observed path more than once.
    DuplicateObservedEntryPath {
        /// Duplicate observed archive path retained exactly for diagnostics.
        path: String,
    },
    /// The supplied manifest is invalid before archive inventory comparison.
    InvalidManifest {
        /// Exact manifest validation failure.
        reason: ProfileManifestError,
    },
    /// One observed non-manifest path is unsafe or outside admitted roots.
    InvalidObservedEntryPath {
        /// Observed archive path retained exactly for diagnostics.
        path: String,
        /// Exact canonical-path rule that rejected the observed path.
        reason: ProfileEntryPathError,
    },
    /// One manifest-declared non-manifest entry is absent from the archive.
    MissingDeclaredEntry {
        /// Missing declared path retained exactly for diagnostics.
        path: String,
    },
    /// The archive contains no root `manifest.json`.
    MissingManifest,
    /// One observed non-manifest entry has no manifest declaration.
    UndeclaredObservedEntry {
        /// Undeclared observed archive path retained exactly for diagnostics.
        path: String,
    },
}

/// Exact independently observed evidence for one archive entry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProfileEntryEvidence {
    /// Observed uncompressed entry byte length.
    pub byte_length: u64,
    /// SHA-256 digest computed over the exact observed entry bytes.
    pub digest: Sha256Digest,
}

/// Typed entry-verification failure before any content is decoded.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileEntryVerificationError {
    /// Observed entry bytes do not match the declared length.
    ByteLengthMismatch {
        /// Byte length declared by the manifest.
        declared: u64,
        /// Byte length observed by the container adapter.
        observed: u64,
    },
    /// Observed entry digest differs from the manifest declaration.
    DigestMismatch,
}

/// Validate that archive names exactly match one parsed profile manifest.
///
/// The observed path list represents archive entries before their contents are
/// decoded. It must contain exactly one root manifest and exactly the declared
/// non-manifest entries, with no duplicate, unsafe, or undeclared paths.
///
/// # Errors
///
/// Returns a typed failure for invalid manifest metadata or any mismatch
/// between manifest declarations and observed archive names.
pub fn validate_profile_entry_inventory(
    manifest: &ProfileManifest,
    supported_required_features: &[&str],
    observed_paths: &[&str],
) -> Result<(), ProfileEntryInventoryError> {
    validate_profile_manifest(manifest, supported_required_features).map_err(
        |reason| ProfileEntryInventoryError::InvalidManifest { reason },
    )?;
    let declared = manifest
        .entries
        .iter()
        .map(|entry| entry.path.as_str())
        .collect::<BTreeSet<_>>();
    let mut observed = BTreeSet::new();
    let mut observed_non_manifest = BTreeSet::new();
    let mut manifest_seen = false;
    for path in observed_paths {
        if !observed.insert(*path) {
            return Err(ProfileEntryInventoryError::DuplicateObservedEntryPath {
                path: String::from(*path),
            });
        }
        if *path == PROFILE_MANIFEST_PATH {
            manifest_seen = true;
            continue;
        }
        if let Err(reason) = validate_profile_entry_path(path) {
            return Err(ProfileEntryInventoryError::InvalidObservedEntryPath {
                path: String::from(*path),
                reason,
            });
        }
        if !declared.contains(path) {
            return Err(ProfileEntryInventoryError::UndeclaredObservedEntry {
                path: String::from(*path),
            });
        }
        let _inserted = observed_non_manifest.insert(*path);
    }
    if !manifest_seen {
        return Err(ProfileEntryInventoryError::MissingManifest);
    }
    for entry in &manifest.entries {
        if !observed_non_manifest.contains(entry.path.as_str()) {
            return Err(ProfileEntryInventoryError::MissingDeclaredEntry {
                path: entry.path.clone(),
            });
        }
    }
    Ok(())
}

/// Validate parsed portable-profile manifest values before entry decoding.
///
/// Unknown optional features are deliberately ignored by admission and remain
/// owned by the supplied manifest value. Required features must appear in the
/// caller-supplied supported set.
///
/// # Errors
///
/// Returns a typed failure for a future container version, unsupported required
/// feature, unsafe path, duplicate path, or missing media-type identity.
pub fn validate_profile_manifest(
    manifest: &ProfileManifest,
    supported_required_features: &[&str],
) -> Result<(), ProfileManifestError> {
    if manifest.container_version != PROFILE_CONTAINER_VERSION {
        return Err(ProfileManifestError::UnsupportedContainerVersion {
            observed: manifest.container_version.clone(),
        });
    }
    for feature in &manifest.required_features {
        if !supported_required_features.contains(&feature.as_str()) {
            return Err(ProfileManifestError::UnsupportedRequiredFeature {
                feature: feature.clone(),
            });
        }
    }
    let mut paths = BTreeSet::new();
    for entry in &manifest.entries {
        if let Err(reason) = validate_profile_entry_path(&entry.path) {
            return Err(ProfileManifestError::InvalidEntryPath {
                path: entry.path.clone(),
                reason,
            });
        }
        if entry.media_type.is_empty() {
            return Err(ProfileManifestError::EmptyMediaType {
                path: entry.path.clone(),
            });
        }
        if !paths.insert(entry.path.as_str()) {
            return Err(ProfileManifestError::DuplicateEntryPath {
                path: entry.path.clone(),
            });
        }
    }
    Ok(())
}

/// Return non-manifest entries in deterministic canonical archive path order.
///
/// # Errors
///
/// Returns the same manifest failures as [`validate_profile_manifest`] before
/// producing writer ordering evidence.
pub fn canonical_profile_entry_order<'manifest>(
    manifest: &'manifest ProfileManifest,
    supported_required_features: &[&str],
) -> Result<Vec<&'manifest ProfileManifestEntry>, ProfileManifestError> {
    validate_profile_manifest(manifest, supported_required_features)?;
    let mut entries = manifest.entries.iter().collect::<Vec<_>>();
    entries.sort_unstable_by(|left, right| left.path.cmp(&right.path));
    Ok(entries)
}

/// Verify one entry's observed size and SHA-256 evidence before decoding it.
///
/// # Errors
///
/// Returns length mismatch before digest mismatch to avoid interpreting digest
/// equality as evidence that a differently framed entry is admitted.
pub fn verify_profile_entry(
    entry: &ProfileManifestEntry,
    evidence: ProfileEntryEvidence,
) -> Result<(), ProfileEntryVerificationError> {
    if entry.byte_length != evidence.byte_length {
        return Err(ProfileEntryVerificationError::ByteLengthMismatch {
            declared: entry.byte_length,
            observed: evidence.byte_length,
        });
    }
    if entry.digest != evidence.digest {
        return Err(ProfileEntryVerificationError::DigestMismatch);
    }
    Ok(())
}

fn validate_profile_entry_path(
    path: &str,
) -> Result<(), ProfileEntryPathError> {
    if path == PROFILE_MANIFEST_PATH {
        return Err(ProfileEntryPathError::ManifestReserved);
    }
    if path.contains('\\') {
        return Err(ProfileEntryPathError::BackslashSeparator);
    }
    let admitted_root =
        path.starts_with("sections/") || path.starts_with("assets/");
    if !admitted_root {
        return Err(ProfileEntryPathError::UnsupportedRoot);
    }
    for segment in path.split('/') {
        if segment.is_empty() {
            return Err(ProfileEntryPathError::EmptySegment);
        }
        if matches!(segment, "." | "..") {
            return Err(ProfileEntryPathError::TraversalSegment);
        }
    }
    Ok(())
}
