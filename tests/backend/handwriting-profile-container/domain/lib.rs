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
    PROFILE_CONTAINER_VERSION, PROFILE_MANIFEST_PATH, ProfileEntryEvidence,
    ProfileEntryInventoryError, ProfileEntryKind, ProfileEntryPathError,
    ProfileEntryVerificationError, ProfileManifest, ProfileManifestEntry,
    ProfileManifestError, Sha256Digest, canonical_profile_archive_paths,
    canonical_profile_entry_order, profile_entry_kind, profile_section_entries,
    validate_profile_entry_inventory, validate_profile_manifest,
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
