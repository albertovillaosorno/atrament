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
    ProfileImportBudget, ProfileImportBudgetError, ProfileImportPreflightError,
    preflight_explicit_profile_import, validate_profile_import_budget,
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

const fn budget() -> ProfileImportBudget {
    ProfileImportBudget {
        maximum_declared_bytes: 128,
        maximum_entry_bytes: 64,
        maximum_entries: 4,
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
        budget(),
    )
    .expect("all structural preflight checks accept");
    assert!(std::ptr::eq(admitted.path(), &path));
    assert!(std::ptr::eq(admitted.inventory().manifest(), &parsed));
    assert_eq!(admitted.inventory().observed_paths(), OBSERVED);
    assert_eq!(admitted.archive_encoding().evidence(), canonical_encoding());
    assert_eq!(admitted.budget(), budget());
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
                budget(),
            ),
            Err(ProfileImportPreflightError::WrongPathPurpose),
        );
    }
    let path = selected(ExplicitFilePurpose::ImportProfile);
    assert!(matches!(
        preflight_explicit_profile_import(
            &path, &parsed, &["strokes"], OBSERVED,
            malformed_encoding,
            budget(),
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
                        budget(),
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
        budget(),
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
            budget(),
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
            budget(),
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

#[test]
fn resource_budget_admits_exact_boundaries_and_rejects_member_counts() {
    let parsed = manifest();
    assert_eq!(
        validate_profile_import_budget(&parsed, budget()),
        Ok(()),
    );
    let strict = ProfileImportBudget {
        maximum_declared_bytes: 38,
        maximum_entry_bytes: 29,
        maximum_entries: 2,
    };
    assert_eq!(validate_profile_import_budget(&parsed, strict), Ok(()));
    let count_limited = ProfileImportBudget {
        maximum_entries: 1,
        ..strict
    };
    assert_eq!(
        validate_profile_import_budget(&parsed, count_limited),
        Err(ProfileImportBudgetError::EntryCountExceeded {
            limit: 1,
            observed: 2,
        }),
    );
    let path = selected(ExplicitFilePurpose::ImportProfile);
    assert_eq!(
        preflight_explicit_profile_import(
            &path, &parsed, &["strokes"], OBSERVED,
            canonical_encoding(), count_limited,
        ),
        Err(ProfileImportPreflightError::Budget(
            ProfileImportBudgetError::EntryCountExceeded {
                limit: 1,
                observed: 2,
            },
        )),
    );
}

#[test]
fn resource_budget_rejects_individual_and_aggregate_bytes() {
    let parsed = manifest();
    let small_entry = ProfileImportBudget {
        maximum_entry_bytes: 28,
        ..budget()
    };
    assert_eq!(
        validate_profile_import_budget(&parsed, small_entry),
        Err(ProfileImportBudgetError::EntryBytesExceeded {
            declared: 29,
            index: 1,
            limit: 28,
        }),
    );
    let small_total = ProfileImportBudget {
        maximum_declared_bytes: 37,
        ..budget()
    };
    assert_eq!(
        validate_profile_import_budget(&parsed, small_total),
        Err(ProfileImportBudgetError::TotalBytesExceeded {
            declared: 38,
            limit: 37,
        }),
    );
    let path = selected(ExplicitFilePurpose::ImportProfile);
    assert_eq!(
        preflight_explicit_profile_import(
            &path, &parsed, &["strokes"], OBSERVED,
            canonical_encoding(), small_total,
        ),
        Err(ProfileImportPreflightError::Budget(
            ProfileImportBudgetError::TotalBytesExceeded {
                declared: 38,
                limit: 37,
            },
        )),
    );
}

#[test]
fn resource_budget_sum_overflow_fails_without_wraparound() {
    let mut parsed = manifest();
    parsed.entries[0].byte_length = u64::MAX;
    parsed.entries[1].byte_length = 1;
    let largest = ProfileImportBudget {
        maximum_declared_bytes: u64::MAX,
        maximum_entry_bytes: u64::MAX,
        maximum_entries: 2,
    };
    assert_eq!(
        validate_profile_import_budget(&parsed, largest),
        Err(ProfileImportBudgetError::TotalOverflow),
    );
    let path = selected(ExplicitFilePurpose::ImportProfile);
    assert_eq!(
        preflight_explicit_profile_import(
            &path, &parsed, &["strokes"], OBSERVED,
            canonical_encoding(), largest,
        ),
        Err(ProfileImportPreflightError::Budget(
            ProfileImportBudgetError::TotalOverflow,
        )),
    );
}

#[test]
fn resource_budget_allows_zero_entries_but_zero_bytes_fail_closed() {
    let mut parsed = manifest();
    parsed.entries.clear();
    let empty = ProfileImportBudget {
        maximum_declared_bytes: 0,
        maximum_entry_bytes: 0,
        maximum_entries: 0,
    };
    assert_eq!(validate_profile_import_budget(&parsed, empty), Ok(()));
    parsed.entries.push(ProfileManifestEntry {
        byte_length: 1,
        digest: Sha256Digest::from_bytes([1; 32]),
        media_type: String::from("application/json"),
        path: String::from("sections/empty.json"),
    });
    let one = ProfileImportBudget {
        maximum_entries: 1,
        ..empty
    };
    assert_eq!(
        validate_profile_import_budget(&parsed, one),
        Err(ProfileImportBudgetError::EntryBytesExceeded {
            declared: 1,
            index: 0,
            limit: 0,
        }),
    );
}

#[test]
fn resource_budget_checks_observed_archive_names_before_allocating_inventory() {
    let path = selected(ExplicitFilePurpose::ImportProfile);
    let parsed = manifest();
    let exact = ProfileImportBudget {
        maximum_entries: 2,
        ..budget()
    };
    let too_many = [
        PROFILE_MANIFEST_PATH,
        "assets/source.webp",
        "sections/strokes.json",
        "assets/undeclared.bin",
    ];
    assert_eq!(
        preflight_explicit_profile_import(
            &path, &parsed, &["strokes"], &too_many,
            canonical_encoding(), exact,
        ),
        Err(ProfileImportPreflightError::Budget(
            ProfileImportBudgetError::ObservedCountExceeded {
                limit: 3,
                observed: 4,
            },
        )),
    );
    assert!(preflight_explicit_profile_import(
        &path, &parsed, &["strokes"], OBSERVED,
        canonical_encoding(), exact,
    ).is_ok());
}
