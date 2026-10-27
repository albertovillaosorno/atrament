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
    ProfileManifest, ProfileManifestEntry,
};
use atrament_profile_entry_sha256::sha256_profile_bytes;
use atrament_profile_export_assembly::{
    ProfileExportAssemblyError as Error, ProfileExportAssemblyLimits,
    ProfileExportPayload, assemble_profile_export_artifact,
    planned_profile_export_artifact_bytes, write_profile_export_artifact_into,
};
use atrament_profile_import_preflight::ProfileImportBudget;
use atrament_profile_import_verification::verify_attached_profile_import;
use atrament_profile_json_syntax::ProfileJsonSyntaxLimits;
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

const fn limits() -> ProfileExportAssemblyLimits {
    ProfileExportAssemblyLimits {
        manifest_json: ProfileManifestJsonWriteLimits {
            maximum_bytes: 8192,
            maximum_entries: 8,
            maximum_features: 8,
            maximum_string_bytes: 256,
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
        byte_length: bytes.len() as u64,
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
        profile_identity: "writer-🖋".into(),
        required_features: vec!["strokes".into()],
    }
}

#[test]
fn empty_profile_is_real_zip_with_exact_root_identity() {
    let artifact = assemble_profile_export_artifact(
        &manifest(), &[], &["strokes"], limits(),
    ).unwrap();
    let archive = read_stored_profile_zip(
        artifact.bytes(),
        ProfileZipReadLimits {
            maximum_archive_bytes: 16384,
            maximum_entries: 1,
            maximum_entry_bytes: 8192,
            maximum_total_bytes: 12000,
        },
    ).unwrap();
    assert_eq!(archive.observed_paths(), ["manifest.json"]);
    assert_eq!(archive.source_sha256(), artifact.archive_sha256());
    assert_eq!(archive.source_bytes(), artifact.bytes());
    let root = archive.admitted_root_json(ProfileJsonSyntaxLimits {
        maximum_bytes: 8192,
        maximum_nesting: 16,
        maximum_object_members: 32,
        maximum_values: 128,
    }).unwrap();
    assert_eq!(root.source_sha256(), artifact.root_sha256());
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
    let a = assemble_profile_export_artifact(
        &first, &payloads, &["strokes"], limits(),
    ).unwrap();
    first.entries.reverse();
    let b = assemble_profile_export_artifact(
        &first, &payloads.into_iter().rev().collect::<Vec<_>>(),
        &["strokes"], limits(),
    ).unwrap();
    assert_eq!(a.bytes(), b.bytes());
    assert_eq!(a.archive_sha256(), b.archive_sha256());
    assert_eq!(a.root_sha256(), b.root_sha256());

    let archive = read_stored_profile_zip(a.bytes(), ProfileZipReadLimits {
        maximum_archive_bytes: a.bytes().len(),
        maximum_entries: 3,
        maximum_entry_bytes: 8192,
        maximum_total_bytes: 12000,
    }).unwrap();
    let names = archive.observed_paths();
    assert_eq!(names, [
        "assets/ink.bin", "manifest.json", "sections/strokes.json",
    ]);
    let root = archive.admitted_root_json(ProfileJsonSyntaxLimits {
        maximum_bytes: 8192,
        maximum_nesting: 16,
        maximum_object_members: 32,
        maximum_values: 128,
    }).unwrap();
    let attached = decode_attached_profile_manifest_json(
        &root,
        ProfileManifestJsonLimits {
            maximum_entries: 8,
            maximum_features: 8,
            maximum_string_bytes: 256,
        },
    ).unwrap();
    let path = classify_explicit_file_path_intent(
        ExplicitFilePurpose::ImportProfile,
        ExplicitFilePathSource::CallerSelection,
        PathBuf::from("profile.atrament"),
        ExplicitOverwriteIntent::PreserveExisting,
    ).unwrap();
    let mut observed = [""; 3];
    let verified = verify_attached_profile_import(
        &path, &attached, &["strokes"],
        ProfileImportBudget {
            maximum_declared_bytes: 64,
            maximum_entries: 2,
            maximum_entry_bytes: 64,
        },
        &mut observed,
    ).unwrap();
    assert_eq!(verified.manifest(), &first);
    assert_eq!(verified.root_sha256(), a.root_sha256());
    assert_eq!(verified.archive_sha256(), a.archive_sha256());
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
fn late_member_corruption_rejects_entire_archive() {
    let mut declared = manifest();
    declared.entries = vec![
        entry("assets/a.bin", b"aa"),
        entry("sections/z.bin", b"zz"),
    ];
    let clean = [
        ProfileExportPayload { path: "assets/a.bin", bytes: b"aa" },
        ProfileExportPayload { path: "sections/z.bin", bytes: b"zz" },
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
        entry("sections/ñ.json", b"{\"ink\":1}"),
        entry("assets/blank.bin", b""),
    ];
    let payloads = [
        ProfileExportPayload { path: "assets/blank.bin", bytes: b"" },
        ProfileExportPayload {
            path: "sections/ñ.json", bytes: b"{\"ink\":1}",
        },
    ];
    let expected = assemble_profile_export_artifact(
        &profile, &payloads, &["strokes"], limits(),
    ).unwrap();
    let required = planned_profile_export_artifact_bytes(
        &profile, &payloads, &["strokes"], limits(),
    ).unwrap();
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
        let evidence = write_profile_export_artifact_into(
            &profile, &payloads, &["strokes"],
            limits(), &mut destination,
        ).unwrap();
        assert_eq!(evidence.bytes_written, required);
        assert_eq!(evidence.root_sha256, expected.root_sha256());
        assert_eq!(evidence.archive_sha256, expected.archive_sha256());
        assert_eq!(&destination[..required], expected.bytes());
        assert!(destination[required..].iter().all(|&b| b == 0xa5));
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
fn generated_profile_archives_are_deterministic_and_sha_bound() {
    let mut state = 0x8ab5_41c0_9321_77b5u64;
    for case in 0..256 {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let mut profile = manifest();
        let count = usize::try_from((state >> 4) % 5).unwrap();
        let mut values = Vec::new();
        for index in 0..count {
            let name = if state & (1 << (index + 12)) == 0 {
                format!("assets/{index:02}.bin")
            } else {
                format!("sections/{index:02}-ñ.json")
            };
            let data = vec![index as u8; (index + case) % 13];
            profile.entries.push(entry(&name, &data));
            values.push((name, data));
        }
        let mut payloads = values.iter().map(|(path, bytes)| {
            ProfileExportPayload { path, bytes }
        }).collect::<Vec<_>>();
        let expected = assemble_profile_export_artifact(
            &profile, &payloads, &["strokes"], limits(),
        ).unwrap();
        profile.entries.reverse();
        payloads.reverse();
        let actual = assemble_profile_export_artifact(
            &profile, &payloads, &["strokes"], limits(),
        ).unwrap();
        assert_eq!(actual.bytes(), expected.bytes(), "case {case}");
        assert_eq!(actual.root_sha256(), expected.root_sha256());
        assert_eq!(actual.archive_sha256(), expected.archive_sha256());
        let length = planned_profile_export_artifact_bytes(
            &profile, &payloads, &["strokes"], limits(),
        ).unwrap();
        assert_eq!(length, expected.bytes().len());
        let mut buffer = vec![0xceu8; length + 3];
        let metadata = write_profile_export_artifact_into(
            &profile, &payloads, &["strokes"], limits(),
            &mut buffer,
        ).unwrap();
        assert_eq!(metadata.bytes_written, length);
        assert_eq!(metadata.root_sha256, expected.root_sha256());
        assert_eq!(metadata.archive_sha256, expected.archive_sha256());
        assert_eq!(&buffer[..length], expected.bytes());
        assert_eq!(&buffer[length..], &[0xce; 3]);
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
            assert_eq!(&buffer[..length], expected.bytes());
            assert_eq!(&buffer[length..], &[0xce; 3]);
        }
    }
}
