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
//   - End-to-end profile Export-byte to Import-byte verification fixtures.
// - Must-Not:
//   - Open selected files, choose Export intent, or mutate session profiles.
// - Allows:
//   - Inputs: Typed manifests and explicitly supplied in-memory member bytes.
//   - Outputs: Canonical ZIP identity, Import parity, typed negative evidence.
//   - Side effects: Test-owned bytes only.
// - Split-When:
//   - Physical Export file commit adapter gets separate integration coverage.
// - Merge-When:
//   - Trusted selected-source Export and Import share acceptance tests.
// - Summary:
//   - Reject mismatched payloads before any complete artifact can be returned.
// - Description:
//   - Manifest, ZIP, CRC, SHA-256 and ordered Import paths agree independently.
// - Usage:
//   - Run cargo test -p atrament_profile_export_assembly.
// - Defaults:
//   - No filesystem I/O or external Export authority is exercised.
//

//! Complete canonical profile artifact byte boundaries and refusals.

use std::path::PathBuf;

use atrament_explicit_file_path_intent::{
    ExplicitFilePathSource, ExplicitFilePurpose, ExplicitOverwriteIntent,
    classify_explicit_file_path_intent,
};
use atrament_handwriting_profile_container::{
    PROFILE_CONTAINER_VERSION, ProfileEntryVerificationError,
    ProfileFeatureList, ProfileManifest, ProfileManifestEntry,
    ProfileManifestError,
};
use atrament_profile_entry_sha256::sha256_profile_bytes;
use atrament_profile_export_assembly::{
    ProfileExportAssemblyError as Error, ProfileExportAssemblyLimits,
    ProfileExportPayload, assemble_profile_export_artifact,
    planned_profile_export_artifact_bytes, write_profile_export_artifact_into,
};
use atrament_profile_import_preflight::ProfileImportBudget;
use atrament_profile_import_verification::verify_attached_profile_import;
use atrament_profile_json_syntax::{
    ProfileJsonSyntaxError, ProfileJsonSyntaxLimits,
};
use atrament_profile_manifest_json_codec::{
    ProfileManifestJsonLimits, decode_attached_profile_manifest_json,
};
use atrament_profile_manifest_json_writer::{
    ProfileManifestJsonWriteError, ProfileManifestJsonWriteLimits,
};
use atrament_profile_stored_zip_writer::{
    ProfileStoredZipWriteError, ProfileStoredZipWriteLimits,
};
use atrament_profile_zip_reader::{
    ProfileZipReadLimits, read_stored_profile_zip,
};

// Check every successful setup result without bypassing the test failure.
macro_rules! require_ok {
    ($expression:expr) => {{
        let result = $expression;
        assert!(
            result.is_ok(),
            "expected complete artifact: {:?}",
            result.as_ref().err(),
        );
        let Ok(value) = result else {
            return;
        };
        value
    }};
}

const fn limits() -> ProfileExportAssemblyLimits {
    ProfileExportAssemblyLimits {
        manifest_json: ProfileManifestJsonWriteLimits {
            maximum_bytes: 8192,
            maximum_entries: 8,
            maximum_features: 8,
            maximum_string_bytes: 256,
        },
        section_json: ProfileJsonSyntaxLimits {
            maximum_bytes: 4096,
            maximum_nesting: 32,
            maximum_object_members: 64,
            maximum_values: 1024,
        },
        zip: ProfileStoredZipWriteLimits {
            maximum_archive_bytes: 16384,
            maximum_entries: 9,
            maximum_entry_bytes: 8192,
            maximum_total_bytes: 12000,
        },
    }
}

fn entry(path: &str, bytes: &[u8]) -> ProfileManifestEntry {
    ProfileManifestEntry {
        byte_length: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
        digest: sha256_profile_bytes(bytes),
        media_type: "application/json".into(),
        path: path.into(),
    }
}

fn manifest() -> ProfileManifest {
    ProfileManifest {
        container_version: PROFILE_CONTAINER_VERSION.into(),
        entries: vec![],
        optional_features: vec!["future".into()],
        profile_identity: "writer-\u{1f58b}".into(),
        required_features: vec!["strokes".into()],
    }
}

