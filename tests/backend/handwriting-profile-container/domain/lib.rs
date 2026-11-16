// Copyright:
//   - Copyright © 2026 Alberto Villa Osorno.
// SPDX-License-Identifier:
//   - MIT
// Confidential:
//   - false
// License-File:
//   - LICENSE
//
// Boundary-Contract:
// - Owns:
//   - Regression evidence for portable profile manifest and entry invariants.
// - Must-Not:
//   - Read ZIP files, parse JSON, invent resource limits, or decode
//     handwriting.
// - Allows:
//   - Inputs: Deterministic typed manifest and entry-evidence fixtures.
//   - Outputs: Assertions over admission, ordering, and verification failures.
//   - Side effects: Process-local test allocation only.
// - Split-When:
//   - ZIP, JSON, migration, or inspection fixtures gain independent authority.
// - Merge-When:
//   - Portable profile validation moves into another pure domain harness.
// - Summary:
//   - Pins path safety, feature admission, ordering, and entry verification.
// - Description:
//   - Keeps optional metadata opaque while required incompatibility fails
//     closed.
// - Usage:
//   - Compile directly against the handwriting-profile-container domain crate.
// - Defaults:
//   - No filesystem, archive, or renderer behavior is exercised.
//

//! Portable handwriting-profile container regression tests.

use std::ptr;

use atrament_handwriting_profile_container::{
    PROFILE_CONTAINER_VERSION, PROFILE_MANIFEST_PATH,
    ProfileArchiveEncodingError, ProfileArchiveEncodingEvidence,
    ProfileArchiveEntryEncoding, ProfileArchivePlatformExtras,
    ProfileContentChange, ProfileEntryEvidence, ProfileEntryInventoryError,
    ProfileEntryKind, ProfileEntryPathError, ProfileEntryVerificationError,
    ProfileFeatureList, ProfileManifest, ProfileManifestEntry,
    ProfileManifestError, ProfileRewriteDisposition, ProfileZip64Requirement,
    ProfileZip64Use, Sha256Digest, canonical_profile_archive_paths,
    canonical_profile_entry_order, profile_entry_kind,
    profile_rewrite_disposition, profile_section_entries,
    validate_profile_archive_encoding, validate_profile_archive_encoding_view,
    validate_profile_entry_inventory, validate_profile_manifest,
    validate_profile_manifest_view, verify_profile_entry,
};

// Fallible test assertions preserve precise failures without panic-based
// control flow in the generated admission oracles.
macro_rules! check {
    ($condition:expr $(,)?) => {
        if !$condition {
            return Err(format!(
                "line {}: expected {}",
                line!(),
                stringify!($condition),
            ));
        }
    };
    ($condition:expr, $($context:tt)+) => {
        if !$condition {
            return Err(format!(
                "line {}: {}: {}",
                line!(),
                stringify!($condition),
                format_args!($($context)+),
            ));
        }
    };
}

macro_rules! check_eq {
    ($left:expr, $right:expr $(,)?) => {
        if $left != $right {
            return Err(format!(
                "line {}: {} != {}",
                line!(),
                stringify!($left),
                stringify!($right),
            ));
        }
    };
    ($left:expr, $right:expr, $($context:tt)+) => {
        if $left != $right {
            return Err(format!(
                "line {}: {} != {}: {}",
                line!(),
                stringify!($left),
                stringify!($right),
                format_args!($($context)+),
            ));
        }
    };
}

#[derive(Default)]
struct InventoryCoverage {
    bits: u16,
}

impl InventoryCoverage {
    fn note(
        &mut self,
        expected: &Result<(), ProfileEntryInventoryError>,
    ) -> Result<(), String> {
        match &expected {
            Ok(()) => self.bits |= 1u16,
            Err(ProfileEntryInventoryError::DuplicateObservedEntryPath {
                ..
            }) => {
                self.bits |= 2u16;
            },
            Err(ProfileEntryInventoryError::InvalidManifest { .. }) => {
                return Err(String::from(
                    "fixed valid manifest cannot fail inventory admission",
                ));
            },
            Err(ProfileEntryInventoryError::InvalidObservedEntryPath {
                reason,
                ..
            }) => {
                self.bits |= 4u16;
                match reason {
                    ProfileEntryPathError::BackslashSeparator => {
                        self.bits |= 8u16;
                    },
                    ProfileEntryPathError::ControlCharacter => {
                        self.bits |= 4u16;
                    },
                    ProfileEntryPathError::EmptySegment => {
                        self.bits |= 16u16;
                    },
                    ProfileEntryPathError::TraversalSegment => {
                        self.bits |= 32u16;
                    },
                    ProfileEntryPathError::UnsupportedRoot => {
                        self.bits |= 64u16;
                    },
                    ProfileEntryPathError::ManifestReserved => {
                        return Err(String::from(
                            "root manifest is valid inventory metadata",
                        ));
                    },
                }
            },
            Err(ProfileEntryInventoryError::MissingDeclaredEntry {
                ..
            }) => {
                self.bits |= 128u16;
            },
            Err(ProfileEntryInventoryError::MissingManifest) => {
                self.bits |= 256u16;
            },
            Err(
                ProfileEntryInventoryError::NonCanonicalObservedEntryOrder {
                    ..
                }
                | ProfileEntryInventoryError::ObservedCountMismatch { .. },
            ) => {
                return Err(String::from(
                    "generic oracle cannot return sorted errors",
                ));
            },
            Err(ProfileEntryInventoryError::UndeclaredObservedEntry {
                ..
            }) => {
                self.bits |= 512u16;
            },
        }
        Ok(())
    }
}

#[derive(Default)]
struct ManifestCoverage {
    bits: u32,
}

impl ManifestCoverage {
    const fn note(&mut self, expected: &Result<(), ProfileManifestError>) {
        match &expected {
            Ok(()) => self.bits |= 1u32,
            Err(ProfileManifestError::ConflictingFeatureRequirement {
                ..
            }) => {
                self.bits |= 2u32;
            },
            Err(ProfileManifestError::DuplicateFeatureIdentifier {
                list,
                ..
            }) => match list {
                ProfileFeatureList::Required => {
                    self.bits |= 4u32;
                },
                ProfileFeatureList::Optional => {
                    self.bits |= 8u32;
                },
            },
            Err(ProfileManifestError::DuplicateEntryPath { .. }) => {
                self.bits |= 16u32;
            },
            Err(ProfileManifestError::EmptyMediaType { .. }) => {
                self.bits |= 32u32;
            },
            Err(ProfileManifestError::InvalidMediaType { .. }) => {
                self.bits |= 64u32;
            },
            Err(ProfileManifestError::InvalidEntryPath { reason, .. }) => {
                self.bits |= 128u32;
                match reason {
                    ProfileEntryPathError::BackslashSeparator => {
                        self.bits |= 256u32;
                    },
                    ProfileEntryPathError::ControlCharacter => {
                        self.bits |= 128u32;
                    },
                    ProfileEntryPathError::EmptySegment => {
                        self.bits |= 512u32;
                    },
                    ProfileEntryPathError::ManifestReserved => {
                        self.bits |= 1024u32;
                    },
                    ProfileEntryPathError::TraversalSegment => {
                        self.bits |= 2048u32;
                    },
                    ProfileEntryPathError::UnsupportedRoot => {
                        self.bits |= 4096u32;
                    },
                }
            },
            Err(ProfileManifestError::UnsupportedContainerVersion {
                ..
            }) => {
                self.bits |= 8192u32;
            },
            Err(ProfileManifestError::UnsupportedRequiredFeature {
                ..
            }) => {
                self.bits |= 16384u32;
            },
        }
    }
}

fn selected<T>(items: &[T], index: usize) -> Result<&T, String> {
    items
        .get(index)
        .ok_or_else(|| format!("missing index {index} of {}", items.len()))
}

fn selected_mut<T>(items: &mut [T], index: usize) -> Result<&mut T, String> {
    let len = items.len();
    items
        .get_mut(index)
        .ok_or_else(|| format!("missing index {index} of {len}"))
}

const fn digest(byte: u8) -> Sha256Digest {
    Sha256Digest::from_bytes([byte; 32])
}

fn entry(path: &str, media_type: &str, byte: u8) -> ProfileManifestEntry {
    ProfileManifestEntry {
        byte_length: u64::from(byte),
        digest: digest(byte),
        media_type: String::from(media_type),
        path: String::from(path),
    }
}

fn manifest(entries: Vec<ProfileManifestEntry>) -> ProfileManifest {
    ProfileManifest {
        container_version: String::from(PROFILE_CONTAINER_VERSION),
        entries,
        optional_features: vec![String::from("future-optional")],
        profile_identity: String::from("writer-fixture"),
        required_features: vec![String::from("stroke-vocabulary")],
    }
}

fn reference_archive_encoding(
    evidence: ProfileArchiveEncodingEvidence,
) -> Result<(), ProfileArchiveEncodingError> {
    if evidence.entry_encoding == ProfileArchiveEntryEncoding::Compressed {
        return Err(ProfileArchiveEncodingError::CompressedEntry);
    }
    if evidence.platform_extras == ProfileArchivePlatformExtras::Present {
        return Err(ProfileArchiveEncodingError::PlatformSpecificExtras);
    }
    match (evidence.zip64_requirement, evidence.zip64_use) {
        (ProfileZip64Requirement::Required, ProfileZip64Use::Absent) => {
            Err(ProfileArchiveEncodingError::MissingRequiredZip64)
        },
        (ProfileZip64Requirement::Ordinary, ProfileZip64Use::Present) => {
            Err(ProfileArchiveEncodingError::UnexpectedZip64)
        },
        _ => Ok(()),
    }
}

