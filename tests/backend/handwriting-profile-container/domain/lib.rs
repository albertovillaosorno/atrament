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
use atrament_handwriting_profile_container::{
    PROFILE_CONTAINER_VERSION, PROFILE_MANIFEST_PATH,
    ProfileArchiveEncodingError, ProfileArchiveEncodingEvidence,
    ProfileArchiveEntryEncoding, ProfileArchivePlatformExtras,
    ProfileContentChange, ProfileEntryEvidence,
    ProfileEntryInventoryError, ProfileEntryKind, ProfileEntryPathError,
    ProfileEntryVerificationError, ProfileManifest, ProfileManifestEntry,
    ProfileManifestError, ProfileRewriteDisposition, ProfileZip64Requirement,
    ProfileZip64Use, Sha256Digest, canonical_profile_archive_paths,
    canonical_profile_entry_order, profile_entry_kind,
    profile_rewrite_disposition, profile_section_entries,
    validate_profile_archive_encoding, validate_profile_entry_inventory,
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
                    assert_eq!(
                        validate_profile_archive_encoding(evidence),
                        reference_archive_encoding(evidence),
                    );
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
    for feature in &value.required_features {
        if !supported.contains(&feature.as_str()) {
            return Err(ProfileManifestError::UnsupportedRequiredFeature {
                feature: feature.clone(),
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
        if item.media_type.is_empty() {
            return Err(ProfileManifestError::EmptyMediaType {
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
    let media_types = ["application/json", "image/webp", "", "opaque/type"];
    let supported = ["stroke-vocabulary", "future-supported"];
    let mut seed = 0x5eed_4d41_2026_u64;
    let mut seen_versions = [false; 4];
    let mut seen_feature_sets = [false; 5];
    let mut seen_entry_counts = [false; 5];
    let mut seen_paths = [false; 11];
    let mut seen_media_types = [false; 4];
    let mut saw_valid = false;
    let mut saw_duplicate_path = false;
    let mut saw_empty_media_type = false;
    let mut saw_invalid_path = false;
    let mut saw_manifest_backslash = false;
    let mut saw_manifest_empty_segment = false;
    let mut saw_manifest_reserved = false;
    let mut saw_manifest_traversal = false;
    let mut saw_manifest_unsupported_root = false;
    let mut saw_unsupported_version = false;
    let mut saw_unsupported_feature = false;
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
        let value = ProfileManifest {
            container_version: String::from(versions[version_index]),
            entries,
            optional_features: vec![String::from("future-optional")],
            profile_identity: String::from("writer-generated"),
            required_features: feature_sets[feature_index]
                .iter()
                .map(|feature| String::from(*feature))
                .collect(),
        };
        let expected = reference_manifest(&value, &supported);
        match &expected {
            Ok(()) => saw_valid = true,
            Err(ProfileManifestError::DuplicateEntryPath { .. }) => {
                saw_duplicate_path = true;
            },
            Err(ProfileManifestError::EmptyMediaType { .. }) => {
                saw_empty_media_type = true;
            },
            Err(ProfileManifestError::InvalidEntryPath { reason, .. }) => {
                saw_invalid_path = true;
                match reason {
                    ProfileEntryPathError::BackslashSeparator => {
                        saw_manifest_backslash = true;
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
    assert!(saw_invalid_path);
    assert!(saw_manifest_backslash);
    assert!(saw_manifest_empty_segment);
    assert!(saw_manifest_reserved);
    assert!(saw_manifest_traversal);
    assert!(saw_manifest_unsupported_root);
    assert!(saw_unsupported_version);
    assert!(saw_unsupported_feature);
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
        r"x\y",
    ];
    let mut exercised = 0_usize;
    let mut saw_asset = false;
    let mut saw_section = false;
    let mut saw_backslash = false;
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
    assert_eq!(exercised, 162);
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
    }
    assert!(saw_length_mismatch);
    assert!(saw_digest_mismatch);
    assert!(saw_valid);
}