#[test]
fn empty_profile_is_real_zip_with_exact_root_identity() {
    let artifact = require_ok!(assemble_profile_export_artifact(
        &manifest(), &[], &["strokes"], limits(),
    ));
    let archive = require_ok!(read_stored_profile_zip(
        artifact.bytes(),
        ProfileZipReadLimits {
            maximum_archive_bytes: 16384,
            maximum_entries: 1,
            maximum_entry_bytes: 8192,
            maximum_total_bytes: 12000,
        },
    ));
    assert_eq!(archive.observed_paths(), ["manifest.json"]);
    assert_eq!(archive.source_sha256(), artifact.archive_sha256());
    assert_eq!(archive.source_bytes(), artifact.bytes());
    let root = require_ok!(archive.admitted_root_json(ProfileJsonSyntaxLimits {
        maximum_bytes: 8192,
        maximum_nesting: 16,
        maximum_object_members: 32,
        maximum_values: 128,
    }));
    assert_eq!(root.source_sha256(), artifact.root_sha256());
}

#[test]
fn debug_of_complete_export_and_borrowed_payload_redacts_private_bytes() {
    let section = br#"{"private":"private-section-marker"}"#;
    let mut profile = manifest();
    profile.entries = vec![entry("sections/private.json", section)];
    let payload = ProfileExportPayload {
        bytes: section,
        path: "sections/private.json",
    };
    let artifact = require_ok!(assemble_profile_export_artifact(
        &profile, &[payload], &["strokes"], limits(),
    ));
    let archived_bytes = format!("{:?}", artifact.bytes());
    let payload_bytes = format!("{section:?}");
    let debug = format!("{artifact:?} {payload:?}");
    assert!(debug.contains("ProfileExportArtifact"));
    assert!(debug.contains("ProfileExportPayload"));
    assert!(!debug.contains("bytes:"));
    assert!(!debug.contains(&archived_bytes));
    assert!(!debug.contains(&payload_bytes));
    assert!(!debug.contains("sections/private.json"));
    assert!(!debug.contains("private-section-marker"));
}

#[test]
fn complete_archive_is_order_independent_and_import_verifiable() {
    let asset = b"IMAGE";
    let strokes = b"{\"ink\":true}";
    let mut first = manifest();
    first.entries = vec![
        entry("sections/strokes.json", strokes),
        entry("assets/ink.bin", asset),
    ];
    let payloads = [
        ProfileExportPayload {
            bytes: strokes, path: "sections/strokes.json",
        },
        ProfileExportPayload {
            bytes: asset, path: "assets/ink.bin",
        },
    ];
    let first_artifact = require_ok!(assemble_profile_export_artifact(
        &first, &payloads, &["strokes"], limits(),
    ));
    first.entries.reverse();
    let second_artifact = require_ok!(assemble_profile_export_artifact(
        &first, &payloads.into_iter().rev().collect::<Vec<_>>(),
        &["strokes"], limits(),
    ));
    assert_eq!(first_artifact.bytes(), second_artifact.bytes());
    assert_eq!(
        first_artifact.archive_sha256(), second_artifact.archive_sha256(),
    );
    assert_eq!(first_artifact.root_sha256(), second_artifact.root_sha256());

    let archive = require_ok!(read_stored_profile_zip(
        first_artifact.bytes(),
        ProfileZipReadLimits {
        maximum_archive_bytes: first_artifact.bytes().len(),
        maximum_entries: 3,
        maximum_entry_bytes: 8192,
        maximum_total_bytes: 12000,
    }));
    let names = archive.observed_paths();
    assert_eq!(names, [
        "assets/ink.bin", "manifest.json", "sections/strokes.json",
    ]);
    let root = require_ok!(archive.admitted_root_json(ProfileJsonSyntaxLimits {
        maximum_bytes: 8192,
        maximum_nesting: 16,
        maximum_object_members: 32,
        maximum_values: 128,
    }));
    let attached = require_ok!(decode_attached_profile_manifest_json(
        &root,
        ProfileManifestJsonLimits {
            maximum_entries: 8,
            maximum_features: 8,
            maximum_string_bytes: 256,
        },
    ));
    let path = require_ok!(classify_explicit_file_path_intent(
        ExplicitFilePurpose::ImportProfile,
        ExplicitFilePathSource::CallerSelection,
        PathBuf::from("profile.atrament"),
        ExplicitOverwriteIntent::PreserveExisting,
    ));
    let mut observed = [""; 3];
    let verified = require_ok!(verify_attached_profile_import(
        &path, &attached, &["strokes"],
        ProfileImportBudget {
            maximum_declared_bytes: 64,
            maximum_entries: 2,
            maximum_entry_bytes: 64,
        },
        &mut observed,
    ));
    assert_eq!(verified.manifest(), &first);
    assert_eq!(verified.root_sha256(), first_artifact.root_sha256());
    assert_eq!(verified.archive_sha256(), first_artifact.archive_sha256());
    assert_eq!(
        verified.verified_entry_bytes("assets/ink.bin"), Some(&asset[..]),
    );
    assert_eq!(
        verified.verified_entry_bytes("sections/strokes.json"),
        Some(&strokes[..]),
    );
}