#[test]
fn every_archive_encoding_state_matches_canonical_reference()
-> Result<(), String> {
    for entry_encoding in [
        ProfileArchiveEntryEncoding::Compressed,
        ProfileArchiveEntryEncoding::Stored,
    ] {
        for platform_extras in [
            ProfileArchivePlatformExtras::Absent,
            ProfileArchivePlatformExtras::Present,
        ] {
            for zip64_requirement in [
                ProfileZip64Requirement::Ordinary,
                ProfileZip64Requirement::Required,
            ] {
                for zip64_use in
                    [ProfileZip64Use::Absent, ProfileZip64Use::Present]
                {
                    let evidence = ProfileArchiveEncodingEvidence {
                        entry_encoding,
                        platform_extras,
                        zip64_requirement,
                        zip64_use,
                    };
                    let expected = reference_archive_encoding(evidence);
                    check_eq!(
                        validate_profile_archive_encoding(evidence),
                        expected,
                    );
                    match expected {
                        Ok(()) => check_eq!(
                            validate_profile_archive_encoding_view(evidence)
                                .map_err(|reason| format!(
                                    "canonical ZIP evidence seals: {reason:?}"
                                ))?
                                .evidence(),
                            evidence,
                        ),
                        Err(reason) => check_eq!(
                            validate_profile_archive_encoding_view(evidence),
                            Err(reason),
                        ),
                    }
                }
            }
        }
    }
    Ok(())
}

#[test]
fn canonical_archive_encoding_requires_stored_entries_and_no_platform_extras() {
    let ordinary = ProfileArchiveEncodingEvidence {
        entry_encoding: ProfileArchiveEntryEncoding::Stored,
        platform_extras: ProfileArchivePlatformExtras::Absent,
        zip64_requirement: ProfileZip64Requirement::Ordinary,
        zip64_use: ProfileZip64Use::Absent,
    };
    assert_eq!(validate_profile_archive_encoding(ordinary), Ok(()));
    assert_eq!(
        validate_profile_archive_encoding(ProfileArchiveEncodingEvidence {
            entry_encoding: ProfileArchiveEntryEncoding::Compressed,
            ..ordinary
        }),
        Err(ProfileArchiveEncodingError::CompressedEntry),
    );
    assert_eq!(
        validate_profile_archive_encoding(ProfileArchiveEncodingEvidence {
            platform_extras: ProfileArchivePlatformExtras::Present,
            ..ordinary
        }),
        Err(ProfileArchiveEncodingError::PlatformSpecificExtras),
    );
}

#[test]
fn canonical_archive_encoding_uses_zip64_exactly_when_required() {
    let ordinary = ProfileArchiveEncodingEvidence {
        entry_encoding: ProfileArchiveEntryEncoding::Stored,
        platform_extras: ProfileArchivePlatformExtras::Absent,
        zip64_requirement: ProfileZip64Requirement::Ordinary,
        zip64_use: ProfileZip64Use::Absent,
    };
    assert_eq!(
        validate_profile_archive_encoding(ProfileArchiveEncodingEvidence {
            zip64_use: ProfileZip64Use::Present,
            ..ordinary
        }),
        Err(ProfileArchiveEncodingError::UnexpectedZip64),
    );
    assert_eq!(
        validate_profile_archive_encoding(ProfileArchiveEncodingEvidence {
            zip64_requirement: ProfileZip64Requirement::Required,
            ..ordinary
        }),
        Err(ProfileArchiveEncodingError::MissingRequiredZip64),
    );
    assert_eq!(
        validate_profile_archive_encoding(ProfileArchiveEncodingEvidence {
            zip64_requirement: ProfileZip64Requirement::Required,
            zip64_use: ProfileZip64Use::Present,
            ..ordinary
        }),
        Ok(()),
    );
}

#[test]
fn archive_inventory_exactly_matches_manifest_entries() {
    let value = manifest(vec![
        entry("sections/strokes.json", "application/json", 7),
        entry("assets/sample.webp", "image/webp", 9),
    ]);
    assert_eq!(
        validate_profile_entry_inventory(&value, &["stroke-vocabulary"], &[
            "assets/sample.webp",
            PROFILE_MANIFEST_PATH,
            "sections/strokes.json",
        ],),
        Ok(()),
    );
}

#[test]
fn archive_inventory_rejects_missing_and_undeclared_entries() {
    let value = manifest(vec![
        entry("sections/strokes.json", "application/json", 7),
        entry("assets/sample.webp", "image/webp", 9),
    ]);
    assert_eq!(
        validate_profile_entry_inventory(&value, &["stroke-vocabulary"], &[
            PROFILE_MANIFEST_PATH,
            "sections/strokes.json"
        ],),
        Err(ProfileEntryInventoryError::MissingDeclaredEntry {
            path: String::from("assets/sample.webp"),
        }),
    );
    assert_eq!(
        validate_profile_entry_inventory(&value, &["stroke-vocabulary"], &[
            PROFILE_MANIFEST_PATH,
            "sections/strokes.json",
            "assets/sample.webp",
            "assets/extra.bin",
        ],),
        Err(ProfileEntryInventoryError::UndeclaredObservedEntry {
            path: String::from("assets/extra.bin"),
        }),
    );
}

#[test]
fn archive_inventory_rejects_duplicate_unsafe_and_missing_manifest_paths() {
    let value =
        manifest(vec![entry("sections/strokes.json", "application/json", 7)]);
    assert_eq!(
        validate_profile_entry_inventory(&value, &["stroke-vocabulary"], &[
            PROFILE_MANIFEST_PATH,
            PROFILE_MANIFEST_PATH
        ],),
        Err(ProfileEntryInventoryError::DuplicateObservedEntryPath {
            path: String::from(PROFILE_MANIFEST_PATH),
        }),
    );
    assert_eq!(
        validate_profile_entry_inventory(&value, &["stroke-vocabulary"], &[
            PROFILE_MANIFEST_PATH,
            "sections/../strokes.json"
        ],),
        Err(ProfileEntryInventoryError::InvalidObservedEntryPath {
            path: String::from("sections/../strokes.json"),
            reason: ProfileEntryPathError::TraversalSegment,
        }),
    );
    assert_eq!(
        validate_profile_entry_inventory(&value, &["stroke-vocabulary"], &[
            "sections/strokes.json"
        ],),
        Err(ProfileEntryInventoryError::MissingManifest),
    );

    let mut future = value;
    future.container_version = String::from("atrament.profile/2");
    assert_eq!(
        validate_profile_entry_inventory(&future, &["stroke-vocabulary"], &[
            PROFILE_MANIFEST_PATH,
            "sections/strokes.json"
        ],),
        Err(ProfileEntryInventoryError::InvalidManifest {
            reason: ProfileManifestError::UnsupportedContainerVersion {
                observed: String::from("atrament.profile/2"),
            },
        }),
    );
}

fn reference_inventory_path_error(path: &str) -> Option<ProfileEntryPathError> {
    if path.contains('\\') {
        return Some(ProfileEntryPathError::BackslashSeparator);
    }
    if !path.starts_with("sections/") && !path.starts_with("assets/") {
        return Some(ProfileEntryPathError::UnsupportedRoot);
    }
    if path.chars().any(char::is_control) {
        return Some(ProfileEntryPathError::ControlCharacter);
    }
    for segment in path.split('/') {
        if segment.is_empty() {
            return Some(ProfileEntryPathError::EmptySegment);
        }
        if matches!(segment, "." | "..") {
            return Some(ProfileEntryPathError::TraversalSegment);
        }
    }
    None
}

fn reference_inventory(
    value: &ProfileManifest,
    observed: &[String],
) -> Result<(), ProfileEntryInventoryError> {
    let declared = value
        .entries
        .iter()
        .map(|entry| entry.path.as_str())
        .collect::<Vec<_>>();
    let mut seen = Vec::new();
    let mut observed_non_manifest = Vec::new();
    let mut manifest_seen = false;
    for path in observed {
        if seen.contains(&path.as_str()) {
            return Err(
                ProfileEntryInventoryError::DuplicateObservedEntryPath {
                    path: path.clone(),
                },
            );
        }
        seen.push(path.as_str());
        if path == PROFILE_MANIFEST_PATH {
            manifest_seen = true;
            continue;
        }
        if let Some(reason) = reference_inventory_path_error(path) {
            return Err(ProfileEntryInventoryError::InvalidObservedEntryPath {
                path: path.clone(),
                reason,
            });
        }
        if !declared.contains(&path.as_str()) {
            return Err(ProfileEntryInventoryError::UndeclaredObservedEntry {
                path: path.clone(),
            });
        }
        observed_non_manifest.push(path.as_str());
    }
    if !manifest_seen {
        return Err(ProfileEntryInventoryError::MissingManifest);
    }
    for entry in &value.entries {
        if !observed_non_manifest.contains(&entry.path.as_str()) {
            return Err(ProfileEntryInventoryError::MissingDeclaredEntry {
                path: entry.path.clone(),
            });
        }
    }
    Ok(())
}

const fn next_inventory_value(seed: &mut u64) -> u64 {
    *seed = seed
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    *seed
}

