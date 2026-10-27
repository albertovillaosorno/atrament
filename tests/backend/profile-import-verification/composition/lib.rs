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
//   - Source-bound manifest, inventory and payload verification tests.
// - Must-Not:
//   - Open real files or accept/import a handwriting profile into a session.
// - Allows:
//   - Inputs: Generated in-memory ZIPs and caller-classified synthetic paths.
//   - Outputs: Admitted byte identities and precise rejection assertions.
//   - Side effects: Test-local allocations only.
// - Split-When:
//   - Real source-path and decoded-section Import tests become independent.
// - Merge-When:
//   - End-to-end selected-file Import acceptance subsumes these fixtures.
// - Summary:
//   - Fail closed on mismatched path, schema, scratch, inventory or SHA-256.
// - Description:
//   - A valid CRC alone cannot authorize a substituted member payload.
// - Usage:
//   - Run `cargo test -p atrament_profile_import_verification`.
// - Defaults:
//   - No file-system read or mutation is exercised by these tests.
//

//! Integration fixtures for attached, read-only profile Import verification.

use std::path::PathBuf;

use atrament_explicit_file_path_intent::{
    ExplicitFilePathIntent, ExplicitFilePathSource, ExplicitFilePurpose,
    ExplicitOverwriteIntent,
    classify_explicit_file_path_intent,
};
use atrament_handwriting_profile_container::{
    ProfileEntryInventoryError, ProfileEntryVerificationError,
    ProfileManifestError,
};
use atrament_profile_entry_sha256::sha256_profile_bytes;
use atrament_profile_import_preflight::{
    ProfileImportBudget, ProfileImportBudgetError, ProfileImportPreflightError,
};
use atrament_profile_import_verification::{
    ProfileImportVerificationError as Error, verify_attached_profile_import,
};
use atrament_profile_json_syntax::ProfileJsonSyntaxLimits;
use atrament_profile_manifest_json_codec::{
    DecodedProfileZipManifest, ProfileManifestJsonLimits,
    decode_attached_profile_manifest_json,
};
use atrament_profile_stored_zip_writer::{
    ProfileStoredZipMember, ProfileStoredZipWriteLimits,
    write_stored_profile_zip,
};
use atrament_profile_zip_reader::{
    ProfileZipArchive, ProfileZipPathCopyError,
    ProfileZipPayloadVerificationError,
    ProfileZipReadLimits, read_stored_profile_zip,
};

const MEMBER: &str = "sections/strokes.json";

const fn archive_limits() -> ProfileZipReadLimits {
    ProfileZipReadLimits {
        maximum_archive_bytes: 16_384,
        maximum_entries: 4,
        maximum_entry_bytes: 8_192,
        maximum_total_bytes: 12_000,
    }
}

const fn write_limits() -> ProfileStoredZipWriteLimits {
    ProfileStoredZipWriteLimits {
        maximum_archive_bytes: 16_384,
        maximum_entries: 4,
        maximum_entry_bytes: 8_192,
        maximum_total_bytes: 12_000,
    }
}

const fn budget() -> ProfileImportBudget {
    ProfileImportBudget {
        maximum_declared_bytes: 32,
        maximum_entries: 3,
        maximum_entry_bytes: 32,
    }
}

fn manifest(payload: &[u8], features: &str) -> String {
    let digest = sha256_profile_bytes(payload);
    let hash: String = digest
        .bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    format!(
        concat!(
            r#"{{"container_version":"atrament.profile/1","#,
            r#""profile_identity":"unit-1","required_features":{},"#,
            r#""optional_features":[],"entries":[{{"path":"{}","#,
            r#""media_type":"application/json","byte_length":{},"#,
            r#""sha256":"{}"}}]}}"#,
        ),
        features,
        MEMBER,
        payload.len(),
        hash,
    )
}

fn zip(root: &str, payload: &[u8], extra: Option<(&str, &[u8])>) -> Vec<u8> {
    let mut members = vec![
        ProfileStoredZipMember {
            name: "manifest.json",
            bytes: root.as_bytes(),
        },
        ProfileStoredZipMember {
            name: MEMBER,
            bytes: payload,
        },
    ];
    if let Some((name, bytes)) = extra {
        members.push(ProfileStoredZipMember { name, bytes });
        members.sort_unstable_by(|left, right| left.name.cmp(right.name));
    }
    write_stored_profile_zip(&members, write_limits())
        .expect("in-memory ZIP writer")
}