#[test]
fn inventory_mismatch_refuses_before_digest_or_zip() {
    let mut declared = manifest();
    declared.entries = vec![entry("assets/a.bin", b"A")];
    assert!(matches!(
        assemble_profile_export_artifact(
            &declared, &[], &["strokes"], limits(),
        ),
        Err(Error::PayloadCountMismatch {
            declared: 1, supplied: 0,
        }),
    ));
    let unexpected = [ProfileExportPayload {
        bytes: b"A", path: "assets/b.bin",
    }];
    assert_eq!(
        assemble_profile_export_artifact(
            &declared, &unexpected, &["strokes"], limits(),
        ).err(),
        Some(Error::PayloadPathMismatch { index: 0 }),
    );
    let duplicate = [
        ProfileExportPayload { bytes: b"A", path: "assets/a.bin" },
        ProfileExportPayload { bytes: b"A", path: "assets/a.bin" },
    ];
    assert_eq!(
        assemble_profile_export_artifact(
            &declared, &duplicate, &["strokes"], limits(),
        ).err(),
        Some(Error::PayloadCountMismatch {
            declared: 1, supplied: 2,
        }),
    );

    declared.entries.push(entry("sections/b.bin", b"B"));
    assert_eq!(
        assemble_profile_export_artifact(
            &declared, &duplicate, &["strokes"], limits(),
        ).err(),
        Some(Error::PayloadPathMismatch { index: 1 }),
    );
}

#[test]
fn section_media_type_mismatch_refuses_all_writers_before_output() {
    let json = br#"{"ink":true}"#;
    let opaque = b"\xff\x00bytes";
    let mut profile = manifest();
    profile.entries = vec![
        entry("sections/notes.json", json),
        entry("assets/image.bin", opaque),
    ];
    // Opaque asset MIME metadata remains permitted, but JSON sections
    // must match the exact format admitted by selected-file Import.
    assert_eq!(profile.entries.len(), 2);
    let Some(asset_entry) = profile.entries.get_mut(1) else {
        return;
    };
    asset_entry.media_type = "image/png".into();
    let payloads = [
        ProfileExportPayload {
            path: "sections/notes.json", bytes: json,
        },
        ProfileExportPayload {
            path: "assets/image.bin", bytes: opaque,
        },
    ];
    assert!(assemble_profile_export_artifact(
        &profile, &payloads, &["strokes"], limits(),
    ).is_ok(), "opaque asset type is not interpreted");

    for media_type in [
        "text/plain", "image/png", "application/json; charset=utf-8",
    ] {
        let Some(section_entry) = profile.entries.get_mut(0) else {
            return;
        };
        section_entry.media_type = media_type.into();
        let expected = Error::SectionMediaType { index: 1 };
        assert_eq!(
            assemble_profile_export_artifact(
                &profile, &payloads, &["strokes"], limits(),
            ).err(),
            Some(expected.clone()),
        );
        assert_eq!(
            planned_profile_export_artifact_bytes(
                &profile, &payloads, &["strokes"], limits(),
            ).err(),
            Some(expected.clone()),
        );
        let mut destination = [0xa9u8; 512];
        assert_eq!(
            write_profile_export_artifact_into(
                &profile, &payloads, &["strokes"], limits(),
                &mut destination,
            ).err(),
            Some(expected),
        );
        assert_eq!(destination, [0xa9; 512]);
    }
}