fn next_inventory_selector(seed: &mut u64) -> usize {
    usize::try_from(next_inventory_value(seed) >> 32).unwrap_or_default()
}

fn generated_observed_inventory(
    canonical: &[&str],
    tokens: &[&str],
    seed: &mut u64,
    operation: usize,
) -> Result<Vec<String>, String> {
    let member_index =
        next_inventory_selector(seed).rem_euclid(canonical.len());
    let insertion_index = next_inventory_selector(seed)
        .rem_euclid(canonical.len().saturating_add(1));
    let token_index = next_inventory_selector(seed).rem_euclid(tokens.len());
    let mut observed = canonical
        .iter()
        .map(|path| String::from(*path))
        .collect::<Vec<_>>();
    let token = *selected(tokens, token_index)?;
    match operation {
        0 => *selected_mut(&mut observed, member_index)? = String::from(token),
        1 => observed.insert(insertion_index, String::from(token)),
        2 => {
            let _removed = observed.remove(member_index);
        },
        _ => {
            let duplicate = selected(&observed, member_index)?.clone();
            observed.insert(insertion_index, duplicate);
        },
    }
    Ok(observed)
}

#[test]
fn generated_archive_inventory_mutations_match_reference_oracle()
-> Result<(), String> {
    const CASES: usize = 4_096;
    let value = manifest(vec![
        entry("sections/strokes.json", "application/json", 7),
        entry("assets/sample.webp", "image/webp", 9),
        entry("sections/identity.json", "application/json", 11),
    ]);
    let canonical = [
        PROFILE_MANIFEST_PATH,
        "sections/strokes.json",
        "assets/sample.webp",
        "sections/identity.json",
    ];
    let tokens = [
        PROFILE_MANIFEST_PATH,
        "sections/strokes.json",
        "assets/sample.webp",
        "sections/identity.json",
        "assets/extra.bin",
        "sections/../strokes.json",
        r"assets\sample.webp",
        "sections//identity.json",
        "other/value.bin",
        "sections/%2e%2e/value.json",
        "assets/\u{e9}.bin",
        "sections/value with space.json",
        "",
    ];
    let mut seed = 0x5eed_a2c4_2026u64;
    let mut seen_operations = [false; 4];
    let mut coverage = InventoryCoverage::default();
    let validated =
        validate_profile_manifest_view(&value, &["stroke-vocabulary"])
            .map_err(|reason| {
                format!("fixed inventory manifest is valid: {reason:?}")
            })?;
    check!(ptr::eq(validated.manifest(), &raw const value));
    for case in 0..CASES {
        let operation = next_inventory_selector(&mut seed).rem_euclid(4);
        *selected_mut(&mut seen_operations, operation)? = true;
        let observed = generated_observed_inventory(
            &canonical, &tokens, &mut seed, operation,
        )?;
        let observed_refs =
            observed.iter().map(String::as_str).collect::<Vec<_>>();
        let expected = reference_inventory(&value, &observed);
        coverage.note(&expected)?;
        check_eq!(
            validate_profile_entry_inventory(
                &value,
                &["stroke-vocabulary"],
                &observed_refs,
            ),
            expected,
            "generated archive inventory case {case}",
        );
        check_eq!(
            validated.validate_entry_inventory(&observed_refs),
            expected,
            "sealed archive inventory case {case}",
        );
        match (validated.admit_entry_inventory(&observed_refs), &expected) {
            (Ok(inventory), Ok(())) => {
                check!(ptr::eq(inventory.manifest(), &raw const value));
                check!(ptr::eq(
                    inventory.observed_paths(),
                    observed_refs.as_slice(),
                ));
            },
            (Err(actual), Err(expected_error)) => {
                check_eq!(&actual, expected_error);
            },
            (actual, expected_result) => {
                return Err(format!(
                    "sealed inventory mismatch in case {case}: \
                 {actual:?} vs {expected_result:?}",
                ));
            },
        }
    }
    check!(seen_operations.into_iter().all(|seen| seen));
    check_eq!(coverage.bits, 0x03ffu16);
    Ok(())
}

fn reference_manifest_path_error(path: &str) -> Option<ProfileEntryPathError> {
    if path == PROFILE_MANIFEST_PATH {
        return Some(ProfileEntryPathError::ManifestReserved);
    }
    reference_inventory_path_error(path)
}

fn reference_manifest(
    value: &ProfileManifest,
    supported: &[&str],
) -> Result<(), ProfileManifestError> {
    if value.container_version != PROFILE_CONTAINER_VERSION {
        return Err(ProfileManifestError::UnsupportedContainerVersion {
            observed: value.container_version.clone(),
        });
    }
    let mut required_seen = Vec::new();
    for feature in &value.required_features {
        if !supported.contains(&feature.as_str()) {
            return Err(ProfileManifestError::UnsupportedRequiredFeature {
                feature: feature.clone(),
            });
        }
        if value
            .optional_features
            .iter()
            .any(|optional| optional == feature)
        {
            return Err(ProfileManifestError::ConflictingFeatureRequirement {
                feature: feature.clone(),
            });
        }
        if required_seen.contains(&feature) {
            return Err(ProfileManifestError::DuplicateFeatureIdentifier {
                feature: feature.clone(),
                list: ProfileFeatureList::Required,
            });
        }
        required_seen.push(feature);
    }
    for (index, feature) in value.optional_features.iter().enumerate() {
        if value
            .optional_features
            .iter()
            .take(index)
            .any(|prior| prior == feature)
        {
            return Err(ProfileManifestError::DuplicateFeatureIdentifier {
                feature: feature.clone(),
                list: ProfileFeatureList::Optional,
            });
        }
    }
    let mut paths = Vec::new();
    for item in &value.entries {
        if let Some(reason) = reference_manifest_path_error(&item.path) {
            return Err(ProfileManifestError::InvalidEntryPath {
                path: item.path.clone(),
                reason,
            });
        }
        if item.media_type.trim().is_empty() {
            return Err(ProfileManifestError::EmptyMediaType {
                path: item.path.clone(),
            });
        }
        if item.media_type.chars().any(char::is_control) {
            return Err(ProfileManifestError::InvalidMediaType {
                path: item.path.clone(),
            });
        }
        if paths.contains(&item.path.as_str()) {
            return Err(ProfileManifestError::DuplicateEntryPath {
                path: item.path.clone(),
            });
        }
        paths.push(item.path.as_str());
    }
    Ok(())
}

fn generated_manifest_entries(
    seed: &mut u64,
    count: usize,
    paths: &[&str],
    media_types: &[&str],
    seen_paths: &mut [bool; 11],
    seen_media_types: &mut [bool; 7],
) -> Result<Vec<ProfileManifestEntry>, String> {
    let mut entries: Vec<ProfileManifestEntry> = Vec::with_capacity(count);
    for entry_index in 0..count {
        let path_index = next_inventory_selector(seed).rem_euclid(paths.len());
        *selected_mut(seen_paths, path_index)? = true;
        let media_index =
            next_inventory_selector(seed).rem_euclid(media_types.len());
        *selected_mut(seen_media_types, media_index)? = true;
        let selected_path = if entry_index > 0
            && next_inventory_selector(seed).is_multiple_of(5)
        {
            entries
                .first()
                .ok_or_else(|| String::from("missing generated source"))?
                .path
                .as_str()
        } else {
            *selected(paths, path_index)?
        };
        entries.push(entry(
            selected_path,
            selected(media_types, media_index)?,
            u8::try_from(entry_index.saturating_add(1)).map_err(|reason| {
                format!("generated entry count stays small: {reason:?}")
            })?,
        ));
    }
    Ok(entries)
}

fn generated_manifest_case(
    seed: &mut u64,
    case: usize,
    seen_versions: &mut [bool; 4],
    seen_feature_sets: &mut [bool; 5],
    seen_entry_counts: &mut [bool; 5],
    seen_paths: &mut [bool; 11],
    seen_media_types: &mut [bool; 7],
) -> Result<ProfileManifest, String> {
    let versions = [
        PROFILE_CONTAINER_VERSION,
        "atrament.profile/0",
        "atrament.profile/2",
        "",
    ];
    let feature_sets: &[&[&str]] = &[
        &["stroke-vocabulary"],
        &["future-supported"],
        &["stroke-vocabulary", "future-supported"],
        &["future-required"],
        &["stroke-vocabulary", "future-required"],
    ];
    let paths = [
        "sections/strokes.json",
        "assets/sample.webp",
        "sections/identity.json",
        "assets/\u{e9}.bin",
        "sections/%2e%2e/value.json",
        PROFILE_MANIFEST_PATH,
        "sections/../value.json",
        r"assets\sample.webp",
        "sections//value.json",
        "other/value.bin",
        "",
    ];
    let media_types = [
        "application/json",
        "image/webp",
        "",
        "opaque/type",
        " ",
        "text/plain\r\ninjected",
        "image/\0webp",
    ];
    let version_index =
        next_inventory_selector(seed).rem_euclid(versions.len());
    *selected_mut(seen_versions, version_index)? = true;
    let feature_index =
        next_inventory_selector(seed).rem_euclid(feature_sets.len());
    *selected_mut(seen_feature_sets, feature_index)? = true;
    let entry_count = next_inventory_selector(seed).rem_euclid(5);
    *selected_mut(seen_entry_counts, entry_count)? = true;
    let entries = generated_manifest_entries(
        seed,
        entry_count,
        &paths,
        &media_types,
        seen_paths,
        seen_media_types,
    )?;
    let mut optional_features = if case.is_multiple_of(7) {
        vec![String::from("stroke-vocabulary")]
    } else {
        vec![String::from("future-optional")]
    };
    if case.is_multiple_of(11) {
        optional_features.push(
            optional_features
                .first()
                .cloned()
                .ok_or_else(|| String::from("no optional feature"))?,
        );
    }
    let mut required_features = selected(feature_sets, feature_index)?
        .iter()
        .map(|feature| String::from(*feature))
        .collect::<Vec<_>>();
    if case.is_multiple_of(13) {
        required_features.push(
            required_features
                .first()
                .cloned()
                .ok_or_else(|| String::from("no required feature"))?,
        );
    }
    let value = ProfileManifest {
        container_version: String::from(*selected(&versions, version_index)?),
        entries,
        optional_features,
        profile_identity: String::from("writer-generated"),
        required_features,
    };
    Ok(value)
}