fn checked(bytes: &[u8]) -> ProfileZipArchive<'_> {
    read_stored_profile_zip(bytes, archive_limits())
        .expect("valid stored framing and CRC")
}

fn decoded<'archive, 'bytes>(
    archive: &'archive ProfileZipArchive<'bytes>,
) -> DecodedProfileZipManifest<'archive, 'bytes> {
    let source = archive
        .admitted_root_json(ProfileJsonSyntaxLimits {
            maximum_bytes: 8_192,
            maximum_nesting: 12,
            maximum_object_members: 24,
            maximum_values: 64,
        })
        .expect("root JSON syntax");
    decode_attached_profile_manifest_json(
        &source,
        ProfileManifestJsonLimits {
            maximum_entries: 3,
            maximum_features: 4,
            maximum_string_bytes: 128,
        },
    )
    .expect("strict typed root")
}

fn selected(purpose: ExplicitFilePurpose) -> ExplicitFilePathIntent {
    classify_explicit_file_path_intent(
        purpose,
        ExplicitFilePathSource::CallerSelection,
        PathBuf::from("selected/note.atrament"),
        ExplicitOverwriteIntent::PreserveExisting,
    )
    .expect("synthetic selected file intent")
}

#[test]
fn source_bound_success_releases_only_digest_verified_nonroot_bytes() {
    let payload = b"source-bound";
    let root = manifest(payload, r#"["strokes"]"#);
    let source_bytes = zip(&root, payload, None);
    let archive = checked(&source_bytes);
    let source = decoded(&archive);
    let path = selected(ExplicitFilePurpose::ImportProfile);
    let mut names = ["untouched"; 3];
    let result = verify_attached_profile_import(
        &path, &source, &["strokes"], budget(), &mut names,
    )
        .expect("complete source-bound admission");
    assert!(std::ptr::eq(result.path(), &path));
    assert!(std::ptr::eq(result.manifest(), source.manifest()));
    assert_eq!(result.root_sha256(), source.root_sha256());
    assert_eq!(result.archive_sha256(), archive.source_sha256());
    assert_eq!(result.verified_entry_count(), 1);
    assert_eq!(
        result.verified_entry_bytes(MEMBER),
        Some(payload.as_slice())
    );
    assert_eq!(result.verified_entry_bytes("manifest.json"), None);
    assert_eq!(result.verified_entry_bytes("sections/missing.json"), None);
    assert_eq!(names, ["manifest.json", MEMBER, "untouched"]);
}

#[test]
fn wrong_path_purpose_refuses_before_touching_any_scratch_slots() {
    let payload = b"a";
    let source_bytes = zip(&manifest(payload, "[]"), payload, None);
    let archive = checked(&source_bytes);
    let source = decoded(&archive);
    for purpose in [
        ExplicitFilePurpose::ExportProfile,
        ExplicitFilePurpose::ImportNotebook,
        ExplicitFilePurpose::ExportNotebook,
    ] {
        let path = selected(purpose);
        let mut names = ["before"];
        assert!(matches!(
            verify_attached_profile_import(
                &path, &source, &[], budget(), &mut names,
            ),
            Err(Error::Preflight(
                ProfileImportPreflightError::WrongPathPurpose
            ))
        ));
        assert_eq!(names, ["before"]);
    }
}

#[test]
fn too_few_name_slots_refuse_atomically_with_exact_requirement() {
    let payload = b"one";
    let source_bytes = zip(&manifest(payload, "[]"), payload, None);
    let archive = checked(&source_bytes);
    let source = decoded(&archive);
    let path = selected(ExplicitFilePurpose::ImportProfile);
    let mut names = ["preserved"];
    assert!(matches!(
        verify_attached_profile_import(
            &path, &source, &[], budget(), &mut names,
        ),
        Err(Error::ObservedPathSlots(
            ProfileZipPathCopyError::BufferTooSmall { required: 2 }
        ))
    ));
    assert_eq!(names, ["preserved"]);
}

#[test]
fn required_feature_and_resource_limits_block_before_member_access() {
    let payload = b"too-large";
    let root = manifest(payload, r#"["strokes"]"#);
    let source_bytes = zip(&root, payload, None);
    let archive = checked(&source_bytes);
    let source = decoded(&archive);
    let path = selected(ExplicitFilePurpose::ImportProfile);
    let mut names = [""; 2];
    assert!(matches!(
        verify_attached_profile_import(
                &path, &source, &[], budget(), &mut names,
            ),
        Err(Error::Preflight(ProfileImportPreflightError::Manifest(
            ProfileManifestError::UnsupportedRequiredFeature { .. }
        )))
    ));
    let mut restricted = budget();
    restricted.maximum_entry_bytes = 1;
    assert!(matches!(
        verify_attached_profile_import(
            &path, &source, &["strokes"], restricted, &mut names,
        ),
        Err(Error::Preflight(ProfileImportPreflightError::Budget(
            ProfileImportBudgetError::EntryBytesExceeded { .. }
        )))
    ));
}

#[test]
fn extra_archive_member_refuses_sorted_inventory_count() {
    let payload = b"ok";
    let root = manifest(payload, "[]");
    let source_bytes = zip(
        &root,
        payload,
        Some(("assets/unlisted.bin", b"not-declared")),
    );
    let archive = checked(&source_bytes);
    let source = decoded(&archive);
    let path = selected(ExplicitFilePurpose::ImportProfile);
    let mut names = [""; 3];
    assert!(matches!(
        verify_attached_profile_import(
            &path, &source, &[], budget(), &mut names,
        ),
        Err(Error::Preflight(
            ProfileImportPreflightError::EntryInventory(
                ProfileEntryInventoryError::ObservedCountMismatch {
                    expected: 2,
                    observed: 3,
                }
            )
        ))
    ));
}

#[test]
fn crc_correct_changed_payload_fails_sha_without_exposing_bytes() {
    let original = b"first";
    let altered = b"other";
    let root = manifest(original, "[]");
    let original_zip = zip(&root, original, None);
    let altered_zip = zip(&root, altered, None);
    let original_archive = checked(&original_zip);
    let altered_archive = checked(&altered_zip);
    let original_source = decoded(&original_archive);
    let altered_source = decoded(&altered_archive);
    let path = selected(ExplicitFilePurpose::ImportProfile);
    let mut names = [""; 2];
    let original_verified = verify_attached_profile_import(
        &path, &original_source, &[], budget(), &mut names,
    )
            .expect("unchanged digest");
    assert_eq!(
        original_verified.verified_entry_bytes(MEMBER),
        Some(original.as_slice())
    );
    assert_eq!(original_source.root_sha256(), altered_source.root_sha256());
    assert_ne!(
        original_archive.source_sha256(),
        altered_archive.source_sha256()
    );
    assert!(matches!(
        verify_attached_profile_import(
            &path, &altered_source, &[], budget(), &mut names,
        ),
        Err(Error::Payload(ProfileZipPayloadVerificationError::Entry {
            index: 0,
            reason: ProfileEntryVerificationError::DigestMismatch,
        }))
    ));
}

#[test]
fn stored_payload_length_mismatch_takes_precedence_over_sha256() {
    let declared_payload = b"123";
    let actual_payload = b"1234";
    let root = manifest(declared_payload, "[]");
    let source_bytes = zip(&root, actual_payload, None);
    let archive = checked(&source_bytes);
    let source = decoded(&archive);
    let path = selected(ExplicitFilePurpose::ImportProfile);
    let mut names = [""; 2];
    assert!(matches!(
        verify_attached_profile_import(
                &path, &source, &[], budget(), &mut names,
            ),
        Err(Error::Payload(ProfileZipPayloadVerificationError::Entry {
            index: 0,
            reason: ProfileEntryVerificationError::ByteLengthMismatch {
                declared: 3,
                observed: 4,
            },
        }))
    ));
}

fn two_member_manifest(primary: &[u8], extra: &[u8]) -> String {
    let root = manifest(primary, "[]");
    let prefix = root.strip_suffix("]}")
        .expect("one-entry fixture has a complete entries array");
    let digest = sha256_profile_bytes(extra);
    let hash: String = digest.bytes().iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let additional_entry = format!(
        concat!(
            r#"{{"path":"assets/icon.png","media_type":"image/png","#,
            r#""byte_length":{},"sha256":"{}"}}"#,
        ),
        extra.len(), hash,
    );
    format!("{prefix},{additional_entry}]}}")
}

#[test]
fn multiple_members_are_only_exposed_after_complete_digest_verification() {
    let section = b"first";
    let asset = b"second";
    let root = two_member_manifest(section, asset);
    let bytes = zip(&root, section, Some(("assets/icon.png", asset)));
    let archive = checked(&bytes);
    let source = decoded(&archive);
    let path = selected(ExplicitFilePurpose::ImportProfile);
    let mut names = [""; 3];
    let result = verify_attached_profile_import(
        &path, &source, &[], budget(), &mut names,
    ).expect("both declared hashes agree with the original ZIP");
    assert_eq!(result.verified_entry_count(), 2);
    assert_eq!(result.verified_entry_bytes(MEMBER), Some(section.as_slice()));
    assert_eq!(
        result.verified_entry_bytes("assets/icon.png"),
        Some(asset.as_slice()),
    );
    assert_eq!(names, ["assets/icon.png", "manifest.json", MEMBER]);
}

#[test]
fn any_modified_member_blocks_all_verified_bytes_even_after_first_passes() {
    let section = b"first";
    let asset = b"second";
    let root = two_member_manifest(section, asset);
    let original = zip(&root, section, Some(("assets/icon.png", asset)));
    let original_archive = checked(&original);
    let original_decoded = decoded(&original_archive);
    let path = selected(ExplicitFilePurpose::ImportProfile);
    let mut names = [""; 3];
    assert!(verify_attached_profile_import(
        &path, &original_decoded, &[], budget(), &mut names,
    ).is_ok());
    // Declarations order differs from ZIP order. The second declaration
    // fails only after the first section's valid SHA-256 has been checked.
    for (section_bytes, asset_bytes, failed_index) in [
        (section.as_slice(), b"other!".as_slice(), 1),
        (b"other".as_slice(), asset.as_slice(), 0),
    ] {
        let tampered = zip(
            &root, section_bytes, Some(("assets/icon.png", asset_bytes)),
        );
        let tampered_archive = checked(&tampered);
        let tampered_decoded = decoded(&tampered_archive);
        let mut tampered_names = [""; 3];
        assert_eq!(tampered_decoded.root_sha256(),
            original_decoded.root_sha256());
        assert_ne!(tampered_archive.source_sha256(),
            original_archive.source_sha256());
        assert!(matches!(
            verify_attached_profile_import(
                &path, &tampered_decoded, &[], budget(), &mut tampered_names,
            ),
            Err(Error::Payload(ProfileZipPayloadVerificationError::Entry {
                index,
                reason: ProfileEntryVerificationError::DigestMismatch,
            })) if index == failed_index
        ));
    }
}

#[test]
fn manifest_only_archive_admits_zero_byte_budget_without_payload_leak() {
    let root = concat!(
        r#"{"container_version":"atrament.profile/1","#,
        r#""profile_identity":"empty","required_features":[],"#,
        r#""optional_features":[],"entries":[]}"#,
    );
    let only_member = [ProfileStoredZipMember {
        name: "manifest.json",
        bytes: root.as_bytes(),
    }];
    let bytes = write_stored_profile_zip(&only_member, write_limits())
        .expect("manifest-only stored archive");
    let archive = checked(&bytes);
    let source = decoded(&archive);
    let path = selected(ExplicitFilePurpose::ImportProfile);
    let mut names = ["untouched"; 2];
    let verified = verify_attached_profile_import(
        &path,
        &source,
        &[],
        ProfileImportBudget {
            maximum_declared_bytes: 0,
            maximum_entries: 0,
            maximum_entry_bytes: 0,
        },
        &mut names,
    ).expect("a zero-entry declaration consumes no payload budget");
    assert_eq!(verified.verified_entry_count(), 0);
    assert!(verified.manifest().entries.is_empty());
    assert_eq!(verified.verified_entry_bytes("manifest.json"), None);
    assert_eq!(verified.verified_entry_bytes(MEMBER), None);
    assert_eq!(names, ["manifest.json", "untouched"]);
}