#[test]
fn section_syntax_and_budget_refuse_all_export_paths_without_output() {
    let asset = b"\xff\x00sample";
    let mut buffer = [0xb7u8; 512];
    for (section, reason) in [
        (&b"[]"[..], ProfileJsonSyntaxError::RootNotObject),
        (&b"{"[..], ProfileJsonSyntaxError::Malformed),
        (&b"\xff"[..], ProfileJsonSyntaxError::InvalidUtf8),
        (
            &br#"{"a":1,"a":2}"#[..],
            ProfileJsonSyntaxError::DuplicateObjectKey,
        ),
    ] {
        let mut profile = manifest();
        profile.entries = vec![
            entry("sections/a.json", b"{}"),
            entry("assets/raw.bin", asset),
            entry("sections/z.json", section),
        ];
        let payloads = [
            ProfileExportPayload {
                path: "sections/z.json", bytes: section,
            },
            ProfileExportPayload {
                path: "assets/raw.bin", bytes: asset,
            },
            ProfileExportPayload {
                path: "sections/a.json", bytes: b"{}",
            },
        ];
        let expected = Error::SectionJson { index: 2, reason };
        assert_eq!(
            assemble_profile_export_artifact(
                &profile, &payloads, &["strokes"], limits(),
            ).err(),
            Some(expected.clone()),
        );
        assert_eq!(
            planned_profile_export_artifact_bytes(
                &profile, &payloads, &["strokes"], limits(),
            ).err(),
            Some(expected.clone()),
        );
        assert_eq!(
            write_profile_export_artifact_into(
                &profile, &payloads, &["strokes"], limits(), &mut buffer,
            ).err(),
            Some(expected),
        );
        assert_eq!(buffer, [0xb7; 512]);
    }

    let section = br#"{"ink":1}"#;
    let mut profile = manifest();
    profile.entries = vec![entry("sections/ink.json", section)];
    let payloads = [ProfileExportPayload {
        path: "sections/ink.json", bytes: section,
    }];
    let mut budget = limits();
    budget.section_json.maximum_bytes = section.len() - 1;
    let expected = Error::SectionJson {
        index: 0,
        reason: ProfileJsonSyntaxError::MaximumBytesExceeded,
    };
    assert_eq!(
        assemble_profile_export_artifact(
            &profile, &payloads, &["strokes"], budget,
        ).err(),
        Some(expected.clone()),
    );
    assert_eq!(
        planned_profile_export_artifact_bytes(
            &profile, &payloads, &["strokes"], budget,
        ).err(),
        Some(expected.clone()),
    );
    assert_eq!(
        write_profile_export_artifact_into(
            &profile, &payloads, &["strokes"], budget, &mut buffer,
        ).err(),
        Some(expected),
    );
    assert_eq!(buffer, [0xb7; 512]);
}

#[test]
fn late_member_corruption_rejects_entire_archive() {
    let mut declared = manifest();
    declared.entries = vec![
        entry("assets/a.bin", b"aa"),
        entry("sections/z.bin", b"{}"),
    ];
    let clean = [
        ProfileExportPayload { path: "assets/a.bin", bytes: b"aa" },
        ProfileExportPayload { path: "sections/z.bin", bytes: b"{}" },
    ];
    let mut bytes = clean;
    bytes[1].bytes = b"zy";
    assert_eq!(
        assemble_profile_export_artifact(
            &declared, &bytes, &["strokes"], limits(),
        ).err(),
        Some(Error::PayloadVerification {
            index: 1,
            reason: ProfileEntryVerificationError::DigestMismatch,
        }),
    );
    bytes[1].bytes = b"long";
    assert_eq!(
        assemble_profile_export_artifact(
            &declared, &bytes, &["strokes"], limits(),
        ).err(),
        Some(Error::PayloadVerification {
            index: 1,
            reason: ProfileEntryVerificationError::ByteLengthMismatch {
                declared: 2, observed: 4,
            },
        }),
    );
    bytes[1] = clean[1];
    bytes[0].bytes = b"ab";
    assert_eq!(
        assemble_profile_export_artifact(
            &declared, &bytes, &["strokes"], limits(),
        ).err(),
        Some(Error::PayloadVerification {
            index: 0,
            reason: ProfileEntryVerificationError::DigestMismatch,
        }),
    );
    assert!(assemble_profile_export_artifact(
        &declared, &clean, &["strokes"], limits(),
    ).is_ok());
}