#[test]
fn generated_manifest_values_match_reference_admission_oracle()
-> Result<(), String> {
    const CASES: usize = 4_096;
    let supported = ["stroke-vocabulary", "future-supported"];
    let mut seed = 0x5eed_4d41_2026u64;
    let mut seen_versions = [false; 4];
    let mut seen_feature_sets = [false; 5];
    let mut seen_entry_counts = [false; 5];
    let mut seen_paths = [false; 11];
    let mut seen_media_types = [false; 7];
    let mut coverage = ManifestCoverage::default();
    for case in 0..CASES {
        let value = generated_manifest_case(
            &mut seed,
            case,
            &mut seen_versions,
            &mut seen_feature_sets,
            &mut seen_entry_counts,
            &mut seen_paths,
            &mut seen_media_types,
        )?;
        let expected = reference_manifest(&value, &supported);
        coverage.note(&expected);
        check_eq!(
            validate_profile_manifest(&value, &supported),
            expected,
            "generated manifest case {case}",
        );
        match (
            validate_profile_manifest_view(&value, &supported),
            &expected,
        ) {
            (Ok(validated), Ok(())) => {
                check!(ptr::eq(validated.manifest(), &raw const value));
                for entry in &value.entries {
                    let resolved =
                        validated.entry(&entry.path).ok_or_else(|| {
                            String::from("valid declaration resolves exactly")
                        })?;
                    check!(ptr::eq(resolved, entry));
                }
            },
            (Err(actual), Err(expected_error)) => {
                check_eq!(&actual, expected_error);
            },
            (actual, expected_result) => {
                return Err(format!(
                    "sealed manifest mismatch in case {case}: \
                 {actual:?} vs {expected_result:?}",
                ));
            },
        }
    }
    check!(seen_versions.into_iter().all(|seen| seen));
    check!(seen_feature_sets.into_iter().all(|seen| seen));
    check!(seen_entry_counts.into_iter().all(|seen| seen));
    check!(seen_paths.into_iter().all(|seen| seen));
    check!(seen_media_types.into_iter().all(|seen| seen));
    check_eq!(coverage.bits, 0x7fffu32);
    Ok(())
}

#[test]
fn manifest_admits_sections_assets_and_preserves_optional_features() {
    let value = manifest(vec![
        entry("sections/strokes.json", "application/json", 7),
        entry("assets/sample.webp", "image/webp", 9),
    ]);
    assert_eq!(
        validate_profile_manifest(&value, &["stroke-vocabulary"]),
        Ok(()),
    );
    assert_eq!(value.optional_features, ["future-optional"]);
}

#[test]
fn required_and_optional_feature_overlap_refuses_before_any_entry_decoding() {
    let mut value =
        manifest(vec![entry("sections/a.json", "application/json", 1)]);
    for feature in ["stroke-vocabulary", "future-optional", "ma\u{f1}ana"] {
        value.required_features = vec![String::from(feature)];
        value.optional_features = vec!["unknown-future".into(), feature.into()];
        let admitted = validate_profile_manifest(&value, &[feature]);
        assert_eq!(
            admitted,
            Err(ProfileManifestError::ConflictingFeatureRequirement {
                feature: feature.into(),
            }),
        );
        assert_eq!(
            validate_profile_manifest_view(&value, &[feature]).err(),
            admitted.err(),
        );
    }
    // A feature is optional only when absent from the required list. Exact
    // spelling matters; unknown optional identifiers remain uninterpreted.
    value.required_features = vec!["stroke-vocabulary".into()];
    value.optional_features = vec!["Stroke-vocabulary".into(), "future".into()];
    assert_eq!(
        validate_profile_manifest(&value, &["stroke-vocabulary"]),
        Ok(()),
    );
    value.optional_features.push("stroke-vocabulary".into());
    assert_eq!(
        validate_profile_manifest(&value, &[]),
        Err(ProfileManifestError::UnsupportedRequiredFeature {
            feature: "stroke-vocabulary".into(),
        }),
        "unsupported mandatory features must still fail first",
    );
}

#[test]
fn repeated_feature_identifiers_refuse_with_exact_list_and_first_failure() {
    let mut value = manifest(vec![]);
    value.required_features = vec!["strokes".into(), "strokes".into()];
    value.optional_features = vec!["unknown".into()];
    assert_eq!(
        validate_profile_manifest(&value, &["strokes"]),
        Err(ProfileManifestError::DuplicateFeatureIdentifier {
            feature: "strokes".into(),
            list: ProfileFeatureList::Required,
        }),
    );
    value.required_features = vec!["strokes".into()];
    value.optional_features =
        vec!["unknown".into(), "future".into(), "unknown".into()];
    assert_eq!(
        validate_profile_manifest(&value, &["strokes"]),
        Err(ProfileManifestError::DuplicateFeatureIdentifier {
            feature: "unknown".into(),
            list: ProfileFeatureList::Optional,
        }),
    );
    // First required-feature failure still wins over optional-list duplicates.
    assert_eq!(
        validate_profile_manifest(&value, &[]),
        Err(ProfileManifestError::UnsupportedRequiredFeature {
            feature: "strokes".into(),
        }),
    );
    value.optional_features.push("strokes".into());
    assert_eq!(
        validate_profile_manifest(&value, &["strokes"]),
        Err(ProfileManifestError::ConflictingFeatureRequirement {
            feature: "strokes".into(),
        }),
    );
    // Feature spelling is byte-exact; case and Unicode forms remain distinct.
    value.optional_features = vec![
        "future".into(),
        "Future".into(),
        "\u{e9}".into(),
        "e\u{301}".into(),
    ];
    assert_eq!(validate_profile_manifest(&value, &["strokes"]), Ok(()));
}

#[test]
fn large_feature_lists_use_exact_membership_and_first_failure_order() {
    let mut value = manifest(vec![]);
    value.required_features = (0u32..4_096)
        .map(|index| format!("required-{index}"))
        .collect();
    value.optional_features = (0u32..4_096)
        .map(|index| format!("optional-{index}"))
        .collect();
    let required_copy = value.required_features.clone();
    let supported =
        required_copy.iter().map(String::as_str).collect::<Vec<_>>();
    assert_eq!(validate_profile_manifest(&value, &supported), Ok(()));
    value.optional_features.push("optional-4095".into());
    assert_eq!(
        validate_profile_manifest(&value, &supported),
        Err(ProfileManifestError::DuplicateFeatureIdentifier {
            feature: "optional-4095".into(),
            list: ProfileFeatureList::Optional,
        }),
    );
    let _removed = value.optional_features.pop();
    value.required_features.push("required-4095".into());
    assert_eq!(
        validate_profile_manifest(&value, &supported),
        Err(ProfileManifestError::DuplicateFeatureIdentifier {
            feature: "required-4095".into(),
            list: ProfileFeatureList::Required,
        }),
    );
    let _dropped_required = value.required_features.pop();
    value.optional_features.push("required-4095".into());
    assert_eq!(
        validate_profile_manifest(&value, &supported),
        Err(ProfileManifestError::ConflictingFeatureRequirement {
            feature: "required-4095".into(),
        }),
    );
    assert_eq!(
        validate_profile_manifest(
            &value,
            supported.get(..4_095).unwrap_or_default()
        ),
        Err(ProfileManifestError::UnsupportedRequiredFeature {
            feature: "required-4095".into(),
        }),
    );
}

#[test]
fn canonical_archive_order_includes_root_manifest_by_path() {
    let value = manifest(vec![
        entry("sections/z.json", "application/json", 3),
        entry("assets/b.bin", "application/octet-stream", 2),
        entry("sections/a.json", "application/json", 1),
    ]);
    assert_eq!(
        canonical_profile_archive_paths(&value, &["stroke-vocabulary"]),
        Ok(vec![
            "assets/b.bin",
            PROFILE_MANIFEST_PATH,
            "sections/a.json",
            "sections/z.json",
        ]),
    );
}

