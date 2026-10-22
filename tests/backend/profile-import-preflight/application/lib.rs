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
//   - Executable evidence for explicit profile Import prerequisites.
// - Must-Not:
//   - Access actual files, trust invented digest observations, or decode ZIP.
// - Allows:
//   - Inputs: Typed Import selections and synthetic parsed archive facts.
//   - Outputs: Exact typed errors, sealed views, and verification outcomes.
//   - Side effects: None outside test-process allocations.
// - Split-When:
//   - Archive parsing and real byte digests require a distinct fixture.
// - Merge-When:
//   - Real explicit Import integration replaces all these prerequisites.
// - Summary:
//   - Blocks inappropriate paths, unsafe archive encoding, and bad inventory.
// - Description:
//   - Covers exact entry evidence without treating it as a completed import.
// - Usage:
//   - Run through Cargo workspace integration tests.
// - Defaults:
//   - Structural admission never writes, reads, or changes session state.
//

//! Explicit, read-only portable profile Import preflight regression evidence.

use std::path::PathBuf;

use atrament_explicit_file_path_intent::{
    ExplicitFilePathIntent, ExplicitFilePathSource, ExplicitFilePurpose,
    ExplicitOverwriteIntent, classify_explicit_file_path_intent,
};
use atrament_handwriting_profile_container::{
    PROFILE_CONTAINER_VERSION, PROFILE_MANIFEST_PATH,
    ProfileArchiveEncodingError,
    ProfileArchiveEncodingEvidence, ProfileArchiveEntryEncoding,
    ProfileArchivePlatformExtras, ProfileEntryEvidence,
    ProfileEntryInventoryError, ProfileEntryVerificationError,
    ProfileManifest, ProfileManifestEntry, ProfileManifestError,
    ProfileZip64Requirement, ProfileZip64Use, Sha256Digest,
};
use atrament_profile_import_preflight::{
    ProfileImportPreflightError, preflight_explicit_profile_import,
};

fn selected(purpose: ExplicitFilePurpose) -> ExplicitFilePathIntent {
    classify_explicit_file_path_intent(
        purpose,
        ExplicitFilePathSource::CallerSelection,
        PathBuf::from("selected/profile.atrament"),
        ExplicitOverwriteIntent::PreserveExisting,
    )
    .expect("independent trusted file selection")
}

const fn canonical_encoding() -> ProfileArchiveEncodingEvidence {
    ProfileArchiveEncodingEvidence {
        entry_encoding: ProfileArchiveEntryEncoding::Stored,
        platform_extras: ProfileArchivePlatformExtras::Absent,
        zip64_requirement: ProfileZip64Requirement::Ordinary,
        zip64_use: ProfileZip64Use::Absent,
    }
}

fn manifest() -> ProfileManifest {
    ProfileManifest {
        container_version: String::from(PROFILE_CONTAINER_VERSION),
        entries: vec![
            ProfileManifestEntry {
                byte_length: 9,
                digest: Sha256Digest::from_bytes([11; 32]),
                media_type: String::from("image/webp"),
                path: String::from("assets/source.webp"),
            },
            ProfileManifestEntry {
                byte_length: 29,
                digest: Sha256Digest::from_bytes([17; 32]),
                media_type: String::from("application/json"),
                path: String::from("sections/strokes.json"),
            },
        ],
        optional_features: vec![String::from("future-optional")],
        profile_identity: String::from("selected-profile"),
        required_features: vec![String::from("strokes")],
    }
}

const OBSERVED: &[&str] = &[
    "sections/strokes.json", PROFILE_MANIFEST_PATH, "assets/source.webp",
];

#[test]
fn valid_explicit_import_seals_exact_entries_but_no_imported_profile() {
    let path = selected(ExplicitFilePurpose::ImportProfile);
    let parsed = manifest();
    let admitted = preflight_explicit_profile_import(
        &path, &parsed, &["strokes"], OBSERVED, canonical_encoding(),
    )
    .expect("all structural preflight checks accept");
    assert!(std::ptr::eq(admitted.path(), &path));
    assert!(std::ptr::eq(admitted.inventory().manifest(), &parsed));
    assert_eq!(admitted.inventory().observed_paths(), OBSERVED);
    assert_eq!(admitted.archive_encoding().evidence(), canonical_encoding());
    assert_eq!(
        admitted.inventory().manifest().optional_features,
        ["future-optional"],
    );
    let evidence = ProfileEntryEvidence {
        byte_length: 29,
        digest: Sha256Digest::from_bytes([17; 32]),
    };
    let entry = admitted
        .verify_entry("sections/strokes.json", evidence)
        .expect("digest and length match")
        .expect("entry exists");
    assert_eq!(entry.entry().path, "sections/strokes.json");
    assert_eq!(entry.evidence(), evidence);
    assert_eq!(
        admitted.verify_entry("sections/%73trokes.json", evidence),
        Ok(None),
        "near-match names never undergo percent decoding",
    );
    assert_eq!(
        admitted.verify_entry("sections/strokes.json", ProfileEntryEvidence {
            byte_length: 28,
            digest: Sha256Digest::from_bytes([18; 32]),
        }),
        Err(ProfileEntryVerificationError::ByteLengthMismatch {
            declared: 29,
            observed: 28,
        }),
    );
    assert_eq!(
        admitted.verify_entry("sections/strokes.json", ProfileEntryEvidence {
            byte_length: 29,
            digest: Sha256Digest::from_bytes([18; 32]),
        }),
        Err(ProfileEntryVerificationError::DigestMismatch),
    );
}

