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

use atrament_handwriting_profile_container::{
    PROFILE_CONTAINER_VERSION, PROFILE_MANIFEST_PATH,
    ProfileArchiveEncodingError, ProfileArchiveEncodingEvidence,
    ProfileArchiveEntryEncoding, ProfileArchivePlatformExtras,
    ProfileContentChange, ProfileEntryEvidence,
    ProfileEntryInventoryError, ProfileEntryKind, ProfileEntryPathError,
    ProfileEntryVerificationError, ProfileManifest, ProfileManifestEntry,
    ProfileFeatureList, ProfileManifestError, ProfileRewriteDisposition,
    ProfileZip64Requirement,
    ProfileZip64Use, Sha256Digest, canonical_profile_archive_paths,
    canonical_profile_entry_order, profile_entry_kind,
    profile_rewrite_disposition, profile_section_entries,
    validate_profile_archive_encoding, validate_profile_archive_encoding_view,
    validate_profile_entry_inventory,
    validate_profile_manifest, validate_profile_manifest_view,
    verify_profile_entry,
};

fn digest(byte: u8) -> Sha256Digest {
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
fn every_archive_encoding_state_matches_canonical_reference() {
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
                for zip64_use in [
                    ProfileZip64Use::Absent,
                    ProfileZip64Use::Present,
                ] {
                    let evidence = ProfileArchiveEncodingEvidence {
                        entry_encoding,
                        platform_extras,
                        zip64_requirement,
                        zip64_use,
                    };
                    let expected = reference_archive_encoding(evidence);
                    assert_eq!(
                        validate_profile_archive_encoding(evidence),
                        expected,
                    );
                    match expected {
                        Ok(()) => assert_eq!(
                            validate_profile_archive_encoding_view(evidence)
                                .expect("canonical ZIP evidence seals")
                                .evidence(),
                            evidence,
                        ),
                        Err(reason) => assert_eq!(
                            validate_profile_archive_encoding_view(evidence),
                            Err(reason),
                        ),
                    }
                }
            }
        }
    }
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
        validate_profile_entry_inventory(
            &value,
            &["stroke-vocabulary"],
            &[
                "assets/sample.webp",
                PROFILE_MANIFEST_PATH,
                "sections/strokes.json",
            ],
        ),
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
        validate_profile_entry_inventory(
            &value,
            &["stroke-vocabulary"],
            &[PROFILE_MANIFEST_PATH, "sections/strokes.json"],
        ),
        Err(ProfileEntryInventoryError::MissingDeclaredEntry {
            path: String::from("assets/sample.webp"),
        }),
    );
    assert_eq!(
        validate_profile_entry_inventory(
            &value,
            &["stroke-vocabulary"],
            &[
                PROFILE_MANIFEST_PATH,
                "sections/strokes.json",
                "assets/sample.webp",
                "assets/extra.bin",
            ],
        ),
        Err(ProfileEntryInventoryError::UndeclaredObservedEntry {
            path: String::from("assets/extra.bin"),
        }),
    );
}