fn next_index_permutation(values: &mut [usize]) -> bool {
    let Some(pivot) = values
        .windows(2)
        .rposition(|pair| pair.first() < pair.get(1))
    else {
        return false;
    };
    let Some(&pivot_value) = values.get(pivot) else {
        return false;
    };
    let Some(successor) = values
        .iter()
        .enumerate()
        .rev()
        .find(|(index, value)| *index > pivot && **value > pivot_value)
        .map(|(index, _)| index)
    else {
        return false;
    };
    values.swap(pivot, successor);
    let Some(tail) = values.get_mut(pivot.saturating_add(1)..) else {
        return false;
    };
    tail.reverse();
    true
}
fn verify_canonical_projection_order(
    value: &ProfileManifest,
    expected_archive_paths: &[&str],
    expected_entry_paths: &[&str],
    expected_section_paths: &[&str],
    order: &[usize],
) -> Result<(), String> {
    let original_order = value
        .entries
        .iter()
        .map(|item| item.path.clone())
        .collect::<Vec<_>>();
    let validated =
        validate_profile_manifest_view(value, &["stroke-vocabulary"]).map_err(
            |reason| {
                format!("valid declarations seal one manifest: {reason:?}")
            },
        )?;
    check!(ptr::eq(validated.manifest(), value));
    for &expected_path in expected_entry_paths {
        let resolved = validated.entry(expected_path).ok_or_else(|| {
            String::from("canonical declared path resolves exactly")
        })?;
        check_eq!(resolved.path, expected_path);
        check!(value.entries.iter().any(|entry| {
            ptr::eq(entry, resolved) && entry.path == expected_path
        }));
    }
    check_eq!(validated.entry(PROFILE_MANIFEST_PATH), None);
    check_eq!(validated.entry("sections/missing.json"), None);
    check_eq!(validated.entry("sections/../a.json"), None);
    check_eq!(
        validated.canonical_archive_paths(),
        expected_archive_paths,
        "sealed archive order for permutation {order:?}",
    );
    check_eq!(
        validated
            .canonical_entry_order()
            .iter()
            .map(|item| item.path.as_str())
            .collect::<Vec<_>>(),
        expected_entry_paths,
        "sealed entry order for permutation {order:?}",
    );
    check_eq!(
        validated
            .section_entries()
            .iter()
            .map(|item| item.path.as_str())
            .collect::<Vec<_>>(),
        expected_section_paths,
        "sealed section order for permutation {order:?}",
    );

    check_eq!(
        canonical_profile_archive_paths(value, &["stroke-vocabulary"]),
        Ok(expected_archive_paths.to_vec()),
        "archive order for permutation {order:?}",
    );
    let entry_order =
        canonical_profile_entry_order(value, &["stroke-vocabulary"]).map_err(
            |reason| {
                format!(
                    "valid declarations have canonical entry order: {reason:?}"
                )
            },
        )?;
    check_eq!(
        entry_order
            .iter()
            .map(|item| item.path.as_str())
            .collect::<Vec<_>>(),
        expected_entry_paths,
        "entry order for permutation {order:?}",
    );
    let sections = profile_section_entries(value, &["stroke-vocabulary"])
        .map_err(|reason| {
            format!("valid declarations have section projection: {reason:?}")
        })?;
    check_eq!(
        sections
            .iter()
            .map(|item| item.path.as_str())
            .collect::<Vec<_>>(),
        expected_section_paths,
        "section order for permutation {order:?}",
    );
    check_eq!(
        value
            .entries
            .iter()
            .map(|item| item.path.as_str())
            .collect::<Vec<_>>(),
        original_order
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        "projection mutated declaration order for {order:?}",
    );
    Ok(())
}

#[test]
fn every_declaration_order_has_identical_canonical_profile_projections()
-> Result<(), String> {
    let declarations = [
        ("sections/z.json", "application/json", 1u8),
        ("assets/b.bin", "application/octet-stream", 2u8),
        ("sections/a.json", "application/json", 3u8),
        ("assets/y.bin", "application/octet-stream", 4u8),
        ("sections/m.json", "application/json", 5u8),
        ("assets/c.bin", "application/octet-stream", 6u8),
    ];
    let expected_archive_paths = [
        "assets/b.bin",
        "assets/c.bin",
        "assets/y.bin",
        PROFILE_MANIFEST_PATH,
        "sections/a.json",
        "sections/m.json",
        "sections/z.json",
    ];
    let expected_entry_paths = [
        "assets/b.bin",
        "assets/c.bin",
        "assets/y.bin",
        "sections/a.json",
        "sections/m.json",
        "sections/z.json",
    ];
    let expected_section_paths =
        ["sections/a.json", "sections/m.json", "sections/z.json"];
    let mut order = [0usize, 1, 2, 3, 4, 5];
    let mut cases = 0usize;
    loop {
        let entries = order
            .iter()
            .map(|&index| {
                let &(path, media_type, byte) = selected(&declarations, index)
                    .map_err(|reason| format!("permutation: {reason}"))?;
                Ok(entry(path, media_type, byte))
            })
            .collect::<Result<Vec<_>, String>>()?;
        let value = manifest(entries);
        verify_canonical_projection_order(
            &value,
            &expected_archive_paths,
            &expected_entry_paths,
            &expected_section_paths,
            &order,
        )?;
        cases += 1;
        if !next_index_permutation(&mut order) {
            break;
        }
    }
    check_eq!(cases, 720);
    Ok(())
}

#[test]
fn canonical_order_is_path_sorted_without_rewriting_manifest()
-> Result<(), String> {
    let value = manifest(vec![
        entry("sections/z.json", "application/json", 3),
        entry("assets/b.bin", "application/octet-stream", 2),
        entry("sections/a.json", "application/json", 1),
    ]);
    let ordered = canonical_profile_entry_order(&value, &["stroke-vocabulary"])
        .map_err(|reason| {
            format!("valid manifest has canonical entry ordering: {reason:?}")
        })?;
    check_eq!(
        ordered
            .iter()
            .map(|entry| entry.path.as_str())
            .collect::<Vec<_>>(),
        ["assets/b.bin", "sections/a.json", "sections/z.json"],
    );
    check_eq!(
        value.entries.first().map(|item| item.path.as_str()),
        Some("sections/z.json")
    );
    Ok(())
}

#[test]
fn control_characters_in_entry_paths_fail_closed_before_decoding() {
    for root in ["sections", "assets"] {
        for character in ['\0', '\r', '\n', '\t', '\u{0085}'] {
            let path = format!("{root}/prefix{character}suffix.json");
            assert_eq!(
                profile_entry_kind(&path),
                Err(ProfileEntryPathError::ControlCharacter),
            );
            let value = manifest(vec![entry(&path, "application/json", 1)]);
            assert_eq!(
                validate_profile_manifest(&value, &["stroke-vocabulary"]),
                Err(ProfileManifestError::InvalidEntryPath {
                    path: path.clone(),
                    reason: ProfileEntryPathError::ControlCharacter,
                }),
            );
            let empty_manifest = manifest(vec![]);
            assert_eq!(
                validate_profile_entry_inventory(
                    &empty_manifest,
                    &["stroke-vocabulary"],
                    &[PROFILE_MANIFEST_PATH, &path],
                ),
                Err(ProfileEntryInventoryError::InvalidObservedEntryPath {
                    path,
                    reason: ProfileEntryPathError::ControlCharacter,
                }),
            );
        }
    }
}

#[test]
fn unsafe_duplicate_and_manifest_paths_reject_before_decoding() {
    for (path, reason) in [
        ("manifest.json", ProfileEntryPathError::ManifestReserved),
        ("../sample.bin", ProfileEntryPathError::UnsupportedRoot),
        (
            "sections/../sample.json",
            ProfileEntryPathError::TraversalSegment,
        ),
        ("sections//sample.json", ProfileEntryPathError::EmptySegment),
        (
            r"sections\sample.json",
            ProfileEntryPathError::BackslashSeparator,
        ),
        ("other/sample.json", ProfileEntryPathError::UnsupportedRoot),
    ] {
        let value = manifest(vec![entry(path, "application/json", 1)]);
        assert_eq!(
            validate_profile_manifest(&value, &["stroke-vocabulary"]),
            Err(ProfileManifestError::InvalidEntryPath {
                path: String::from(path),
                reason,
            }),
            "{path}",
        );
    }

    let value = manifest(vec![
        entry("sections/a.json", "application/json", 1),
        entry("sections/a.json", "application/json", 2),
    ]);
    assert_eq!(
        validate_profile_manifest(&value, &["stroke-vocabulary"]),
        Err(ProfileManifestError::DuplicateEntryPath {
            path: String::from("sections/a.json"),
        }),
    );
}

#[test]
fn empty_media_type_rejects_before_entry_decoding() {
    let value = manifest(vec![entry("sections/a.json", "", 1)]);
    assert_eq!(
        validate_profile_manifest(&value, &["stroke-vocabulary"]),
        Err(ProfileManifestError::EmptyMediaType {
            path: String::from("sections/a.json"),
        }),
    );
}

#[test]
fn profile_media_type_controls_and_empty_whitespace_reject_before_decoding() {
    let entry_path = "sections/media.json";
    for media_type in [" ", "   ", "\t", "\n"] {
        let value = manifest(vec![entry(entry_path, media_type, 1)]);
        assert_eq!(
            validate_profile_manifest(&value, &["stroke-vocabulary"]),
            Err(ProfileManifestError::EmptyMediaType {
                path: String::from(entry_path),
            }),
            "media_type={media_type:?}",
        );
    }
    for media_type in
        ["image/\0webp", "text/plain\r\ninjected", "image/\u{0085}x"]
    {
        let value = manifest(vec![entry(entry_path, media_type, 1)]);
        assert_eq!(
            validate_profile_manifest(&value, &["stroke-vocabulary"]),
            Err(ProfileManifestError::InvalidMediaType {
                path: String::from(entry_path),
            }),
            "media_type={media_type:?}",
        );
    }
    let value =
        manifest(vec![entry(entry_path, "text/plain; charset=utf-8", 1)]);
    assert_eq!(
        validate_profile_manifest(&value, &["stroke-vocabulary"]),
        Ok(()),
    );
}