#[test]
fn repeated_feature_identifiers_refuse_complete_export_without_output() {
    for list in [ProfileFeatureList::Required, ProfileFeatureList::Optional] {
        let mut profile = manifest();
        let feature = match list {
            ProfileFeatureList::Required => {
                profile.required_features.push("strokes".into());
                "strokes"
            }
            ProfileFeatureList::Optional => {
                profile.optional_features.push("future".into());
                "future"
            }
        };
        let expected = Error::ManifestJson(
            ProfileManifestJsonWriteError::Manifest(
                ProfileManifestError::DuplicateFeatureIdentifier {
                    feature: feature.into(),
                    list,
                },
            ),
        );
        assert_eq!(
            assemble_profile_export_artifact(
                &profile, &[], &["strokes"], limits(),
            ).err(),
            Some(expected.clone()),
        );
        assert_eq!(
            planned_profile_export_artifact_bytes(
                &profile, &[], &["strokes"], limits(),
            ).err(),
            Some(expected.clone()),
        );
        let mut destination = [0xa3u8; 512];
        assert_eq!(
            write_profile_export_artifact_into(
                &profile, &[], &["strokes"], limits(),
                &mut destination,
            ).err(),
            Some(expected),
        );
        assert_eq!(destination, [0xa3; 512]);
    }
}

#[test]
fn manifest_and_zip_budgets_fail_without_artifact() {
    let profile = manifest();
    let mut budget = limits();
    budget.manifest_json.maximum_bytes = 0;
    assert_eq!(
        assemble_profile_export_artifact(
            &profile, &[], &["strokes"], budget,
        ).err(),
        Some(Error::ManifestJson(
            ProfileManifestJsonWriteError::ManifestTooLarge,
        )),
    );
    budget = limits();
    budget.zip.maximum_archive_bytes = 1;
    assert_eq!(
        assemble_profile_export_artifact(
            &profile, &[], &["strokes"], budget,
        ).err(),
        Some(Error::Zip(ProfileStoredZipWriteError::ArchiveTooLarge)),
    );
    budget = limits();
    budget.zip.maximum_entries = 0;
    assert_eq!(
        assemble_profile_export_artifact(
            &profile, &[], &["strokes"], budget,
        ).err(),
        Some(Error::Zip(ProfileStoredZipWriteError::ResourceLimit)),
    );
}

#[test]
fn caller_owned_archive_is_atomic_and_matches_exact_planned_size() {
    let mut profile = manifest();
    profile.entries = vec![
        entry("sections/\u{f1}.json", b"{\"ink\":1}"),
        entry("assets/blank.bin", b""),
    ];
    let payloads = [
        ProfileExportPayload { path: "assets/blank.bin", bytes: b"" },
        ProfileExportPayload {
            path: "sections/\u{f1}.json", bytes: b"{\"ink\":1}",
        },
    ];
    let expected = require_ok!(assemble_profile_export_artifact(
        &profile, &payloads, &["strokes"], limits(),
    ));
    let required = require_ok!(planned_profile_export_artifact_bytes(
        &profile, &payloads, &["strokes"], limits(),
    ));
    assert_eq!(required, expected.bytes().len());
    for capacity in [0, 1, required - 1] {
        let mut destination = vec![0xa5u8; capacity];
        assert_eq!(
            write_profile_export_artifact_into(
                &profile, &payloads, &["strokes"],
                limits(), &mut destination,
            ).err(),
            Some(Error::Zip(ProfileStoredZipWriteError::BufferTooSmall {
                required,
            })),
        );
        assert!(destination.iter().all(|&byte| byte == 0xa5));
    }
    for capacity in [required, required + 1, required + 20] {
        let mut destination = vec![0xa5u8; capacity];
        let evidence = require_ok!(write_profile_export_artifact_into(
            &profile, &payloads, &["strokes"],
            limits(), &mut destination,
        ));
        assert_eq!(evidence.bytes_written, required);
        assert_eq!(evidence.root_sha256, expected.root_sha256());
        assert_eq!(evidence.archive_sha256, expected.archive_sha256());
        assert_eq!(destination.get(..required), Some(expected.bytes()));
        assert!(destination.get(required..).is_some_and(|tail| {
            tail.iter().all(|&byte| byte == 0xa5)
        }));
    }
}