#[test]
fn archive_inventory_rejects_duplicate_unsafe_and_missing_manifest_paths() {
    let value = manifest(vec![entry(
        "sections/strokes.json",
        "application/json",
        7,
    )]);
    assert_eq!(
        validate_profile_entry_inventory(
            &value,
            &["stroke-vocabulary"],
            &[PROFILE_MANIFEST_PATH, PROFILE_MANIFEST_PATH],
        ),
        Err(ProfileEntryInventoryError::DuplicateObservedEntryPath {
            path: String::from(PROFILE_MANIFEST_PATH),
        }),
    );
    assert_eq!(
        validate_profile_entry_inventory(
            &value,
            &["stroke-vocabulary"],
            &[PROFILE_MANIFEST_PATH, "sections/../strokes.json"],
        ),
        Err(ProfileEntryInventoryError::InvalidObservedEntryPath {
            path: String::from("sections/../strokes.json"),
            reason: ProfileEntryPathError::TraversalSegment,
        }),
    );
    assert_eq!(
        validate_profile_entry_inventory(
            &value,
            &["stroke-vocabulary"],
            &["sections/strokes.json"],
        ),
        Err(ProfileEntryInventoryError::MissingManifest),
    );

    let mut future = value.clone();
    future.container_version = String::from("atrament.profile/2");
    assert_eq!(
        validate_profile_entry_inventory(
            &future,
            &["stroke-vocabulary"],
            &[PROFILE_MANIFEST_PATH, "sections/strokes.json"],
        ),
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
        if seen.iter().any(|previous| *previous == path.as_str()) {
            return Err(ProfileEntryInventoryError::DuplicateObservedEntryPath {
                path: path.clone(),
            });
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

fn next_inventory_value(seed: &mut u64) -> u64 {
    *seed = seed
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    *seed
}

fn next_inventory_selector(seed: &mut u64) -> usize {
    (next_inventory_value(seed) >> 32) as usize
}

#[test]
fn generated_archive_inventory_mutations_match_reference_oracle() {
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
        "assets/é.bin",
        "sections/value with space.json",
        "",
    ];
    let mut seed = 0x5eed_a2c4_2026_u64;
    let mut seen_operations = [false; 4];
    let mut saw_valid = false;
    let mut saw_duplicate = false;
    let mut saw_invalid_observed_path = false;
    let mut saw_inventory_backslash = false;
    let mut saw_inventory_empty_segment = false;
    let mut saw_inventory_traversal = false;
    let mut saw_inventory_unsupported_root = false;
    let mut saw_missing_manifest = false;
    let mut saw_missing_declared = false;
    let mut saw_undeclared_observed = false;
    let validated = validate_profile_manifest_view(
        &value,
        &["stroke-vocabulary"],
    )
    .expect("fixed inventory manifest is valid");
    assert!(std::ptr::eq(validated.manifest(), &value));
    for case in 0..CASES {
        let operation = next_inventory_selector(&mut seed) % 4;
        seen_operations[operation] = true;
        let member_index = next_inventory_selector(&mut seed) % canonical.len();
        let insertion_index =
            next_inventory_selector(&mut seed) % (canonical.len() + 1);
        let token_index = next_inventory_selector(&mut seed) % tokens.len();
        let mut observed = canonical
            .iter()
            .map(|path| String::from(*path))
            .collect::<Vec<_>>();
        match operation {
            0 => observed[member_index] = String::from(tokens[token_index]),
            1 => observed.insert(
                insertion_index,
                String::from(tokens[token_index]),
            ),
            2 => {
                let _removed = observed.remove(member_index);
            },
            _ => {
                let duplicate = observed[member_index].clone();
                observed.insert(insertion_index, duplicate);
            },
        }
        let observed_refs = observed
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        let expected = reference_inventory(&value, &observed);
        match &expected {
            Ok(()) => saw_valid = true,
            Err(
                ProfileEntryInventoryError::DuplicateObservedEntryPath { .. },
            ) => {
                saw_duplicate = true;
            },
            Err(ProfileEntryInventoryError::InvalidManifest { .. }) => {
                panic!("fixed valid manifest cannot fail inventory admission");
            },
            Err(
                ProfileEntryInventoryError::InvalidObservedEntryPath {
                    reason,
                    ..
                },
            ) => {
                saw_invalid_observed_path = true;
                match reason {
                    ProfileEntryPathError::BackslashSeparator => {
                        saw_inventory_backslash = true;
                    },
                    ProfileEntryPathError::ControlCharacter => {
                        saw_invalid_observed_path = true;
                    },
                    ProfileEntryPathError::EmptySegment => {
                        saw_inventory_empty_segment = true;
                    },
                    ProfileEntryPathError::TraversalSegment => {
                        saw_inventory_traversal = true;
                    },
                    ProfileEntryPathError::UnsupportedRoot => {
                        saw_inventory_unsupported_root = true;
                    },
                    ProfileEntryPathError::ManifestReserved => {
                        panic!(
                            "root manifest is valid inventory metadata",
                        );
                    },
                }
            },
            Err(ProfileEntryInventoryError::MissingDeclaredEntry { .. }) => {
                saw_missing_declared = true;
            },
            Err(ProfileEntryInventoryError::MissingManifest) => {
                saw_missing_manifest = true;
            },
            Err(ProfileEntryInventoryError::NonCanonicalObservedEntryOrder {
                ..
            } | ProfileEntryInventoryError::ObservedCountMismatch {
                ..
            }) => {
                panic!("generic oracle cannot return sorted errors");
            },
            Err(ProfileEntryInventoryError::UndeclaredObservedEntry { .. }) => {
                saw_undeclared_observed = true;
            },
        }
        assert_eq!(
            validate_profile_entry_inventory(
                &value,
                &["stroke-vocabulary"],
                &observed_refs,
            ),
            expected,
            "generated archive inventory case {case}",
        );
        assert_eq!(
            validated.validate_entry_inventory(&observed_refs),
            expected,
            "sealed archive inventory case {case}",
        );
        match (validated.admit_entry_inventory(&observed_refs), &expected) {
            (Ok(inventory), Ok(())) => {
                assert!(std::ptr::eq(inventory.manifest(), &value));
                assert!(std::ptr::eq(
                    inventory.observed_paths(),
                    observed_refs.as_slice(),
                ));
            },
            (Err(actual), Err(expected_error)) => {
                assert_eq!(&actual, expected_error);
            },
            (actual, expected) => panic!(
                "sealed inventory mismatch in case {case}: \
                 {actual:?} vs {expected:?}",
            ),
        }
    }
    assert!(seen_operations.into_iter().all(|seen| seen));
    assert!(saw_valid);
    assert!(saw_duplicate);
    assert!(saw_invalid_observed_path);
    assert!(saw_inventory_backslash);
    assert!(saw_inventory_empty_segment);
    assert!(saw_inventory_traversal);
    assert!(saw_inventory_unsupported_root);
    assert!(saw_missing_manifest);
    assert!(saw_missing_declared);
    assert!(saw_undeclared_observed);
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
        if value.optional_features[..index].contains(&feature) {
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
        if paths.iter().any(|path| *path == item.path.as_str()) {
            return Err(ProfileManifestError::DuplicateEntryPath {
                path: item.path.clone(),
            });
        }
        paths.push(item.path.as_str());
    }
    Ok(())
}

#[test]
fn generated_manifest_values_match_reference_admission_oracle() {
    const CASES: usize = 4_096;
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
        "assets/é.bin",
        "sections/%2e%2e/value.json",
        PROFILE_MANIFEST_PATH,
        "sections/../value.json",
        r"assets\sample.webp",
        "sections//value.json",
        "other/value.bin",
        "",
    ];
    let media_types = [
        "application/json", "image/webp", "", "opaque/type",
        " ", "text/plain\r\ninjected", "image/\0webp",
    ];
    let supported = ["stroke-vocabulary", "future-supported"];
    let mut seed = 0x5eed_4d41_2026_u64;
    let mut seen_versions = [false; 4];
    let mut seen_feature_sets = [false; 5];
    let mut seen_entry_counts = [false; 5];
    let mut seen_paths = [false; 11];
    let mut seen_media_types = [false; 7];
    let mut saw_valid = false;
    let mut saw_duplicate_path = false;
    let mut saw_empty_media_type = false;
    let mut saw_invalid_media_type = false;
    let mut saw_invalid_path = false;
    let mut saw_manifest_backslash = false;
    let mut saw_manifest_empty_segment = false;
    let mut saw_manifest_reserved = false;
    let mut saw_manifest_traversal = false;
    let mut saw_manifest_unsupported_root = false;
    let mut saw_unsupported_version = false;
    let mut saw_unsupported_feature = false;
    let mut saw_conflicting_feature_requirement = false;
    let mut saw_duplicate_required_feature = false;
    let mut saw_duplicate_optional_feature = false;
    for case in 0..CASES {
        let version_index =
            next_inventory_selector(&mut seed) % versions.len();
        seen_versions[version_index] = true;
        let feature_index =
            next_inventory_selector(&mut seed) % feature_sets.len();
        seen_feature_sets[feature_index] = true;
        let entry_count = next_inventory_selector(&mut seed) % 5;
        seen_entry_counts[entry_count] = true;
        let mut entries: Vec<ProfileManifestEntry> =
            Vec::with_capacity(entry_count);
        for entry_index in 0..entry_count {
            let path_index =
                next_inventory_selector(&mut seed) % paths.len();
            seen_paths[path_index] = true;
            let media_index =
                next_inventory_selector(&mut seed) % media_types.len();
            seen_media_types[media_index] = true;
            let selected_path = if entry_index > 0
                && next_inventory_selector(&mut seed).is_multiple_of(5)
            {
                entries[0].path.as_str()
            } else {
                paths[path_index]
            };
            entries.push(entry(
                selected_path,
                media_types[media_index],
                u8::try_from(entry_index + 1)
                    .expect("generated entry count stays small"),
            ));
        }
        let mut optional_features = if case.is_multiple_of(7) {
            vec![String::from("stroke-vocabulary")]
        } else {
            vec![String::from("future-optional")]
        };
        if case.is_multiple_of(11) {
            optional_features.push(optional_features[0].clone());
        }
        let mut required_features = feature_sets[feature_index]
            .iter()
            .map(|feature| String::from(*feature))
            .collect::<Vec<_>>();
        if case.is_multiple_of(13) {
            required_features.push(required_features[0].clone());
        }
        let value = ProfileManifest {
            container_version: String::from(versions[version_index]),
            entries,
            optional_features,
            profile_identity: String::from("writer-generated"),
            required_features,
        };
        let expected = reference_manifest(&value, &supported);
        match &expected {
            Ok(()) => saw_valid = true,
            Err(ProfileManifestError::ConflictingFeatureRequirement { .. }) => {
                saw_conflicting_feature_requirement = true;
            },
            Err(ProfileManifestError::DuplicateFeatureIdentifier {
                list, ..
            }) => {
                match list {
                    ProfileFeatureList::Required => {
                        saw_duplicate_required_feature = true;
                    }
                    ProfileFeatureList::Optional => {
                        saw_duplicate_optional_feature = true;
                    }
                }
            }
            Err(ProfileManifestError::DuplicateEntryPath { .. }) => {
                saw_duplicate_path = true;
            },
            Err(ProfileManifestError::EmptyMediaType { .. }) => {
                saw_empty_media_type = true;
            },
            Err(ProfileManifestError::InvalidMediaType { .. }) => {
                saw_invalid_media_type = true;
            },
            Err(ProfileManifestError::InvalidEntryPath { reason, .. }) => {
                saw_invalid_path = true;
                match reason {
                    ProfileEntryPathError::BackslashSeparator => {
                        saw_manifest_backslash = true;
                    },
                    ProfileEntryPathError::ControlCharacter => {
                        saw_invalid_path = true;
                    },
                    ProfileEntryPathError::EmptySegment => {
                        saw_manifest_empty_segment = true;
                    },
                    ProfileEntryPathError::ManifestReserved => {
                        saw_manifest_reserved = true;
                    },
                    ProfileEntryPathError::TraversalSegment => {
                        saw_manifest_traversal = true;
                    },
                    ProfileEntryPathError::UnsupportedRoot => {
                        saw_manifest_unsupported_root = true;
                    },
                }
            },
            Err(ProfileManifestError::UnsupportedContainerVersion { .. }) => {
                saw_unsupported_version = true;
            },
            Err(ProfileManifestError::UnsupportedRequiredFeature { .. }) => {
                saw_unsupported_feature = true;
            },
        }
        assert_eq!(
            validate_profile_manifest(&value, &supported),
            expected,
            "generated manifest case {case}",
        );
        match (
            validate_profile_manifest_view(&value, &supported),
            &expected,
        ) {
            (Ok(validated), Ok(())) => {
                assert!(std::ptr::eq(validated.manifest(), &value));
                for entry in &value.entries {
                    let resolved = validated
                        .entry(&entry.path)
                        .expect("valid declaration resolves exactly");
                    assert!(std::ptr::eq(resolved, entry));
                }
            },
            (Err(actual), Err(expected_error)) => {
                assert_eq!(&actual, expected_error);
            },
            (actual, expected) => panic!(
                "sealed manifest mismatch in case {case}: \
                 {actual:?} vs {expected:?}",
            ),
        }
    }
    assert!(seen_versions.into_iter().all(|seen| seen));
    assert!(seen_feature_sets.into_iter().all(|seen| seen));
    assert!(seen_entry_counts.into_iter().all(|seen| seen));
    assert!(seen_paths.into_iter().all(|seen| seen));
    assert!(seen_media_types.into_iter().all(|seen| seen));
    assert!(saw_valid);
    assert!(saw_duplicate_path);
    assert!(saw_empty_media_type);
    assert!(saw_invalid_media_type);
    assert!(saw_invalid_path);
    assert!(saw_manifest_backslash);
    assert!(saw_manifest_empty_segment);
    assert!(saw_manifest_reserved);
    assert!(saw_manifest_traversal);
    assert!(saw_manifest_unsupported_root);
    assert!(saw_unsupported_version);
    assert!(saw_unsupported_feature);
    assert!(saw_conflicting_feature_requirement);
    assert!(saw_duplicate_required_feature);
    assert!(saw_duplicate_optional_feature);
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
    let mut value = manifest(vec![
        entry("sections/a.json", "application/json", 1),
    ]);
    for feature in ["stroke-vocabulary", "future-optional", "ma\u{f1}ana"] {
        value.required_features = vec![feature.to_string()];
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
    value.optional_features = vec![
        "unknown".into(), "future".into(), "unknown".into(),
    ];
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
        "é".into(),
        "e\u{301}".into(),
    ];
    assert_eq!(validate_profile_manifest(&value, &["strokes"]), Ok(()));
}

#[test]
fn large_feature_lists_use_exact_membership_and_first_failure_order() {
    let mut value = manifest(vec![]);
    value.required_features = (0..4096)
        .map(|index| format!("required-{index}"))
        .collect();
    value.optional_features = (0..4096)
        .map(|index| format!("optional-{index}"))
        .collect();
    let required_copy = value.required_features.clone();
    let supported = required_copy.iter()
        .map(String::as_str).collect::<Vec<_>>();
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
    let _removed = value.required_features.pop();
    value.optional_features.push("required-4095".into());
    assert_eq!(
        validate_profile_manifest(&value, &supported),
        Err(ProfileManifestError::ConflictingFeatureRequirement {
            feature: "required-4095".into(),
        }),
    );
    assert_eq!(
        validate_profile_manifest(&value, &supported[..4095]),
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
    let Some(pivot) = (0..values.len().saturating_sub(1))
        .rev()
        .find(|&index| values[index] < values[index + 1])
    else {
        return false;
    };
    let successor = (pivot + 1..values.len())
        .rev()
        .find(|&index| values[pivot] < values[index])
        .expect("nonterminal permutation has a successor");
    values.swap(pivot, successor);
    values[pivot + 1..].reverse();
    true
}

#[test]
fn every_declaration_order_has_identical_canonical_profile_projections() {
    let declarations = [
        ("sections/z.json", "application/json", 1_u8),
        ("assets/b.bin", "application/octet-stream", 2_u8),
        ("sections/a.json", "application/json", 3_u8),
        ("assets/y.bin", "application/octet-stream", 4_u8),
        ("sections/m.json", "application/json", 5_u8),
        ("assets/c.bin", "application/octet-stream", 6_u8),
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
    let expected_section_paths = [
        "sections/a.json",
        "sections/m.json",
        "sections/z.json",
    ];
    let mut order = [0_usize, 1, 2, 3, 4, 5];
    let mut cases = 0_usize;
    loop {
        let entries = order
            .iter()
            .map(|&index| {
                let (path, media_type, byte) = declarations[index];
                entry(path, media_type, byte)
            })
            .collect::<Vec<_>>();
        let value = manifest(entries);
        let original_order = value
            .entries
            .iter()
            .map(|item| item.path.clone())
            .collect::<Vec<_>>();
        let validated = validate_profile_manifest_view(
            &value,
            &["stroke-vocabulary"],
        )
        .expect("valid declarations seal one manifest");
        assert!(std::ptr::eq(validated.manifest(), &value));
        for expected_path in expected_entry_paths {
            let resolved = validated
                .entry(expected_path)
                .expect("canonical declared path resolves exactly");
            assert_eq!(resolved.path, expected_path);
            assert!(value.entries.iter().any(|entry| {
                std::ptr::eq(entry, resolved) && entry.path == expected_path
            }));
        }
        assert_eq!(validated.entry(PROFILE_MANIFEST_PATH), None);
        assert_eq!(validated.entry("sections/missing.json"), None);
        assert_eq!(validated.entry("sections/../a.json"), None);
        assert_eq!(
            validated.canonical_archive_paths(),
            expected_archive_paths,
            "sealed archive order for permutation {order:?}",
        );
        assert_eq!(
            validated
                .canonical_entry_order()
                .iter()
                .map(|item| item.path.as_str())
                .collect::<Vec<_>>(),
            expected_entry_paths,
            "sealed entry order for permutation {order:?}",
        );
        assert_eq!(
            validated
                .section_entries()
                .iter()
                .map(|item| item.path.as_str())
                .collect::<Vec<_>>(),
            expected_section_paths,
            "sealed section order for permutation {order:?}",
        );

        assert_eq!(
            canonical_profile_archive_paths(&value, &["stroke-vocabulary"]),
            Ok(expected_archive_paths.to_vec()),
            "archive order for permutation {order:?}",
        );
        let entry_order = canonical_profile_entry_order(
            &value,
            &["stroke-vocabulary"],
        )
        .expect("valid declarations have canonical entry order");
        assert_eq!(
            entry_order
                .iter()
                .map(|item| item.path.as_str())
                .collect::<Vec<_>>(),
            expected_entry_paths,
            "entry order for permutation {order:?}",
        );
        let sections = profile_section_entries(&value, &["stroke-vocabulary"])
            .expect("valid declarations have section projection");
        assert_eq!(
            sections
                .iter()
                .map(|item| item.path.as_str())
                .collect::<Vec<_>>(),
            expected_section_paths,
            "section order for permutation {order:?}",
        );
        assert_eq!(
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
        cases += 1;
        if !next_index_permutation(&mut order) {
            break;
        }
    }
    assert_eq!(cases, 720);
}

#[test]
fn canonical_order_is_path_sorted_without_rewriting_manifest() {
    let value = manifest(vec![
        entry("sections/z.json", "application/json", 3),
        entry("assets/b.bin", "application/octet-stream", 2),
        entry("sections/a.json", "application/json", 1),
    ]);
    let ordered = canonical_profile_entry_order(&value, &["stroke-vocabulary"])
        .expect("valid manifest has canonical entry ordering");
    assert_eq!(
        ordered.iter().map(|entry| entry.path.as_str()).collect::<Vec<_>>(),
        ["assets/b.bin", "sections/a.json", "sections/z.json"],
    );
    assert_eq!(value.entries[0].path, "sections/z.json");
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
            let value = manifest(vec![]);
            assert_eq!(
                validate_profile_entry_inventory(
                    &value,
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
        ("sections/../sample.json", ProfileEntryPathError::TraversalSegment),
        ("sections//sample.json", ProfileEntryPathError::EmptySegment),
        (r"sections\sample.json", ProfileEntryPathError::BackslashSeparator),
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
    for media_type in [
        "image/\0webp", "text/plain\r\ninjected", "image/\u{0085}x",
    ] {
        let value = manifest(vec![entry(entry_path, media_type, 1)]);
        assert_eq!(
            validate_profile_manifest(&value, &["stroke-vocabulary"]),
            Err(ProfileManifestError::InvalidMediaType {
                path: String::from(entry_path),
            }),
            "media_type={media_type:?}",
        );
    }
    let value = manifest(vec![entry(
        entry_path,
        "text/plain; charset=utf-8",
        1,
    )]);
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
fn generated_profile_path_corpus_pins_safe_path_admission() {
    let segments = [
        "",
        ".",
        "..",
        "%2e%2e",
        "a",
        "café",
        "sample.json",
        "space name",
        "x\0y",
        "x\ny",
        r"x\y",
    ];
    let mut exercised = 0_usize;
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
                    let first_segment_error = [left, right]
                        .into_iter()
                        .find_map(|segment| {
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
                        panic!(
                            "root-scoped Cartesian path has unexpected class",
                        );
                    },
                }
                assert_eq!(profile_entry_kind(&path), expected, "{path}");
                exercised += 1;
            }
        }
    }
    assert_eq!(exercised, 242);
    assert!(saw_control);
    assert!(saw_asset);
    assert!(saw_section);
    assert!(saw_backslash);
    assert!(saw_empty_segment);
    assert!(saw_traversal);

    for path in [
        "assets",
        "other/sample.json",
        "sections",
        "sample.json",
    ] {
        assert_eq!(
            profile_entry_kind(path),
            Err(ProfileEntryPathError::UnsupportedRoot),
            "{path}",
        );
    }
    assert_eq!(
        profile_entry_kind(r"other\sample.json"),
        Err(ProfileEntryPathError::BackslashSeparator),
    );
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
fn section_projection_is_sorted_and_never_returns_opaque_assets() {
    let value = manifest(vec![
        entry("sections/z.json", "application/json", 3),
        entry("assets/source.bin", "application/octet-stream", 2),
        entry("sections/a.json", "application/json", 1),
    ]);
    let sections = profile_section_entries(&value, &["stroke-vocabulary"])
        .expect("valid manifest exposes typed section metadata");
    assert_eq!(
        sections
            .iter()
            .map(|entry| entry.path.as_str())
            .collect::<Vec<_>>(),
        ["sections/a.json", "sections/z.json"],
    );
    assert_eq!(value.entries.len(), 3);
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
        validate_profile_manifest(
            &unsupported_required,
            &["stroke-vocabulary"],
        ),
        Err(ProfileManifestError::UnsupportedRequiredFeature {
            feature: String::from("future-required"),
        }),
    );
    assert_eq!(unsupported_required, required_before);

    let declared = current.entries[0].clone();
    let declared_before = declared.clone();
    assert_eq!(
        verify_profile_entry(
            &declared,
            ProfileEntryEvidence {
                byte_length: 6,
                digest: digest(7),
            },
        ),
        Err(ProfileEntryVerificationError::ByteLengthMismatch {
            declared: 7,
            observed: 6,
        }),
    );
    assert_eq!(declared, declared_before);
    assert_eq!(
        verify_profile_entry(
            &declared,
            ProfileEntryEvidence {
                byte_length: 7,
                digest: digest(3),
            },
        ),
        Err(ProfileEntryVerificationError::DigestMismatch),
    );
    assert_eq!(declared, declared_before);

    let inventory_before = current.clone();
    assert_eq!(
        validate_profile_entry_inventory(
            &current,
            &["stroke-vocabulary"],
            &[PROFILE_MANIFEST_PATH],
        ),
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

    let mut value = manifest(vec![]);
    value.required_features.push(String::from("future-required"));
    assert_eq!(
        validate_profile_manifest(&value, &["stroke-vocabulary"]),
        Err(ProfileManifestError::UnsupportedRequiredFeature {
            feature: String::from("future-required"),
        }),
    );
}

#[test]
fn entry_evidence_checks_length_before_digest() {
    let value = entry("assets/sample.bin", "application/octet-stream", 7);
    assert_eq!(
        verify_profile_entry(
            &value,
            ProfileEntryEvidence {
                byte_length: 6,
                digest: digest(7),
            },
        ),
        Err(ProfileEntryVerificationError::ByteLengthMismatch {
            declared: 7,
            observed: 6,
        }),
    );
    assert_eq!(
        verify_profile_entry(
            &value,
            ProfileEntryEvidence {
                byte_length: 8,
                digest: digest(1),
            },
        ),
        Err(ProfileEntryVerificationError::ByteLengthMismatch {
            declared: 7,
            observed: 8,
        }),
    );
    assert_eq!(
        verify_profile_entry(
            &value,
            ProfileEntryEvidence {
                byte_length: 7,
                digest: digest(1),
            },
        ),
        Err(ProfileEntryVerificationError::DigestMismatch),
    );
    assert_eq!(
        verify_profile_entry(
            &value,
            ProfileEntryEvidence {
                byte_length: 7,
                digest: digest(7),
            },
        ),
        Ok(()),
    );
}


#[test]
fn admitted_inventory_seals_verified_entry_evidence() {
    let value = manifest(vec![entry(
        "assets/sample.bin",
        "application/octet-stream",
        7,
    )]);
    let observed = [PROFILE_MANIFEST_PATH, "assets/sample.bin"];
    let validated = validate_profile_manifest_view(
        &value,
        &["stroke-vocabulary"],
    )
    .expect("manifest is admitted");
    let inventory = validated
        .admit_entry_inventory(&observed)
        .expect("inventory matches admitted manifest");
    let evidence = ProfileEntryEvidence {
        byte_length: 7,
        digest: digest(7),
    };
    let verified = inventory
        .verify_entry("assets/sample.bin", evidence)
        .expect("entry evidence matches")
        .expect("declared path resolves");
    assert!(std::ptr::eq(verified.entry(), &value.entries[0]));
    assert_eq!(verified.evidence(), evidence);
    assert_eq!(
        inventory.verify_entry(PROFILE_MANIFEST_PATH, evidence),
        Ok(None),
    );
    assert_eq!(
        inventory.verify_entry("assets/missing.bin", evidence),
        Ok(None),
    );
    assert_eq!(
        inventory.verify_entry("assets/../sample.bin", evidence),
        Ok(None),
    );
}

#[test]
fn generated_full_range_entry_evidence_matches_reference_precedence() {
    const CASES: usize = 4_096;
    let mut seed = 0x5eed_e17e_2026_u64;
    let mut saw_length_mismatch = false;
    let mut saw_digest_mismatch = false;
    let mut saw_valid = false;
    for case in 0..CASES {
        let declared_length = next_inventory_value(&mut seed);
        let observed_length = if next_inventory_selector(&mut seed) & 3 == 0 {
            declared_length
        } else {
            next_inventory_value(&mut seed)
        };
        let declared_digest_byte =
            next_inventory_value(&mut seed).to_le_bytes()[0];
        let observed_digest_byte =
            if next_inventory_selector(&mut seed) & 3 == 0 {
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
        assert_eq!(
            verify_profile_entry(&declared, evidence),
            expected,
            "generated entry evidence case {case}",
        );
        let value = manifest(vec![declared]);
        let observed = [PROFILE_MANIFEST_PATH, "assets/generated.bin"];
        let inventory = validate_profile_manifest_view(
            &value,
            &["stroke-vocabulary"],
        )
        .expect("generated manifest remains valid")
        .admit_entry_inventory(&observed)
        .expect("generated inventory remains exact");
        match expected {
            Ok(()) => {
                let verified = inventory
                    .verify_entry("assets/generated.bin", evidence)
                    .expect("matching evidence verifies")
                    .expect("generated declaration resolves");
                assert!(std::ptr::eq(verified.entry(), &value.entries[0]));
                assert_eq!(verified.evidence(), evidence);
            },
            Err(reason) => assert_eq!(
                inventory.verify_entry("assets/generated.bin", evidence),
                Err(reason),
                "sealed entry evidence case {case}",
            ),
        }
    }
    assert!(saw_length_mismatch);
    assert!(saw_digest_mismatch);
    assert!(saw_valid);
}

#[test]
fn sealed_sorted_inventory_matches_general_admission_without_allocated_sets() {
    let value = manifest(vec![
        entry("sections/strokes.json", "application/json", 7),
        entry("assets/ñ.webp", "image/webp", 9),
    ]);
    let admitted =
        validate_profile_manifest_view(&value, &["stroke-vocabulary"])
            .expect("valid typed manifest may keep unsorted declarations");
    let paths = ["assets/ñ.webp", "manifest.json", "sections/strokes.json"];
    let sorted = admitted.admit_sorted_entry_inventory(&paths)
        .expect("strictly increasing exact inventory is admitted");
    let general = admitted.admit_entry_inventory(&paths)
        .expect("general set-based comparison must agree");
    assert_eq!(sorted, general);
    assert_eq!(sorted.manifest(), &value);
    assert_eq!(sorted.observed_paths(), paths);
    assert!(std::ptr::eq(sorted.observed_paths()[0].as_ptr(),
        paths[0].as_ptr()));
    let evidence = ProfileEntryEvidence {
        byte_length: 9,
        digest: digest(9),
    };
    assert!(sorted.verify_entry("assets/ñ.webp", evidence)
        .expect("same exact typed declaration").is_some());
}

#[test]
fn sorted_inventory_fails_closed_for_inversion_duplicate_and_extra_names() {
    let value = manifest(vec![
        entry("sections/strokes.json", "application/json", 7),
        entry("assets/sample.webp", "image/webp", 9),
    ]);
    let admitted =
        validate_profile_manifest_view(&value, &["stroke-vocabulary"])
            .expect("admitted manifest has safe unique paths");
    for (observed, expected) in [
        (vec!["assets/sample.webp", "assets/sample.webp", "manifest.json"],
            ProfileEntryInventoryError::DuplicateObservedEntryPath {
                path: String::from("assets/sample.webp"),
            }),
        (vec!["manifest.json", "assets/sample.webp", "sections/strokes.json"],
            ProfileEntryInventoryError::NonCanonicalObservedEntryOrder {
                earlier: String::from("manifest.json"),
                later: String::from("assets/sample.webp"),
            }),
        (vec!["assets/sample.webp", "sections/strokes.json"],
            ProfileEntryInventoryError::MissingManifest),
        (vec!["manifest.json", "sections/strokes.json"],
            ProfileEntryInventoryError::MissingDeclaredEntry {
                path: String::from("assets/sample.webp"),
            }),
        (vec!["assets/extra.webp", "assets/sample.webp", "manifest.json",
            "sections/strokes.json"],
            ProfileEntryInventoryError::ObservedCountMismatch {
                expected: 3,
                observed: 4,
            }),
        (vec!["assets/../sample.webp", "manifest.json",
            "sections/strokes.json"],
            ProfileEntryInventoryError::InvalidObservedEntryPath {
                path: String::from("assets/../sample.webp"),
                reason: ProfileEntryPathError::TraversalSegment,
            }),
    ] {
        assert_eq!(admitted.admit_sorted_entry_inventory(&observed),
            Err(expected));
    }
    assert_eq!(admitted.admit_sorted_entry_inventory(&[
        "assets/extra.webp", "manifest.json", "sections/strokes.json",
    ]), Err(ProfileEntryInventoryError::MissingDeclaredEntry {
        path: String::from("assets/sample.webp"),
    }));
}

#[test]
fn sorted_inventory_handles_largest_ordinary_zip32_path_count() {
    const NONROOT_COUNT: usize = 65_533;
    let entries = (0..NONROOT_COUNT)
        .map(|index| entry(&format!("assets/{index:05}.bin"),
            "application/octet-stream", 0))
        .collect::<Vec<_>>();
    let value = manifest(entries);
    let admitted =
        validate_profile_manifest_view(&value, &["stroke-vocabulary"])
            .expect("large but finitely bounded typed declaration");
    let mut observed = value.entries.iter()
        .map(|entry| entry.path.as_str()).collect::<Vec<_>>();
    observed.push(PROFILE_MANIFEST_PATH);
    let sealed = admitted.admit_sorted_entry_inventory(&observed)
        .expect("65534 sorted observed names admit without membership sets");
    assert_eq!(sealed.observed_paths().len(), NONROOT_COUNT + 1);
    assert!(std::ptr::eq(sealed.observed_paths()[0].as_ptr(),
        observed[0].as_ptr()));
    let _removed_root = observed.pop();
    assert_eq!(admitted.admit_sorted_entry_inventory(&observed),
        Err(ProfileEntryInventoryError::MissingManifest));
}

#[test]
fn generated_sorted_inventory_admission_matches_general_success_semantics() {
    let value = manifest(vec![
        entry("sections/ñ.json", "application/json", 2),
        entry("assets/🖌️.webp", "image/webp", 1),
        entry("sections/x.json", "application/json", 4),
        entry("assets/a.webp", "image/webp", 3),
    ]);
    let admitted =
        validate_profile_manifest_view(&value, &["stroke-vocabulary"])
            .expect("same manifest for both inventory validators");
    let choices = [
        "assets/a.webp", "assets/extra.webp", "assets/🖌️.webp",
        "manifest.json", "sections/x.json", "sections/ñ.json",
        "sections/z.json",
    ];
    let mut seed = 0x4154_5241_4d45_4e54u64;
    let mut saw_admission = false;
    let mut saw_rejection = false;
    for case in 0..8_192usize {
        seed = seed.wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let mask = usize::try_from((seed >> 16) & 127)
            .expect("seven-bit selection mask");
        let mut paths = choices.iter().enumerate()
            .filter(|(index, _path)| (mask & (1 << index)) != 0)
            .map(|(_index, path)| *path)
            .collect::<Vec<_>>();
        if case % 5 == 0 && !paths.is_empty() {
            paths.insert(0, paths[0]);
        }
        let general = admitted.admit_entry_inventory(&paths);
        let sorted = admitted.admit_sorted_entry_inventory(&paths);
        assert_eq!(general.is_ok(), sorted.is_ok(),
            "sorted names preserve general admission case={case}");
        if general.is_ok() {
            saw_admission = true;
            assert_eq!(general, sorted);
        } else {
            saw_rejection = true;
        }
    }
    assert!(saw_admission);
    assert!(saw_rejection);
    let reversed = [
        "manifest.json", "assets/a.webp", "assets/🖌️.webp",
        "sections/x.json", "sections/ñ.json",
    ];
    assert!(admitted.admit_entry_inventory(&reversed).is_ok());
    assert!(matches!(
        admitted.admit_sorted_entry_inventory(&reversed),
        Err(ProfileEntryInventoryError::NonCanonicalObservedEntryOrder {
            ..
        }),
    ));
}

#[test]
fn manifest_metadata_debug_never_discloses_private_profile_fields() {
    let secret_path = "sections/private-strokes-9471.json";
    let declaration = entry(secret_path, "application/private-3507", 1);
    let manifest = ProfileManifest {
        container_version: PROFILE_CONTAINER_VERSION.to_owned(),
        entries: vec![declaration.clone()],
        optional_features: vec!["optional-secret-1003".to_owned()],
        profile_identity: "writer-private-7349".to_owned(),
        required_features: vec!["required-secret-2805".to_owned()],
    };
    let admitted = validate_profile_manifest_view(
        &manifest,
        &["required-secret-2805"],
    )
    .expect("valid private metadata");
    let paths = [PROFILE_MANIFEST_PATH, secret_path];
    let inventory = admitted
        .admit_sorted_entry_inventory(&paths)
        .expect("exact inventory");
    let verified = inventory
        .verify_entry(
            secret_path,
            ProfileEntryEvidence {
                byte_length: 1,
                digest: digest(1),
            },
        )
        .expect("verified matching SHA evidence")
        .expect("declared entry exists");
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
            assert!(
                !printed.contains(private),
                "Debug leaked {private}: {printed}"
            );
        }
    }
    assert_eq!(manifest.entries[0], declaration);
    assert_eq!(admitted.manifest(), &manifest);
    assert_eq!(inventory.observed_paths(), paths);
    assert_eq!(verified.entry(), &declaration);
    assert_eq!(verified.evidence().byte_length, 1);
}