#[test]
fn entry_kind_is_path_owned_and_rejects_noncanonical_names() {
    assert_eq!(
        profile_entry_kind("assets/source/sample.webp"),
        Ok(ProfileEntryKind::Asset),
    );
    assert_eq!(
        profile_entry_kind("sections/identity.json"),
        Ok(ProfileEntryKind::Section),
    );
    assert_eq!(
        profile_entry_kind("sections/../identity.json"),
        Err(ProfileEntryPathError::TraversalSegment),
    );
    assert_eq!(
        profile_entry_kind(PROFILE_MANIFEST_PATH),
        Err(ProfileEntryPathError::ManifestReserved),
    );
}

#[test]
fn profile_path_corpus_pins_safe_path_admission() -> Result<(), String> {
    let segments = [
        "",
        ".",
        "..",
        "%2e%2e",
        "a",
        "caf\u{e9}",
        "sample.json",
        "space name",
        "x\0y",
        "x\ny",
        r"x\y",
    ];
    let mut exercised = 0usize;
    let mut saw_asset = false;
    let mut saw_section = false;
    let mut saw_backslash = false;
    let mut saw_control = false;
    let mut saw_empty_segment = false;
    let mut saw_traversal = false;
    for (root, expected_kind) in [
        ("assets", ProfileEntryKind::Asset),
        ("sections", ProfileEntryKind::Section),
    ] {
        for left in segments {
            for right in segments {
                let path = format!("{root}/{left}/{right}");
                let expected = if left.contains('\\') || right.contains('\\') {
                    Err(ProfileEntryPathError::BackslashSeparator)
                } else if left.chars().any(char::is_control)
                    || right.chars().any(char::is_control)
                {
                    Err(ProfileEntryPathError::ControlCharacter)
                } else {
                    let first_segment_error =
                        [left, right].into_iter().find_map(|segment| {
                            if segment.is_empty() {
                                Some(ProfileEntryPathError::EmptySegment)
                            } else if matches!(segment, "." | "..") {
                                Some(ProfileEntryPathError::TraversalSegment)
                            } else {
                                None
                            }
                        });
                    first_segment_error.map_or(Ok(expected_kind), Err)
                };
                match expected {
                    Ok(ProfileEntryKind::Asset) => saw_asset = true,
                    Ok(ProfileEntryKind::Section) => saw_section = true,
                    Err(ProfileEntryPathError::BackslashSeparator) => {
                        saw_backslash = true;
                    },
                    Err(ProfileEntryPathError::ControlCharacter) => {
                        saw_control = true;
                    },
                    Err(ProfileEntryPathError::EmptySegment) => {
                        saw_empty_segment = true;
                    },
                    Err(ProfileEntryPathError::TraversalSegment) => {
                        saw_traversal = true;
                    },
                    Err(
                        ProfileEntryPathError::ManifestReserved
                        | ProfileEntryPathError::UnsupportedRoot,
                    ) => {
                        return Err(String::from(
                            "root-scoped Cartesian path has unexpected class",
                        ));
                    },
                }
                check_eq!(profile_entry_kind(&path), expected, "{path}");
                exercised += 1;
            }
        }
    }
    check_eq!(exercised, 242);
    check!(saw_control);
    check!(saw_asset);
    check!(saw_section);
    check!(saw_backslash);
    check!(saw_empty_segment);
    check!(saw_traversal);

    for path in ["assets", "other/sample.json", "sections", "sample.json"] {
        check_eq!(
            profile_entry_kind(path),
            Err(ProfileEntryPathError::UnsupportedRoot),
            "{path}",
        );
    }
    check_eq!(
        profile_entry_kind(r"other\sample.json"),
        Err(ProfileEntryPathError::BackslashSeparator),
    );
    Ok(())
}

#[test]
fn profile_rewrite_requires_canonical_bytes_only_after_change() {
    assert_eq!(
        profile_rewrite_disposition(ProfileContentChange::Changed),
        ProfileRewriteDisposition::CanonicalRewriteRequired,
    );
    assert_eq!(
        profile_rewrite_disposition(ProfileContentChange::Unchanged),
        ProfileRewriteDisposition::OriginalBytesMayBePreserved,
    );
}

#[test]
fn section_projection_is_sorted_and_never_returns_opaque_assets()
-> Result<(), String> {
    let value = manifest(vec![
        entry("sections/z.json", "application/json", 3),
        entry("assets/source.bin", "application/octet-stream", 2),
        entry("sections/a.json", "application/json", 1),
    ]);
    let sections = profile_section_entries(&value, &["stroke-vocabulary"])
        .map_err(|reason| {
            format!("valid manifest exposes typed section metadata: {reason:?}")
        })?;
    check_eq!(
        sections
            .iter()
            .map(|entry| entry.path.as_str())
            .collect::<Vec<_>>(),
        ["sections/a.json", "sections/z.json"],
    );
    check_eq!(value.entries.len(), 3);
    Ok(())
}

#[test]
fn profile_compatibility_matrix_is_explicit_and_nondestructive() {
    let current = manifest(vec![entry(
        "assets/sample.bin",
        "application/octet-stream",
        7,
    )]);
    let current_before = current.clone();
    assert_eq!(
        validate_profile_manifest(&current, &["stroke-vocabulary"]),
        Ok(()),
    );
    assert_eq!(current, current_before);
    assert_eq!(current.optional_features, ["future-optional"]);

    for version in ["atrament.profile/0", "atrament.profile/2"] {
        let mut unsupported = current.clone();
        unsupported.container_version = String::from(version);
        let before = unsupported.clone();
        assert_eq!(
            validate_profile_manifest(&unsupported, &["stroke-vocabulary"]),
            Err(ProfileManifestError::UnsupportedContainerVersion {
                observed: String::from(version),
            }),
        );
        assert_eq!(unsupported, before);
    }

    let mut unsupported_required = current.clone();
    unsupported_required
        .required_features
        .push(String::from("future-required"));
    let required_before = unsupported_required.clone();
    assert_eq!(
        validate_profile_manifest(&unsupported_required, &[
            "stroke-vocabulary"
        ],),
        Err(ProfileManifestError::UnsupportedRequiredFeature {
            feature: String::from("future-required"),
        }),
    );
    assert_eq!(unsupported_required, required_before);

    let Some(declared) = current.entries.first().cloned() else {
        assert_eq!(current.entries.len(), 1, "missing declared entry");
        return;
    };
    let declared_before = declared.clone();
    assert_eq!(
        verify_profile_entry(&declared, ProfileEntryEvidence {
            byte_length: 6,
            digest: digest(7),
        },),
        Err(ProfileEntryVerificationError::ByteLengthMismatch {
            declared: 7,
            observed: 6,
        }),
    );
    assert_eq!(declared, declared_before);
    assert_eq!(
        verify_profile_entry(&declared, ProfileEntryEvidence {
            byte_length: 7,
            digest: digest(3),
        },),
        Err(ProfileEntryVerificationError::DigestMismatch),
    );
    assert_eq!(declared, declared_before);

    let inventory_before = current.clone();
    assert_eq!(
        validate_profile_entry_inventory(&current, &["stroke-vocabulary"], &[
            PROFILE_MANIFEST_PATH
        ],),
        Err(ProfileEntryInventoryError::MissingDeclaredEntry {
            path: String::from("assets/sample.bin"),
        }),
    );
    assert_eq!(current, inventory_before);
}

#[test]
fn future_version_and_unknown_required_feature_fail_closed() {
    let mut value = manifest(vec![]);
    value.container_version = String::from("atrament.profile/2");
    assert_eq!(
        validate_profile_manifest(&value, &["stroke-vocabulary"]),
        Err(ProfileManifestError::UnsupportedContainerVersion {
            observed: String::from("atrament.profile/2"),
        }),
    );

    let mut unsupported_value = manifest(vec![]);
    unsupported_value
        .required_features
        .push(String::from("future-required"));
    assert_eq!(
        validate_profile_manifest(&unsupported_value, &["stroke-vocabulary"]),
        Err(ProfileManifestError::UnsupportedRequiredFeature {
            feature: String::from("future-required"),
        }),
    );
}

#[test]
fn entry_evidence_checks_length_before_digest() {
    let value = entry("assets/sample.bin", "application/octet-stream", 7);
    assert_eq!(
        verify_profile_entry(&value, ProfileEntryEvidence {
            byte_length: 6,
            digest: digest(7),
        },),
        Err(ProfileEntryVerificationError::ByteLengthMismatch {
            declared: 7,
            observed: 6,
        }),
    );
    assert_eq!(
        verify_profile_entry(&value, ProfileEntryEvidence {
            byte_length: 8,
            digest: digest(1),
        },),
        Err(ProfileEntryVerificationError::ByteLengthMismatch {
            declared: 7,
            observed: 8,
        }),
    );
    assert_eq!(
        verify_profile_entry(&value, ProfileEntryEvidence {
            byte_length: 7,
            digest: digest(1),
        },),
        Err(ProfileEntryVerificationError::DigestMismatch),
    );
    assert_eq!(
        verify_profile_entry(&value, ProfileEntryEvidence {
            byte_length: 7,
            digest: digest(7),
        },),
        Ok(()),
    );
}

