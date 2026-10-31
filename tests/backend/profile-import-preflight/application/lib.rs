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
use std::ptr;

use atrament_explicit_file_path_intent::{
    ExplicitFilePathIntent, ExplicitFilePathIntentError,
    ExplicitFilePathSource, ExplicitFilePurpose,
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
    preflight_explicit_profile_import,
    preflight_explicit_profile_import_sorted,
    validate_profile_import_budget,
};

// Test-only guards retain explicit failure assertions at fixture boundaries.
macro_rules! require_ok {
    ($expression:expr) => {{
        let result = $expression;
        assert!(
            result.is_ok(),
            "fixture must be admitted: {:?}", result.as_ref().err(),
        );
        let Ok(value) = result else { return; };
        value
    }};
}

macro_rules! require_some {
    ($expression:expr) => {{
        let selected = $expression;
        assert!(selected.is_some(), "fixture entry must be present");
        let Some(value) = selected else { return; };
        value
    }};
}

const OBSERVED: &[&str] = &[
    "sections/strokes.json", PROFILE_MANIFEST_PATH, "assets/source.webp",
];

fn selected(
    purpose: ExplicitFilePurpose,
) -> Result<ExplicitFilePathIntent, ExplicitFilePathIntentError> {
    classify_explicit_file_path_intent(
        purpose,
        ExplicitFilePathSource::CallerSelection,
        PathBuf::from("selected/profile.atrament"),
        ExplicitOverwriteIntent::PreserveExisting,
    )
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

#[test]
fn valid_explicit_import_seals_exact_entries_but_no_imported_profile() {
    let path = require_ok!(selected(ExplicitFilePurpose::ImportProfile));
    let parsed = manifest();
    let admitted = require_ok!(preflight_explicit_profile_import(
        &path, &parsed, &["strokes"], OBSERVED, canonical_encoding(),
        budget(),
    ));
    assert!(ptr::eq(admitted.path(), &raw const path));
    assert!(ptr::eq(admitted.inventory().manifest(), &raw const parsed));
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
    let entry = require_ok!(admitted.verify_entry(
        "sections/strokes.json", evidence,
    ));
    let verified_entry = require_some!(entry);
    assert_eq!(verified_entry.entry().path, "sections/strokes.json");
    assert_eq!(verified_entry.evidence(), evidence);
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
        let path = require_ok!(selected(purpose));
        assert_eq!(
            preflight_explicit_profile_import(
                &path, &parsed, &["strokes"], OBSERVED,
                malformed_encoding,
                budget(),
            ),
            Err(ProfileImportPreflightError::WrongPathPurpose),
        );
    }
    let path = require_ok!(selected(ExplicitFilePurpose::ImportProfile));
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
    let path = require_ok!(selected(ExplicitFilePurpose::ImportProfile));
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
                                ProfileImportPreflightError::ArchiveEncoding(
                                    actual_error,
                                )
                            ),
                            Err(expected_error),
                        ) => assert_eq!(actual_error, expected_error),
                        (actual, expected_evidence) => assert_eq!(
                            actual.map(|_| ()),
                            expected_evidence.map_err(
                                ProfileImportPreflightError::ArchiveEncoding,
                            ),
                            "encoding admission must match exact evidence",
                        ),
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
    let path = require_ok!(selected(ExplicitFilePurpose::ImportProfile));
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
    let valid_manifest = manifest();
    assert_eq!(
        preflight_explicit_profile_import(
            &path, &valid_manifest, &["strokes"], &["assets/source.webp"],
            canonical_encoding(),
            budget(),
        ),
        Err(ProfileImportPreflightError::EntryInventory(
            ProfileEntryInventoryError::MissingManifest,
        )),
    );
    assert_eq!(
        preflight_explicit_profile_import(
            &path, &valid_manifest, &["strokes"],
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
    let path = require_ok!(selected(ExplicitFilePurpose::ImportProfile));
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
    let path = require_ok!(selected(ExplicitFilePurpose::ImportProfile));
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
    require_some!(parsed.entries.get_mut(0)).byte_length = u64::MAX;
    require_some!(parsed.entries.get_mut(1)).byte_length = 1;
    let largest = ProfileImportBudget {
        maximum_declared_bytes: u64::MAX,
        maximum_entry_bytes: u64::MAX,
        maximum_entries: 2,
    };
    assert_eq!(
        validate_profile_import_budget(&parsed, largest),
        Err(ProfileImportBudgetError::TotalOverflow),
    );
    let path = require_ok!(selected(ExplicitFilePurpose::ImportProfile));
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
    let path = require_ok!(selected(ExplicitFilePurpose::ImportProfile));
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

#[test]
fn sorted_import_preflight_preserves_early_gates_and_exact_admission() {
    let parsed = manifest();
    let sorted = [
        "assets/source.webp", "manifest.json", "sections/strokes.json",
    ];
    let selected_path = require_ok!(selected(
        ExplicitFilePurpose::ImportProfile,
    ));
    let original = require_ok!(preflight_explicit_profile_import(
        &selected_path, &parsed, &["strokes"], &sorted,
        canonical_encoding(), budget(),
    ));
    let optimized = require_ok!(preflight_explicit_profile_import_sorted(
        &selected_path, &parsed, &["strokes"], &sorted,
        canonical_encoding(), budget(),
    ));
    assert_eq!(optimized, original);
    assert_eq!(optimized.inventory().observed_paths(), sorted);
    assert!(ptr::eq(optimized.path(), &raw const selected_path));
    assert!(ptr::eq(optimized.inventory().manifest(), &raw const parsed));
    assert_eq!(preflight_explicit_profile_import_sorted(
        &selected_path, &parsed, &["strokes"], OBSERVED,
        canonical_encoding(), budget(),
    ), Err(ProfileImportPreflightError::EntryInventory(
        ProfileEntryInventoryError::NonCanonicalObservedEntryOrder {
            earlier: String::from("sections/strokes.json"),
            later: String::from("manifest.json"),
        },
    )));
    assert!(preflight_explicit_profile_import(
        &selected_path, &parsed, &["strokes"], OBSERVED,
        canonical_encoding(), budget(),
    ).is_ok(), "general API retains arbitrary-order support");
    let wrong = require_ok!(selected(ExplicitFilePurpose::ExportProfile));
    assert_eq!(preflight_explicit_profile_import_sorted(
        &wrong, &parsed, &["strokes"], OBSERVED,
        canonical_encoding(), budget(),
    ), Err(ProfileImportPreflightError::WrongPathPurpose));
    let unsupported = ProfileArchiveEncodingEvidence {
        entry_encoding: ProfileArchiveEntryEncoding::Compressed,
        ..canonical_encoding()
    };
    assert_eq!(preflight_explicit_profile_import_sorted(
        &selected_path, &parsed, &["strokes"], OBSERVED,
        unsupported, budget(),
    ), Err(ProfileImportPreflightError::ArchiveEncoding(
        ProfileArchiveEncodingError::CompressedEntry,
    )));
    assert_eq!(preflight_explicit_profile_import_sorted(
        &selected_path, &parsed, &["strokes"], OBSERVED,
        canonical_encoding(), ProfileImportBudget {
            maximum_entries: 1,
            ..budget()
        },
    ), Err(ProfileImportPreflightError::Budget(
        ProfileImportBudgetError::EntryCountExceeded {
            limit: 1,
            observed: 2,
        },
    )));
}

#[test]
fn sorted_and_general_import_preflight_share_all_early_gate_combinations() {
    let parsed = manifest();
    let names = [
        "assets/source.webp", "manifest.json", "sections/strokes.json",
    ];
    let mut cases = 0usize;
    let mut accepted = 0usize;
    let mut rejected = 0usize;
    for purpose in [
        ExplicitFilePurpose::ImportProfile,
        ExplicitFilePurpose::ExportProfile,
        ExplicitFilePurpose::ImportNotebook,
        ExplicitFilePurpose::ExportNotebook,
    ] {
        let selection = require_ok!(selected(purpose));
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
                        let encoding = ProfileArchiveEncodingEvidence {
                            entry_encoding, platform_extras,
                            zip64_requirement, zip64_use,
                        };
                        for restrictive_budget in [false, true] {
                            let budget = if restrictive_budget {
                                ProfileImportBudget {
                                    maximum_entries: 1,
                                    ..budget()
                                }
                            } else {
                                budget()
                            };
                            let general = preflight_explicit_profile_import(
                                &selection, &parsed, &["strokes"], &names,
                                encoding, budget,
                            );
                            let sorted =
                                preflight_explicit_profile_import_sorted(
                                    &selection, &parsed, &["strokes"],
                                    &names, encoding, budget,
                                );
                            assert_eq!(sorted, general,
                                "same gate result case={cases}");
                            if general.is_ok() {
                                accepted += 1;
                            } else {
                                rejected += 1;
                            }
                            cases += 1;
                        }
                    }
                }
            }
        }
    }
    assert_eq!(cases, 128);
    assert!(accepted > 0);
    assert!(rejected > 0);
    assert_eq!(accepted + rejected, cases);
}