#[test]
fn caller_owned_output_rejects_bad_payload_before_mutating_any_slot() {
    let mut profile = manifest();
    profile.entries = vec![entry("assets/ink.bin", b"correct")];
    let bad = [ProfileExportPayload {
        path: "assets/ink.bin", bytes: b"corrupt",
    }];
    let mut destination = [0xabu8; 256];
    let expected = Error::PayloadVerification {
        index: 0,
        reason: ProfileEntryVerificationError::DigestMismatch,
    };
    assert_eq!(
        write_profile_export_artifact_into(
            &profile, &bad, &["strokes"],
            limits(), &mut destination,
        ).err(),
        Some(expected.clone()),
    );
    assert_eq!(
        planned_profile_export_artifact_bytes(
            &profile, &bad, &["strokes"], limits(),
        ).err(),
        Some(expected),
    );
    assert_eq!(destination, [0xab; 256]);
    let good = [ProfileExportPayload {
        path: "assets/ink.bin", bytes: b"correct",
    }];
    let mut budget = limits();
    budget.zip.maximum_archive_bytes = 1;
    assert_eq!(
        write_profile_export_artifact_into(
            &profile, &good, &["strokes"],
            budget, &mut destination,
        ).err(),
        Some(Error::Zip(ProfileStoredZipWriteError::ArchiveTooLarge)),
    );
    assert_eq!(destination, [0xab; 256]);
    budget = limits();
    budget.manifest_json.maximum_bytes = 1;
    assert_eq!(
        write_profile_export_artifact_into(
            &profile, &good, &["strokes"],
            budget, &mut destination,
        ).err(),
        Some(Error::ManifestJson(
            ProfileManifestJsonWriteError::ManifestTooLarge,
        )),
    );
    assert_eq!(destination, [0xab; 256]);
}

#[test]
fn zip_preflight_refuses_unaffordable_input_before_hash_comparison() {
    let mut profile = manifest();
    profile.entries.push(entry("assets/ink.bin", b"correct"));
    let bad = [ProfileExportPayload {
        path: "assets/ink.bin", bytes: b"corrupt",
    }];
    let mut budget = limits();
    budget.zip.maximum_entries = 1;
    let refusal = Error::Zip(ProfileStoredZipWriteError::ResourceLimit);
    assert_eq!(
        assemble_profile_export_artifact(
            &profile, &bad, &["strokes"], budget,
        ).err(),
        Some(refusal.clone()),
    );
    assert_eq!(
        planned_profile_export_artifact_bytes(
            &profile, &bad, &["strokes"], budget,
        ).err(),
        Some(refusal.clone()),
    );
    let mut destination = [0xa7u8; 512];
    assert_eq!(
        write_profile_export_artifact_into(
            &profile, &bad, &["strokes"], budget, &mut destination,
        ).err(),
        Some(refusal.clone()),
    );
    assert_eq!(destination, [0xa7; 512]);
    budget = limits();
    budget.zip.maximum_total_bytes = 1;
    assert_eq!(
        assemble_profile_export_artifact(
            &profile, &bad, &["strokes"], budget,
        ).err(),
        Some(refusal.clone()),
    );
    assert_eq!(
        planned_profile_export_artifact_bytes(
            &profile, &bad, &["strokes"], budget,
        ).err(),
        Some(refusal),
    );
    assert_eq!(
        write_profile_export_artifact_into(
            &profile, &bad, &["strokes"],
            limits(), &mut destination,
        ).err(),
        Some(Error::PayloadVerification {
            index: 0,
            reason: ProfileEntryVerificationError::DigestMismatch,
        }),
    );
    assert_eq!(destination, [0xa7; 512]);
}