#[test]
fn admitted_inventory_seals_verified_entry_evidence() -> Result<(), String> {
    let value = manifest(vec![entry(
        "assets/sample.bin",
        "application/octet-stream",
        7,
    )]);
    let observed = [PROFILE_MANIFEST_PATH, "assets/sample.bin"];
    let validated =
        validate_profile_manifest_view(&value, &["stroke-vocabulary"])
            .map_err(|reason| format!("manifest is admitted: {reason:?}"))?;
    let inventory =
        validated
            .admit_entry_inventory(&observed)
            .map_err(|reason| {
                format!("inventory matches admitted manifest: {reason:?}")
            })?;
    let evidence = ProfileEntryEvidence {
        byte_length: 7,
        digest: digest(7),
    };
    let verified = inventory
        .verify_entry("assets/sample.bin", evidence)
        .map_err(|reason| format!("entry evidence matches: {reason:?}"))?
        .ok_or_else(|| String::from("declared path resolves"))?;
    check!(ptr::eq(
        verified.entry(),
        ptr::from_ref(
            value
                .entries
                .first()
                .ok_or_else(|| String::from("missing entry"))?
        )
    ));
    check_eq!(verified.evidence(), evidence);
    check_eq!(
        inventory.verify_entry(PROFILE_MANIFEST_PATH, evidence),
        Ok(None),
    );
    check_eq!(
        inventory.verify_entry("assets/missing.bin", evidence),
        Ok(None),
    );
    check_eq!(
        inventory.verify_entry("assets/../sample.bin", evidence),
        Ok(None),
    );
    Ok(())
}

#[test]
fn generated_full_range_entry_evidence_matches_reference_precedence()
-> Result<(), String> {
    const CASES: usize = 4_096;
    let mut seed = 0x5eed_e17e_2026u64;
    let mut saw_length_mismatch = false;
    let mut saw_digest_mismatch = false;
    let mut saw_valid = false;
    for case in 0..CASES {
        let declared_length = next_inventory_value(&mut seed);
        let observed_length =
            if next_inventory_selector(&mut seed).is_multiple_of(4) {
                declared_length
            } else {
                next_inventory_value(&mut seed)
            };
        let declared_digest_byte =
            next_inventory_value(&mut seed).to_le_bytes()[0];
        let observed_digest_byte =
            if next_inventory_selector(&mut seed).is_multiple_of(4) {
                declared_digest_byte
            } else {
                next_inventory_value(&mut seed).to_le_bytes()[0]
            };
        let mut declared = entry(
            "assets/generated.bin",
            "application/octet-stream",
            declared_digest_byte,
        );
        declared.byte_length = declared_length;
        let evidence = ProfileEntryEvidence {
            byte_length: observed_length,
            digest: digest(observed_digest_byte),
        };
        let expected = if declared_length != observed_length {
            saw_length_mismatch = true;
            Err(ProfileEntryVerificationError::ByteLengthMismatch {
                declared: declared_length,
                observed: observed_length,
            })
        } else if declared_digest_byte != observed_digest_byte {
            saw_digest_mismatch = true;
            Err(ProfileEntryVerificationError::DigestMismatch)
        } else {
            saw_valid = true;
            Ok(())
        };
        check_eq!(
            verify_profile_entry(&declared, evidence),
            expected,
            "generated entry evidence case {case}",
        );
        let value = manifest(vec![declared]);
        let observed = [PROFILE_MANIFEST_PATH, "assets/generated.bin"];
        let inventory =
            validate_profile_manifest_view(&value, &["stroke-vocabulary"])
                .map_err(|reason| {
                    format!("generated manifest remains valid: {reason:?}")
                })?
                .admit_entry_inventory(&observed)
                .map_err(|reason| {
                    format!("generated inventory remains exact: {reason:?}")
                })?;
        match expected {
            Ok(()) => {
                let verified = inventory
                    .verify_entry("assets/generated.bin", evidence)
                    .map_err(|reason| {
                        format!("matching evidence verifies: {reason:?}")
                    })?
                    .ok_or_else(|| {
                        String::from("generated declaration resolves")
                    })?;
                check!(ptr::eq(
                    verified.entry(),
                    ptr::from_ref(
                        value
                            .entries
                            .first()
                            .ok_or_else(|| String::from("missing entry"))?
                    )
                ));
                check_eq!(verified.evidence(), evidence);
            },
            Err(reason) => check_eq!(
                inventory.verify_entry("assets/generated.bin", evidence),
                Err(reason),
                "sealed entry evidence case {case}",
            ),
        }
    }
    check!(saw_length_mismatch);
    check!(saw_digest_mismatch);
    check!(saw_valid);
    Ok(())
}

#[test]
fn sealed_sorted_inventory_matches_general_admission_without_allocated_sets()
-> Result<(), String> {
    let value = manifest(vec![
        entry("sections/strokes.json", "application/json", 7),
        entry("assets/\u{f1}.webp", "image/webp", 9),
    ]);
    let admitted = validate_profile_manifest_view(&value, &[
        "stroke-vocabulary",
    ])
    .map_err(|reason| {
        format!(
            "valid typed manifest may keep unsorted declarations: {reason:?}"
        )
    })?;
    let paths = [
        "assets/\u{f1}.webp",
        "manifest.json",
        "sections/strokes.json",
    ];
    let sorted = admitted.admit_sorted_entry_inventory(&paths).map_err(
        |reason| {
            format!(
                "strictly increasing exact inventory is admitted: {reason:?}"
            )
        },
    )?;
    let general = admitted.admit_entry_inventory(&paths).map_err(|reason| {
        format!("general set-based comparison must agree: {reason:?}")
    })?;
    check_eq!(sorted, general);
    check_eq!(sorted.manifest(), &value);
    check_eq!(sorted.observed_paths(), paths);
    check!(ptr::eq(
        sorted
            .observed_paths()
            .first()
            .ok_or_else(|| String::from("missing observed path"))?
            .as_ptr(),
        paths
            .first()
            .ok_or_else(|| String::from("missing original path"))?
            .as_ptr()
    ));
    let evidence = ProfileEntryEvidence {
        byte_length: 9,
        digest: digest(9),
    };
    check!(
        sorted
            .verify_entry("assets/\u{f1}.webp", evidence)
            .map_err(|reason| format!(
                "same exact typed declaration: {reason:?}"
            ))?
            .is_some()
    );
    Ok(())
}

#[test]
fn sorted_inventory_fails_closed_for_inversion_duplicate_and_extra_names()
-> Result<(), String> {
    let value = manifest(vec![
        entry("sections/strokes.json", "application/json", 7),
        entry("assets/sample.webp", "image/webp", 9),
    ]);
    let admitted =
        validate_profile_manifest_view(&value, &["stroke-vocabulary"])
            .map_err(|reason| {
                format!("admitted manifest has safe unique paths: {reason:?}")
            })?;
    for (observed, expected) in [
        (
            vec!["assets/sample.webp", "assets/sample.webp", "manifest.json"],
            ProfileEntryInventoryError::DuplicateObservedEntryPath {
                path: String::from("assets/sample.webp"),
            },
        ),
        (
            vec![
                "manifest.json",
                "assets/sample.webp",
                "sections/strokes.json",
            ],
            ProfileEntryInventoryError::NonCanonicalObservedEntryOrder {
                earlier: String::from("manifest.json"),
                later: String::from("assets/sample.webp"),
            },
        ),
        (
            vec!["assets/sample.webp", "sections/strokes.json"],
            ProfileEntryInventoryError::MissingManifest,
        ),
        (
            vec!["manifest.json", "sections/strokes.json"],
            ProfileEntryInventoryError::MissingDeclaredEntry {
                path: String::from("assets/sample.webp"),
            },
        ),
        (
            vec![
                "assets/extra.webp",
                "assets/sample.webp",
                "manifest.json",
                "sections/strokes.json",
            ],
            ProfileEntryInventoryError::ObservedCountMismatch {
                expected: 3,
                observed: 4,
            },
        ),
        (
            vec![
                "assets/../sample.webp",
                "manifest.json",
                "sections/strokes.json",
            ],
            ProfileEntryInventoryError::InvalidObservedEntryPath {
                path: String::from("assets/../sample.webp"),
                reason: ProfileEntryPathError::TraversalSegment,
            },
        ),
    ] {
        check_eq!(
            admitted.admit_sorted_entry_inventory(&observed),
            Err(expected)
        );
    }
    check_eq!(
        admitted.admit_sorted_entry_inventory(&[
            "assets/extra.webp",
            "manifest.json",
            "sections/strokes.json",
        ]),
        Err(ProfileEntryInventoryError::MissingDeclaredEntry {
            path: String::from("assets/sample.webp"),
        })
    );
    Ok(())
}