#[test]
fn sealed_import_preflight_debug_hides_private_nested_evidence() {
    let path = require_ok!(classify_explicit_file_path_intent(
        ExplicitFilePurpose::ImportProfile,
        ExplicitFilePathSource::CallerSelection,
        PathBuf::from("source/private-source-8502.atrament"),
        ExplicitOverwriteIntent::PreserveExisting,
    ));
    let mut parsed = manifest();
    parsed.profile_identity = String::from("private-identity-6419");
    parsed.required_features = vec![String::from("private-required-2701")];
    parsed.optional_features = vec![String::from("private-optional-9163")];
    require_some!(parsed.entries.get_mut(0)).path =
        String::from("assets/private-asset-7225.webp");
    require_some!(parsed.entries.get_mut(1)).path =
        String::from("sections/private-stroke-4568.json");
    let observed = [
        "assets/private-asset-7225.webp",
        PROFILE_MANIFEST_PATH,
        "sections/private-stroke-4568.json",
    ];
    let preflight = require_ok!(preflight_explicit_profile_import_sorted(
        &path,
        &parsed,
        &["private-required-2701"],
        &observed,
        canonical_encoding(),
        budget(),
    ));
    for printed in [
        format!("{preflight:?}"),
        format!("{preflight:#?}"),
        format!("{:?}", Ok::<_, ProfileImportPreflightError>(preflight)),
    ] {
        assert!(printed.contains("ProfileImportPreflight"));
        for private in [
            "private-source-8502",
            "private-identity-6419",
            "private-required-2701",
            "private-optional-9163",
            "private-asset-7225",
            "private-stroke-4568",
        ] {
            assert!(!printed.contains(private));
        }
    }
    assert_eq!(preflight.path().path(), path.path());
    assert_eq!(preflight.inventory().manifest(), &parsed);
    assert_eq!(preflight.inventory().observed_paths(), observed);
}