#[test]
fn only_profile_import_purpose_can_reach_archive_preflight() {
    let parsed = manifest();
    let malformed_encoding = ProfileArchiveEncodingEvidence {
        entry_encoding: ProfileArchiveEntryEncoding::Compressed,
        ..canonical_encoding()
    };
    for purpose in [
        ExplicitFilePurpose::ImportNotebook,
        ExplicitFilePurpose::ExportNotebook,
        ExplicitFilePurpose::ExportProfile,
    ] {
        let path = selected(purpose);
        assert_eq!(
            preflight_explicit_profile_import(
                &path, &parsed, &["strokes"], OBSERVED,
                malformed_encoding,
            ),
            Err(ProfileImportPreflightError::WrongPathPurpose),
        );
    }
    let path = selected(ExplicitFilePurpose::ImportProfile);
    assert!(matches!(
        preflight_explicit_profile_import(
            &path, &parsed, &["strokes"], OBSERVED,
            malformed_encoding,
        ),
        Err(ProfileImportPreflightError::ArchiveEncoding(
            ProfileArchiveEncodingError::CompressedEntry
        )),
    ));
}

#[test]
fn all_canonical_zip_encoding_cases_preserve_observation_failures() {
    let path = selected(ExplicitFilePurpose::ImportProfile);
    let parsed = manifest();
    let mut cases = 0usize;
    let mut admitted = 0usize;
    for entry_encoding in [
        ProfileArchiveEntryEncoding::Stored,
        ProfileArchiveEntryEncoding::Compressed,
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
                    let result = preflight_explicit_profile_import(
                        &path, &parsed, &["strokes"], OBSERVED, evidence,
                    );
                    let expected =
                        atrament_handwriting_profile_container::
                            validate_profile_archive_encoding(evidence);
                    match (result, expected) {
                        (Ok(admission), Ok(())) => {
                            admitted += 1;
                            assert_eq!(admission.archive_encoding().evidence(),
                                evidence);
                        },
                        (
                            Err(
                                ProfileImportPreflightError::ArchiveEncoding(a)
                            ),
                            Err(b),
                        ) => assert_eq!(a, b),
                        other => panic!("encoding disagreement: {other:?}"),
                    }
                    cases += 1;
                }
            }
        }
    }
    assert_eq!(cases, 16);
    assert_eq!(admitted, 2);
}

#[test]
fn manifest_rejection_precedes_observed_inventory_mismatch() {
    let path = selected(ExplicitFilePurpose::ImportProfile);
    let mut parsed = manifest();
    parsed.required_features = vec![String::from("unsupported")];
    let outcome = preflight_explicit_profile_import(
        &path, &parsed, &[], &["other.json"], canonical_encoding(),
    );
    assert_eq!(
        outcome,
        Err(ProfileImportPreflightError::Manifest(
            ProfileManifestError::UnsupportedRequiredFeature {
                feature: String::from("unsupported"),
            },
        )),
    );
    let parsed = manifest();
    assert_eq!(
        preflight_explicit_profile_import(
            &path, &parsed, &["strokes"], &["assets/source.webp"],
            canonical_encoding(),
        ),
        Err(ProfileImportPreflightError::EntryInventory(
            ProfileEntryInventoryError::MissingManifest,
        )),
    );
    assert_eq!(
        preflight_explicit_profile_import(
            &path, &parsed, &["strokes"],
            &[PROFILE_MANIFEST_PATH, "assets/source.webp"],
            canonical_encoding(),
        ),
        Err(ProfileImportPreflightError::EntryInventory(
            ProfileEntryInventoryError::MissingDeclaredEntry {
                path: String::from("sections/strokes.json"),
            },
        )),
    );
}

#[test]
fn notebook_model_and_clipboard_text_never_select_profile_import_paths() {
    for source in [
        ExplicitFilePathSource::ClipboardText,
        ExplicitFilePathSource::ModelResponseText,
        ExplicitFilePathSource::NotebookOrSourceText,
        ExplicitFilePathSource::DiagnosticText,
    ] {
        assert!(classify_explicit_file_path_intent(
            ExplicitFilePurpose::ImportProfile,
            source,
            PathBuf::from("untrusted.atrament"),
            ExplicitOverwriteIntent::PreserveExisting,
        ).is_err());
    }
}