#[test]
fn sorted_inventory_handles_largest_ordinary_zip32_path_count()
-> Result<(), String> {
    const NONROOT_COUNT: usize = 65_533;
    let entries = (0..NONROOT_COUNT)
        .map(|index| {
            entry(
                &format!("assets/{index:05}.bin"),
                "application/octet-stream",
                0,
            )
        })
        .collect::<Vec<_>>();
    let value = manifest(entries);
    let admitted =
        validate_profile_manifest_view(&value, &["stroke-vocabulary"])
            .map_err(|reason| {
                format!(
                    "large but finitely bounded typed declaration: {reason:?}"
                )
            })?;
    let mut observed = value
        .entries
        .iter()
        .map(|entry| entry.path.as_str())
        .collect::<Vec<_>>();
    observed.push(PROFILE_MANIFEST_PATH);
    let sealed =
        admitted
            .admit_sorted_entry_inventory(&observed)
            .map_err(|reason| {
                format!("sorted ZIP32 inventory refused: {reason:?}")
            })?;
    check_eq!(sealed.observed_paths().len(), NONROOT_COUNT + 1);
    check!(ptr::eq(
        sealed
            .observed_paths()
            .first()
            .ok_or_else(|| String::from("missing sealed path"))?
            .as_ptr(),
        observed
            .first()
            .ok_or_else(|| String::from("missing observed path"))?
            .as_ptr()
    ));
    let _removed_root = observed.pop();
    check_eq!(
        admitted.admit_sorted_entry_inventory(&observed),
        Err(ProfileEntryInventoryError::MissingManifest)
    );
    Ok(())
}

#[test]
fn generated_sorted_inventory_admission_matches_general_success_semantics()
-> Result<(), String> {
    let value = manifest(vec![
        entry("sections/\u{f1}.json", "application/json", 2),
        entry("assets/\u{1f58c}\u{fe0f}.webp", "image/webp", 1),
        entry("sections/x.json", "application/json", 4),
        entry("assets/a.webp", "image/webp", 3),
    ]);
    let admitted =
        validate_profile_manifest_view(&value, &["stroke-vocabulary"])
            .map_err(|reason| {
                format!(
                    "same manifest for both inventory validators: {reason:?}"
                )
            })?;
    let choices = [
        "assets/a.webp",
        "assets/extra.webp",
        "assets/\u{1f58c}\u{fe0f}.webp",
        "manifest.json",
        "sections/x.json",
        "sections/\u{f1}.json",
        "sections/z.json",
    ];
    let mut seed = 0x4154_5241_4d45_4e54u64;
    let mut saw_admission = false;
    let mut saw_rejection = false;
    for case in 0..8_192usize {
        seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let mask =
            usize::try_from((seed >> 16u32) & 127u64).map_err(|reason| {
                format!("seven-bit selection mask: {reason:?}")
            })?;
        let mut paths = choices
            .iter()
            .enumerate()
            .filter(|(index, _path)| (mask & (1usize << index)) != 0)
            .map(|(_index, path)| *path)
            .collect::<Vec<_>>();
        if case.is_multiple_of(5)
            && let Some(&first) = paths.first()
        {
            paths.insert(0, first);
        }
        let general = admitted.admit_entry_inventory(&paths);
        let sorted = admitted.admit_sorted_entry_inventory(&paths);
        check_eq!(
            general.is_ok(),
            sorted.is_ok(),
            "sorted names preserve general admission case={case}"
        );
        if general.is_ok() {
            saw_admission = true;
            check_eq!(general, sorted);
        } else {
            saw_rejection = true;
        }
    }
    check!(saw_admission);
    check!(saw_rejection);
    let reversed = [
        "manifest.json",
        "assets/a.webp",
        "assets/\u{1f58c}\u{fe0f}.webp",
        "sections/x.json",
        "sections/\u{f1}.json",
    ];
    check!(admitted.admit_entry_inventory(&reversed).is_ok());
    check!(matches!(
        admitted.admit_sorted_entry_inventory(&reversed),
        Err(ProfileEntryInventoryError::NonCanonicalObservedEntryOrder { .. }),
    ));
    Ok(())
}

#[test]
fn manifest_metadata_debug_never_discloses_private_profile_fields()
-> Result<(), String> {
    let secret_path = "sections/private-strokes-9471.json";
    let declaration = entry(secret_path, "application/private-3507", 1);
    let manifest = ProfileManifest {
        container_version: PROFILE_CONTAINER_VERSION.to_owned(),
        entries: vec![declaration.clone()],
        optional_features: vec!["optional-secret-1003".to_owned()],
        profile_identity: "writer-private-7349".to_owned(),
        required_features: vec!["required-secret-2805".to_owned()],
    };
    let admitted =
        validate_profile_manifest_view(&manifest, &["required-secret-2805"])
            .map_err(|reason| format!("valid private metadata: {reason:?}"))?;
    let paths = [PROFILE_MANIFEST_PATH, secret_path];
    let inventory = admitted
        .admit_sorted_entry_inventory(&paths)
        .map_err(|reason| format!("exact inventory: {reason:?}"))?;
    let verified = inventory
        .verify_entry(secret_path, ProfileEntryEvidence {
            byte_length: 1,
            digest: digest(1),
        })
        .map_err(|reason| {
            format!("verified matching SHA evidence: {reason:?}")
        })?
        .ok_or_else(|| String::from("declared entry exists"))?;
    for printed in [
        format!("{declaration:?} {declaration:#?}"),
        format!("{manifest:?} {manifest:#?}"),
        format!("{admitted:?} {admitted:#?}"),
        format!("{inventory:?} {inventory:#?}"),
        format!("{verified:?} {verified:#?}"),
    ] {
        for private in [
            secret_path,
            "application/private-3507",
            "writer-private-7349",
            "required-secret-2805",
            "optional-secret-1003",
        ] {
            check!(
                !printed.contains(private),
                "Debug leaked {private}: {printed}"
            );
        }
    }
    check_eq!(manifest.entries.first(), Some(&declaration));
    check_eq!(admitted.manifest(), &manifest);
    check_eq!(inventory.observed_paths(), paths);
    check_eq!(verified.entry(), &declaration);
    check_eq!(verified.evidence().byte_length, 1);
    Ok(())
}

#[test]
fn private_manifest_and_inventory_errors_have_redacted_debug() {
    let secret = "private-error-location-6921";
    let manifest_cases = [
        (
            ProfileManifestError::ConflictingFeatureRequirement {
                feature: secret.to_owned(),
            },
            "ConflictingFeatureRequirement",
        ),
        (
            ProfileManifestError::DuplicateEntryPath {
                path: secret.to_owned(),
            },
            "DuplicateEntryPath",
        ),
        (
            ProfileManifestError::DuplicateFeatureIdentifier {
                feature: secret.to_owned(),
                list: ProfileFeatureList::Required,
            },
            "DuplicateFeatureIdentifier",
        ),
        (
            ProfileManifestError::EmptyMediaType { path: secret.to_owned() },
            "EmptyMediaType",
        ),
        (
            ProfileManifestError::InvalidEntryPath {
                path: secret.to_owned(),
                reason: ProfileEntryPathError::TraversalSegment,
            },
            "InvalidEntryPath",
        ),
        (
            ProfileManifestError::InvalidMediaType { path: secret.to_owned() },
            "InvalidMediaType",
        ),
        (
            ProfileManifestError::UnsupportedContainerVersion {
                observed: secret.to_owned(),
            },
            "UnsupportedContainerVersion",
        ),
        (
            ProfileManifestError::UnsupportedRequiredFeature {
                feature: secret.to_owned(),
            },
            "UnsupportedRequiredFeature",
        ),
    ];
    for (reason, variant) in manifest_cases {
        let rendered = format!(
            "{reason:?} {reason:#?} {:?}",
            Err::<(), _>(reason.clone())
        );
        assert!(rendered.contains(variant), "variant must remain visible");
        assert!(!rendered.contains(secret), "private string leaked");
    }
}

#[test]
fn private_inventory_errors_have_redacted_debug() {
    let secret = "private-error-location-6921";
    let inventory_cases = [
        (
            ProfileEntryInventoryError::DuplicateObservedEntryPath {
                path: secret.to_owned(),
            },
            "DuplicateObservedEntryPath",
        ),
        (
            ProfileEntryInventoryError::InvalidManifest {
                reason: ProfileManifestError::UnsupportedRequiredFeature {
                    feature: secret.to_owned(),
                },
            },
            "InvalidManifest",
        ),
        (
            ProfileEntryInventoryError::InvalidObservedEntryPath {
                path: secret.to_owned(),
                reason: ProfileEntryPathError::ControlCharacter,
            },
            "InvalidObservedEntryPath",
        ),
        (
            ProfileEntryInventoryError::MissingDeclaredEntry {
                path: secret.to_owned(),
            },
            "MissingDeclaredEntry",
        ),
        (
            ProfileEntryInventoryError::MissingManifest,
            "MissingManifest",
        ),
        (
            ProfileEntryInventoryError::NonCanonicalObservedEntryOrder {
                earlier: secret.to_owned(),
                later: secret.to_owned(),
            },
            "NonCanonicalObservedEntryOrder",
        ),
        (
            ProfileEntryInventoryError::ObservedCountMismatch {
                expected: 2,
                observed: 3,
            },
            "ObservedCountMismatch",
        ),
        (
            ProfileEntryInventoryError::UndeclaredObservedEntry {
                path: secret.to_owned(),
            },
            "UndeclaredObservedEntry",
        ),
    ];
    for (reason, variant) in inventory_cases {
        let rendered = format!(
            "{reason:?} {reason:#?} {:?}",
            Err::<(), _>(reason.clone())
        );
        assert!(rendered.contains(variant), "variant must remain visible");
        assert!(!rendered.contains(secret), "private string leaked");
    }
    // The full offending text is still available through typed matching.
    let actual =
        ProfileManifestError::DuplicateEntryPath { path: secret.to_owned() };
    assert!(matches!(
        actual,
        ProfileManifestError::DuplicateEntryPath { path } if path == secret
    ));
}