fn generated_payload(
    case: usize,
    index: usize,
    state: u64,
) -> (String, Vec<u8>) {
    let name = if state & (1 << index.saturating_add(12)) == 0 {
        format!("assets/{index:02}.bin")
    } else {
        format!("sections/{index:02}-\u{f1}.json")
    };
    let data = if name.starts_with("sections/") {
        format!(r#"{{"case":{case},"index":{index}}}"#).into_bytes()
    } else {
        let byte = u8::try_from(index).unwrap_or(u8::MAX);
        vec![byte; index.saturating_add(case).rem_euclid(13)]
    };
    (name, data)
}

#[test]
fn generated_profile_archives_are_deterministic_and_sha_bound() {
    let mut state = 0x8ab5_41c0_9321_77b5u64;
    for case in 0..256 {
        state = state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
        let mut profile = manifest();
        let count = require_ok!(usize::try_from((state >> 4).rem_euclid(5)));
        assert!(count <= 4, "generated entries remain bounded");
        let mut values = Vec::new();
        for index in 0..count {
            let (name, data) = generated_payload(case, index, state);
            profile.entries.push(entry(&name, &data));
            values.push((name, data));
        }
        let mut payloads = values.iter().map(|(path, bytes)| {
            ProfileExportPayload { path, bytes }
        }).collect::<Vec<_>>();
        let expected = require_ok!(assemble_profile_export_artifact(
            &profile, &payloads, &["strokes"], limits(),
        ));
        profile.entries.reverse();
        payloads.reverse();
        let actual = require_ok!(assemble_profile_export_artifact(
            &profile, &payloads, &["strokes"], limits(),
        ));
        assert_eq!(actual.bytes(), expected.bytes(), "case {case}");
        assert_eq!(actual.root_sha256(), expected.root_sha256());
        assert_eq!(actual.archive_sha256(), expected.archive_sha256());
        let length = require_ok!(planned_profile_export_artifact_bytes(
            &profile, &payloads, &["strokes"], limits(),
        ));
        assert_eq!(length, expected.bytes().len());
        let mut buffer = vec![0xceu8; length + 3];
        let metadata = require_ok!(write_profile_export_artifact_into(
            &profile, &payloads, &["strokes"], limits(),
            &mut buffer,
        ));
        assert_eq!(metadata.bytes_written, length);
        assert_eq!(metadata.root_sha256, expected.root_sha256());
        assert_eq!(metadata.archive_sha256, expected.archive_sha256());
        assert_eq!(buffer.get(..length), Some(expected.bytes()));
        assert_eq!(buffer.get(length..), Some(&[0xce; 3][..]));
        if let Some(entry) = values.first_mut() {
            entry.1.push(42);
            let altered = values.iter().map(|(path, bytes)| {
                ProfileExportPayload { path, bytes }
            }).collect::<Vec<_>>();
            assert!(matches!(
                assemble_profile_export_artifact(
                    &profile, &altered, &["strokes"], limits(),
                ),
                Err(Error::PayloadVerification { .. }),
            ));
            assert!(matches!(
                planned_profile_export_artifact_bytes(
                    &profile, &altered, &["strokes"], limits(),
                ),
                Err(Error::PayloadVerification { .. }),
            ));
            assert!(matches!(
                write_profile_export_artifact_into(
                    &profile, &altered, &["strokes"],
                    limits(), &mut buffer,
                ),
                Err(Error::PayloadVerification { .. }),
            ));
            assert_eq!(buffer.get(..length), Some(expected.bytes()));
            assert_eq!(buffer.get(length..), Some(&[0xce; 3][..]));
        }
    }
}
