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
//   - Test evidence for stored ZIP byte framing, resources and CRC integrity.
// - Must-Not:
//   - Import real files, write a profile, decode typed sections or trust CRC
//     as a substitute for manifest SHA-256.
// - Allows:
//   - Inputs: Independent deterministic byte fixtures and corruption offsets.
//   - Outputs: Exact borrower views and typed error assertions.
//   - Side effects: In-memory allocations only.
// - Split-When:
//   - Additional ZIP64 reader evidence requires a separate parser.
// - Merge-When:
//   - An end-to-end canonical profile ZIP reader replaces these byte fixtures.
// - Summary:
//   - Proves that only exact, bounded, stored member payloads are admitted.
// - Description:
//   - Corrupts ordinary ZIP fields without opening any real filesystem paths.
// - Usage:
//   - Run with the Rust workspace integration suite.
// - Defaults:
//   - Unsupported or malformed archives fail before profile interpretation.
//

//! Deterministic real ZIP byte fixtures and mutation corpus.

use std::path::PathBuf;

use atrament_explicit_file_path_intent::{
    ExplicitFilePathSource, ExplicitFilePurpose, ExplicitOverwriteIntent,
    classify_explicit_file_path_intent,
};
use atrament_handwriting_profile_container::{
    PROFILE_CONTAINER_VERSION, PROFILE_MANIFEST_PATH, ProfileManifest,
    ProfileManifestEntry, Sha256Digest, profile_entry_kind,
};
use atrament_profile_import_preflight::{
    ProfileImportBudget, ProfileImportPreflightError,
    preflight_explicit_profile_import,
};

use atrament_profile_entry_sha256::{
    sha256_profile_bytes, verify_profile_entry_bytes,
};
use atrament_profile_json_syntax::{
    ProfileJsonSyntaxError, ProfileJsonSyntaxLimits, ProfileJsonValueKind,
};
use atrament_profile_zip_reader::{
    ProfileZipManifestSyntaxError,
    ProfileZipPayloadVerificationError, ProfileZipReadError,
    ProfileZipReadLimits, read_stored_profile_zip,
    verify_admitted_profile_zip_payloads,
};

const ENTRIES: [(&str, &[u8], u32); 2] = [
    ("manifest.json", b"{}", 0xa3a6_bf43),
    ("sections/strokes.json", b"123456789", 0xcbf4_3926),
];

fn limits() -> ProfileZipReadLimits {
    ProfileZipReadLimits {
        maximum_archive_bytes: 16_384,
        maximum_entries: 4,
        maximum_entry_bytes: 1_024,
        maximum_total_bytes: 2_048,
    }
}

fn put16(bytes: &mut Vec<u8>, value: u16) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn put32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn make_zip(entries: &[(&str, &[u8], u32)]) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut directory = Vec::new();
    for &(name, payload, crc) in entries {
        let offset = u32::try_from(bytes.len()).expect("small fixture offset");
        let length = u32::try_from(payload.len()).expect("small payload");
        let name_length = u16::try_from(name.len())
            .expect("small fixture name");
        put32(&mut bytes, 0x0403_4b50);
        put16(&mut bytes, 20);
        put16(&mut bytes, 0x0800);
        put16(&mut bytes, 0);
        put16(&mut bytes, 0);
        put16(&mut bytes, 0);
        put32(&mut bytes, crc);
        put32(&mut bytes, length);
        put32(&mut bytes, length);
        put16(&mut bytes, name_length);
        put16(&mut bytes, 0);
        bytes.extend_from_slice(name.as_bytes());
        bytes.extend_from_slice(payload);

        put32(&mut directory, 0x0201_4b50);
        put16(&mut directory, 20);
        put16(&mut directory, 20);
        put16(&mut directory, 0x0800);
        put16(&mut directory, 0);
        put16(&mut directory, 0);
        put16(&mut directory, 0);
        put32(&mut directory, crc);
        put32(&mut directory, length);
        put32(&mut directory, length);
        put16(&mut directory, name_length);
        put16(&mut directory, 0);
        put16(&mut directory, 0);
        put16(&mut directory, 0);
        put16(&mut directory, 0);
        put32(&mut directory, 0);
        put32(&mut directory, offset);
        directory.extend_from_slice(name.as_bytes());
    }
    let dir_offset = u32::try_from(bytes.len()).expect("small fixture size");
    let dir_length = u32::try_from(directory.len()).expect("small directory");
    let count = u16::try_from(entries.len()).expect("small entry count");
    bytes.extend_from_slice(&directory);
    put32(&mut bytes, 0x0605_4b50);
    put16(&mut bytes, 0);
    put16(&mut bytes, 0);
    put16(&mut bytes, count);
    put16(&mut bytes, count);
    put32(&mut bytes, dir_length);
    put32(&mut bytes, dir_offset);
    put16(&mut bytes, 0);
    bytes
}

fn central_start(bytes: &[u8]) -> usize {
    let offset = bytes.len() - 6;
    u32::from_le_bytes(
        bytes[offset..offset + 4].try_into().expect("valid fixture offset"),
    ) as usize
}

#[test]
fn borrowed_entries_are_exact_crc_verified_and_manifest_readable() {
    let bytes = make_zip(&ENTRIES);
    let admitted = read_stored_profile_zip(&bytes, limits())
        .expect("ordinary stored archive is canonical");
    assert_eq!(admitted.observed_paths(), [
        "manifest.json", "sections/strokes.json",
    ]);
    assert_eq!(admitted.entries()[0].bytes(), b"{}");
    assert_eq!(admitted.entries()[0].crc32(), 0xa3a6_bf43);
    assert_eq!(admitted.entries()[1].bytes(), b"123456789");
    assert_eq!(admitted.entries()[1].crc32(), 0xcbf4_3926);
    assert_eq!(admitted.entries()[1].name(), "sections/strokes.json");
    assert!(profile_entry_kind(admitted.entries()[1].name()).is_ok());
}

#[test]
fn truncation_and_trailing_bytes_cannot_be_salvaged_by_zip_scanning() {
    let bytes = make_zip(&ENTRIES);
    for truncated in 0..bytes.len() {
        assert!(read_stored_profile_zip(&bytes[..truncated], limits()).is_err(),
            "truncated={truncated}");
    }
    let mut with_suffix = bytes.clone();
    with_suffix.push(0);
    assert!(read_stored_profile_zip(&with_suffix, limits()).is_err());
    let mut with_prefix = vec![0];
    with_prefix.extend_from_slice(&bytes);
    assert!(read_stored_profile_zip(&with_prefix, limits()).is_err());
}

#[test]
fn local_payload_corruption_cannot_pass_a_matching_directory_checksum() {
    let mut bytes = make_zip(&ENTRIES);
    let offset = 30 + "manifest.json".len();
    bytes[offset] ^= 1;
    assert_eq!(
        read_stored_profile_zip(&bytes, limits()),
        Err(ProfileZipReadError::BadCrc32),
    );
}

#[test]
fn resource_bounds_apply_before_member_index_allocation() {
    let bytes = make_zip(&ENTRIES);
    let too_small_archive = ProfileZipReadLimits {
        maximum_archive_bytes: bytes.len() - 1,
        ..limits()
    };
    assert_eq!(
        read_stored_profile_zip(&bytes, too_small_archive),
        Err(ProfileZipReadError::ArchiveTooLarge),
    );
    for guarded in [
        ProfileZipReadLimits { maximum_entries: 1, ..limits() },
        ProfileZipReadLimits { maximum_entry_bytes: 8, ..limits() },
        ProfileZipReadLimits { maximum_total_bytes: 10, ..limits() },
    ] {
        assert_eq!(
            read_stored_profile_zip(&bytes, guarded),
            Err(ProfileZipReadError::ResourceLimit),
        );
    }
}

#[test]
fn compression_encryption_and_data_descriptors_are_rejected() {
    let original = make_zip(&ENTRIES);
    let central = central_start(&original);
    for (offset, replacement) in [
        (8, [1, 0]),
        (6, [1, 0]),
        (6, [8, 0]),
        (central + 10, [1, 0]),
        (central + 8, [1, 0]),
        (central + 8, [8, 0]),
    ] {
        let mut changed = original.clone();
        changed[offset..offset + 2].copy_from_slice(&replacement);
        let reason = if offset < central {
            ProfileZipReadError::InvalidStructure
        } else {
            ProfileZipReadError::UnsupportedFeature
        };
        assert_eq!(
            read_stored_profile_zip(&changed, limits()),
            Err(reason),
            "offset={offset}",
        );
    }
}

#[test]
fn split_disks_and_zip64_sentinels_reject_without_silent_fallback() {
    let original = make_zip(&ENTRIES);
    let central = central_start(&original);
    for (offset, replacement) in [
        (original.len() - 18, [1, 0, 0, 0]),
        (original.len() - 14, [255, 255, 255, 255]),
        (original.len() - 10, [255, 255, 255, 255]),
        (central + 20, [255, 255, 255, 255]),
        (central + 42, [255, 255, 255, 255]),
    ] {
        let mut changed = original.clone();
        changed[offset..offset + 4].copy_from_slice(&replacement);
        assert!(read_stored_profile_zip(&changed, limits()).is_err(),
            "offset={offset}");
    }
}

#[test]
fn unsorted_and_duplicate_entries_are_never_normalized() {
    for entries in [
        &[ENTRIES[1], ENTRIES[0]][..],
        &[ENTRIES[0], ENTRIES[0]][..],
    ] {
        let bytes = make_zip(entries);
        assert_eq!(
            read_stored_profile_zip(&bytes, limits()),
            Err(ProfileZipReadError::NonCanonical),
        );
    }
}

#[test]
fn zero_entry_archive_is_not_a_profile_but_has_valid_stored_zip_frame() {
    let bytes = make_zip(&[]);
    let read = read_stored_profile_zip(&bytes, limits())
        .expect("structurally empty ZIP leaves manifest admission to domain");
    assert!(read.entries().is_empty());
}

#[test]
fn unsafe_member_names_reject_before_bytes_enter_manifest_preflight() {
    for bad in [
        "../outside", "sections/../secret.json", "sections//empty.json",
        "assets/\0private", "assets\\backslash", "other/not-a-profile",
        "sections/./dot.json", "assets/trailing/",
    ] {
        let entries = [("manifest.json", b"{}".as_slice(), 0xa3a6_bf43),
            (bad, b"{}".as_slice(), 0xa3a6_bf43)];
        let bytes = make_zip(&entries);
        assert_eq!(
            read_stored_profile_zip(&bytes, limits()),
            Err(ProfileZipReadError::InvalidName),
            "bad={bad:?}",
        );
    }
}

#[test]
fn valid_unicode_archive_names_require_unambiguous_utf8_flag() {
    let entries = [("manifest.json", b"{}".as_slice(), 0xa3a6_bf43),
        ("sections/cuaderno ñ.json", b"{}".as_slice(), 0xa3a6_bf43)];
    let bytes = make_zip(&entries);
    let archive = read_stored_profile_zip(&bytes, limits())
        .expect("declared UTF-8 member bytes are valid");
    assert_eq!(archive.entries()[1].name(), "sections/cuaderno ñ.json");
    let central = central_start(&bytes);
    let next = central + 46 + "manifest.json".len();
    let mut without_utf8 = bytes.clone();
    without_utf8[next + 8..next + 10].copy_from_slice(&[0, 0]);
    assert_eq!(
        read_stored_profile_zip(&without_utf8, limits()),
        Err(ProfileZipReadError::UnsupportedFeature),
    );
}

#[test]
fn mutated_bytes_never_panic_or_escape_bounded_archive_slice() {
    let original = make_zip(&ENTRIES);
    let mut explored = 0usize;
    for index in (0..original.len()).step_by(3) {
        for changed_to in [0u8, 0xffu8] {
            let mut changed = original.clone();
            changed[index] = changed_to;
            let caught = std::panic::catch_unwind(|| {
                read_stored_profile_zip(&changed, limits())
            });
            assert!(caught.is_ok(), "byte index={index}");
            explored += 1;
        }
    }
    assert!(explored > 160);
}

#[test]
fn real_zip_observations_feed_exact_typed_profile_import_preflight() {
    let bytes = make_zip(&ENTRIES);
    let archive = read_stored_profile_zip(&bytes, limits())
        .expect("bounded stored ZIP framing passes");
    let paths = archive.observed_paths();
    let selected = classify_explicit_file_path_intent(
        ExplicitFilePurpose::ImportProfile,
        ExplicitFilePathSource::CallerSelection,
        PathBuf::from("selected/profile.atrament"),
        ExplicitOverwriteIntent::PreserveExisting,
    ).expect("trusted path intent is independent of archive text");
    let manifest = ProfileManifest {
        container_version: String::from(PROFILE_CONTAINER_VERSION),
        entries: vec![ProfileManifestEntry {
            byte_length: 9,
            digest: Sha256Digest::from_bytes([42; 32]),
            media_type: String::from("application/json"),
            path: String::from("sections/strokes.json"),
        }],
        optional_features: vec![],
        profile_identity: String::from("observed-in-memory"),
        required_features: vec![String::from("strokes")],
    };
    let budget = ProfileImportBudget {
        maximum_declared_bytes: 9,
        maximum_entries: 1,
        maximum_entry_bytes: 9,
    };
    let encoding = atrament_handwriting_profile_container::
        ProfileArchiveEncodingEvidence {
            entry_encoding: atrament_handwriting_profile_container::
                ProfileArchiveEntryEncoding::Stored,
            platform_extras: atrament_handwriting_profile_container::
                ProfileArchivePlatformExtras::Absent,
            zip64_requirement: atrament_handwriting_profile_container::
                ProfileZip64Requirement::Ordinary,
            zip64_use: atrament_handwriting_profile_container::
                ProfileZip64Use::Absent,
        };
    let inspected = preflight_explicit_profile_import(
        &selected, &manifest, &["strokes"], &paths, encoding, budget,
    ).expect("framing and typed inventory agree");
    assert_eq!(inspected.inventory().observed_paths(), paths);
    assert_eq!(archive.entries()[1].bytes(), b"123456789");
    // The manifest's synthetic SHA-256 is deliberately not verified here:
    // CRC only proves ZIP framing, never cryptographic profile admission.
    assert_eq!(inspected.inventory().manifest().entries[0].digest.bytes(),
        [42; 32]);

    let missing_manifest = make_zip(&[
        ("sections/strokes.json", b"123456789", 0xcbf4_3926),
    ]);
    let archive = read_stored_profile_zip(&missing_manifest, limits())
        .expect("missing manifest is structurally valid ZIP");
    let names = archive.observed_paths();
    assert_eq!(
        preflight_explicit_profile_import(
            &selected, &manifest, &["strokes"], &names, encoding, budget,
        ),
        Err(ProfileImportPreflightError::EntryInventory(
            atrament_handwriting_profile_container::
                ProfileEntryInventoryError::MissingManifest,
        )),
    );
    assert!(paths.contains(&PROFILE_MANIFEST_PATH));
}

#[test]
fn local_header_mismatches_never_replace_directory_authority() {
    let original = make_zip(&ENTRIES);
    for offset in [4usize, 10, 12, 14, 18, 22, 26, 30] {
        let mut bytes = original.clone();
        bytes[offset] ^= 1;
        assert_eq!(
            read_stored_profile_zip(&bytes, limits()),
            Err(ProfileZipReadError::InvalidStructure),
            "mismatched local byte={offset}",
        );
    }
}

#[test]
fn canonical_archive_metadata_rejects_extras_comments_and_foreign_hosts() {
    let original = make_zip(&ENTRIES);
    let directory = central_start(&original);
    for (offset, change) in [
        (directory + 5, 3u8),
        (directory + 30, 1),
        (directory + 32, 1),
        (directory + 34, 1),
        (directory + 36, 1),
        (directory + 38, 1),
    ] {
        let mut bytes = original.clone();
        bytes[offset] = change;
        assert_eq!(
            read_stored_profile_zip(&bytes, limits()),
            Err(ProfileZipReadError::NonCanonical),
            "metadata offset={offset}",
        );
    }
    let mut commented = original.clone();
    let last = commented.len() - 2;
    commented[last] = 1;
    assert_eq!(
        read_stored_profile_zip(&commented, limits()),
        Err(ProfileZipReadError::InvalidStructure),
    );
}

#[test]
fn trailing_or_unclaimed_local_bytes_never_hide_a_second_payload() {
    let original = make_zip(&ENTRIES);
    let directory = central_start(&original);
    for patch in [0u32, 1u32, 3u32, 32u32] {
        let mut bytes = original.clone();
        bytes[18..22].copy_from_slice(&patch.to_le_bytes());
        assert!(read_stored_profile_zip(&bytes, limits()).is_err());
    }
    let mut broken_offset = original.clone();
    let second = directory + 46 + "manifest.json".len();
    broken_offset[second + 42..second + 46]
        .copy_from_slice(&0u32.to_le_bytes());
    assert_eq!(
        read_stored_profile_zip(&broken_offset, limits()),
        Err(ProfileZipReadError::NonCanonical),
    );
}

#[test]
fn root_manifest_utf8_has_a_separate_byte_limit_before_json_parsing() {
    let bytes = make_zip(&ENTRIES);
    let archive = read_stored_profile_zip(&bytes, limits())
        .expect("frame must be valid");
    assert_eq!(archive.manifest_utf8(2), Ok("{}"));
    assert_eq!(
        archive.manifest_utf8(1),
        Err(ProfileZipReadError::ManifestTooLarge),
    );
    let missing = make_zip(&[]);
    let empty = read_stored_profile_zip(&missing, limits())
        .expect("empty ZIP framing is valid without Import authority");
    assert_eq!(
        empty.manifest_utf8(10),
        Err(ProfileZipReadError::MissingManifest),
    );
}

#[test]
fn invalid_manifest_utf8_is_not_reinterpreted_as_an_opaque_encoding() {
    let bytes = make_zip(&[("manifest.json", &[0xff], 0xff00_0000)]);
    let archive = read_stored_profile_zip(&bytes, limits())
        .expect("CRC checks independently of manifest text encoding");
    assert_eq!(
        archive.manifest_utf8(1),
        Err(ProfileZipReadError::InvalidManifestUtf8),
    );
    assert_eq!(
        archive.manifest_utf8(0),
        Err(ProfileZipReadError::ManifestTooLarge),
    );
}

#[test]
fn forged_large_entry_count_is_rejected_before_allocating_entry_index() {
    let original = make_zip(&[]);
    let trailer = original.len() - 22;
    for bogus_count in [1u16, 10u16, 30_000u16] {
        let mut bytes = original.clone();
        let count = bogus_count.to_le_bytes();
        bytes[trailer + 8..trailer + 10].copy_from_slice(&count);
        bytes[trailer + 10..trailer + 12].copy_from_slice(&count);
        let generous = ProfileZipReadLimits {
            maximum_entries: 60_000,
            ..limits()
        };
        assert_eq!(
            read_stored_profile_zip(&bytes, generous),
            Err(ProfileZipReadError::InvalidStructure),
            "bogus declared entries={bogus_count}",
        );
    }
}

#[test]
fn valid_crc32_cannot_replace_manifest_sha256_verification() {
    // The declared digest is the independently known SHA-256 of "123456789".
    let declaration = ProfileManifestEntry {
        byte_length: 9,
        digest: Sha256Digest::from_bytes([
            0x15, 0xe2, 0xb0, 0xd3, 0xc3, 0x38, 0x91, 0xeb,
            0xb0, 0xf1, 0xef, 0x60, 0x9e, 0xc4, 0x19, 0x42,
            0x0c, 0x20, 0xe3, 0x20, 0xce, 0x94, 0xc6, 0x5f,
            0xbc, 0x8c, 0x33, 0x12, 0x44, 0x8e, 0xb2, 0x25,
        ]),
        media_type: "application/json".into(),
        path: "sections/strokes.json".into(),
    };
    let original_bytes = make_zip(&ENTRIES);
    let original = read_stored_profile_zip(&original_bytes, limits())
        .expect("stored ZIP with matching CRC-32");
    assert!(
        verify_profile_entry_bytes(&declaration, original.entries()[1].bytes())
            .is_ok(),
    );

    // An attacker can replace bytes and supply a matching CRC-32 in both
    // ZIP headers; this is not proof of the original manifest content.
    let tampered = make_zip(&[
        ("manifest.json", b"{}", 0xa3a6_bf43),
        ("sections/strokes.json", b"123456788", 0xbcf3_09b0),
    ]);
    let observed = read_stored_profile_zip(&tampered, limits())
        .expect("changed payload has valid ZIP CRC-32");
    assert_eq!(observed.entries()[1].crc32(), 0xbcf3_09b0);
    assert_eq!(
        verify_profile_entry_bytes(&declaration, observed.entries()[1].bytes()),
        Err(
            atrament_handwriting_profile_container::
                ProfileEntryVerificationError::DigestMismatch,
        ),
    );
}

#[test]
fn admitted_zip_payloads_require_all_exact_member_hashes() {
    let bytes = make_zip(&ENTRIES);
    let archive = read_stored_profile_zip(&bytes, limits())
        .expect("known stored ZIP framing");
    let names = archive.observed_paths();
    let selected = classify_explicit_file_path_intent(
        ExplicitFilePurpose::ImportProfile,
        ExplicitFilePathSource::CallerSelection,
        PathBuf::from("selected/profile.atrament"),
        ExplicitOverwriteIntent::PreserveExisting,
    ).expect("independent Import selection");
    let manifest = ProfileManifest {
        container_version: PROFILE_CONTAINER_VERSION.into(),
        entries: vec![ProfileManifestEntry {
            byte_length: 9,
            digest: Sha256Digest::from_bytes([
                0x15, 0xe2, 0xb0, 0xd3, 0xc3, 0x38, 0x91, 0xeb,
                0xb0, 0xf1, 0xef, 0x60, 0x9e, 0xc4, 0x19, 0x42,
                0x0c, 0x20, 0xe3, 0x20, 0xce, 0x94, 0xc6, 0x5f,
                0xbc, 0x8c, 0x33, 0x12, 0x44, 0x8e, 0xb2, 0x25,
            ]),
            media_type: "application/json".into(),
            path: "sections/strokes.json".into(),
        }],
        optional_features: vec![],
        profile_identity: "typed-unbound-to-root".into(),
        required_features: vec!["strokes".into()],
    };
    let encoding =
        atrament_handwriting_profile_container::ProfileArchiveEncodingEvidence {
            entry_encoding:
                atrament_handwriting_profile_container::
                    ProfileArchiveEntryEncoding::Stored,
            platform_extras:
                atrament_handwriting_profile_container::
                    ProfileArchivePlatformExtras::Absent,
            zip64_requirement:
                atrament_handwriting_profile_container::
                    ProfileZip64Requirement::Ordinary,
            zip64_use:
                atrament_handwriting_profile_container::ProfileZip64Use::Absent,
        };
    let budget = ProfileImportBudget {
        maximum_declared_bytes: 9,
        maximum_entries: 1,
        maximum_entry_bytes: 9,
    };
    let admitted = preflight_explicit_profile_import(
        &selected, &manifest, &["strokes"], &names, encoding, budget,
    ).expect("manifest and ZIP member names preflight");
    let verified = verify_admitted_profile_zip_payloads(
        &archive, admitted.inventory(),
    ).expect("known entry SHA-256 matches its declared digest");
    assert_eq!(verified.verified_entry_count(), 1);
    assert_eq!(
        verified.verified_entry_bytes("sections/strokes.json"),
        Some(&b"123456789"[..]),
    );
    assert_eq!(verified.verified_entry_bytes("manifest.json"), None);
    assert_eq!(verified.verified_entry_bytes("sections/%73trokes.json"), None);

    let tampered_bytes = make_zip(&[
        ENTRIES[0],
        ("sections/strokes.json", b"123456788", 0xbcf3_09b0),
    ]);
    let tampered = read_stored_profile_zip(&tampered_bytes, limits())
        .expect("CRC-correct altered ZIP");
    assert_eq!(
        verify_admitted_profile_zip_payloads(
            &tampered, admitted.inventory(),
        ).err(),
        Some(ProfileZipPayloadVerificationError::Entry {
            index: 0,
            reason: atrament_handwriting_profile_container::
                ProfileEntryVerificationError::DigestMismatch,
        }),
    );

    let changed_name_bytes = make_zip(&[
        ENTRIES[0],
        ("sections/other.json", b"123456789", 0xcbf4_3926),
    ]);
    let changed_names = read_stored_profile_zip(&changed_name_bytes, limits())
        .expect("different canonical ZIP names");
    assert_eq!(
        verify_admitted_profile_zip_payloads(
            &changed_names, admitted.inventory(),
        ).err(),
        Some(ProfileZipPayloadVerificationError::InventoryMismatch),
    );
}

#[test]
fn all_member_hashes_must_pass_even_in_non_zip_manifest_order() {
    let entries = [
        ("assets/source.webp", b"x".as_slice(), 0x8cdc_1683),
        ENTRIES[0],
        ENTRIES[1],
    ];
    let bytes = make_zip(&entries);
    let archive = read_stored_profile_zip(&bytes, limits())
        .expect("all three ZIP entries are CRC-correct");
    let manifest = ProfileManifest {
        container_version: PROFILE_CONTAINER_VERSION.into(),
        entries: vec![
            ProfileManifestEntry {
                byte_length: 9,
                digest: sha256_profile_bytes(b"123456789"),
                media_type: "application/json".into(),
                path: "sections/strokes.json".into(),
            },
            ProfileManifestEntry {
                byte_length: 1,
                digest: sha256_profile_bytes(b"x"),
                media_type: "image/webp".into(),
                path: "assets/source.webp".into(),
            },
        ],
        optional_features: vec![],
        profile_identity: "supplied-typed-manifest".into(),
        required_features: vec![],
    };
    let names = archive.observed_paths();
    let admitted = atrament_handwriting_profile_container::
        validate_profile_manifest_view(&manifest, &[])
        .expect("manifest metadata structurally valid");
    let inventory = admitted.admit_entry_inventory(&names)
        .expect("same exact ZIP entries");
    let verified = verify_admitted_profile_zip_payloads(
        &archive, inventory,
    ).expect("both cryptographic hashes match");
    assert_eq!(verified.verified_entry_count(), 2);
    assert_eq!(
        verified.verified_entry_bytes("assets/source.webp"),
        Some(&b"x"[..]),
    );
    assert_eq!(
        verified.verified_entry_bytes("sections/strokes.json"),
        Some(&b"123456789"[..]),
    );

    let mut corrupt_declaration = manifest.clone();
    if let Some(asset) = corrupt_declaration.entries.get_mut(1) {
        asset.digest = Sha256Digest::from_bytes([0; 32]);
    }
    let changed_admission = atrament_handwriting_profile_container::
        validate_profile_manifest_view(&corrupt_declaration, &[])
        .expect("metadata is still structurally valid");
    let changed_inventory = changed_admission.admit_entry_inventory(&names)
        .expect("member names still match");
    assert_eq!(
        verify_admitted_profile_zip_payloads(
            &archive, changed_inventory,
        ).err(),
        Some(ProfileZipPayloadVerificationError::Entry {
            index: 1,
            reason: atrament_handwriting_profile_container::
                ProfileEntryVerificationError::DigestMismatch,
        }),
    );

    // A valid replacement CRC-32 on the later asset cannot bypass its hash.
    let changed_bytes = make_zip(&[
        ("assets/source.webp", b"z", 0x62d2_77af),
        ENTRIES[0],
        ENTRIES[1],
    ]);
    let changed_archive = read_stored_profile_zip(&changed_bytes, limits())
        .expect("CRC-correct replacement must still parse");
    assert_eq!(
        verify_admitted_profile_zip_payloads(
            &changed_archive, inventory,
        ).err(),
        Some(ProfileZipPayloadVerificationError::Entry {
            index: 1,
            reason: atrament_handwriting_profile_container::
                ProfileEntryVerificationError::DigestMismatch,
        }),
    );
}

#[test]
fn verified_member_lookup_preserves_exact_unicode_and_prefixes() {
    let entries = [
        ("assets/source.webp", b"x".as_slice(), 0x8cdc_1683),
        ENTRIES[0],
        ("sections/cuaderno ñ.json", b"123456789", 0xcbf4_3926),
        ENTRIES[1],
    ];
    let bytes = make_zip(&entries);
    let archive = read_stored_profile_zip(&bytes, limits())
        .expect("canonical UTF-8 member names");
    let manifest = ProfileManifest {
        container_version: PROFILE_CONTAINER_VERSION.into(),
        entries: vec![
            ProfileManifestEntry {
                byte_length: 9,
                digest: sha256_profile_bytes(b"123456789"),
                media_type: "application/json".into(),
                path: "sections/strokes.json".into(),
            },
            ProfileManifestEntry {
                byte_length: 1,
                digest: sha256_profile_bytes(b"x"),
                media_type: "image/webp".into(),
                path: "assets/source.webp".into(),
            },
            ProfileManifestEntry {
                byte_length: 9,
                digest: sha256_profile_bytes(b"123456789"),
                media_type: "application/json".into(),
                path: "sections/cuaderno ñ.json".into(),
            },
        ],
        optional_features: vec![],
        profile_identity: "unicode-lookup".into(),
        required_features: vec![],
    };
    let names = archive.observed_paths();
    let admission = atrament_handwriting_profile_container::
        validate_profile_manifest_view(&manifest, &[])
        .expect("valid typed declaration");
    let inventory = admission.admit_entry_inventory(&names)
        .expect("exact declared ZIP names");
    let verified = verify_admitted_profile_zip_payloads(&archive, inventory)
        .expect("all member digests are valid");
    assert_eq!(verified.verified_entry_count(), 3);
    assert_eq!(
        verified.verified_entry_bytes("sections/cuaderno ñ.json"),
        Some(&b"123456789"[..]),
    );
    assert_eq!(
        verified.verified_entry_bytes("sections/strokes.json"),
        Some(&b"123456789"[..]),
    );
    assert_eq!(
        verified.verified_entry_bytes("assets/source.webp"),
        Some(&b"x"[..]),
    );
    for near_miss in [
        "manifest.json", "", "sections/cuaderno n.json",
        "sections/cuaderno ñ.JSON", "sections/cuaderno ñ",
        "sections/cuaderno %C3%B1.json", "sections/cuaderno ñ.json/",
        "sections/strokes.json ", "sections/strokes.json/other",
        "assets/source.web", "assets/source.webp.bak",
    ] {
        assert_eq!(verified.verified_entry_bytes(near_miss), None,
            "exact name={near_miss:?}");
    }
}

#[test]
fn empty_verified_asset_remains_distinct_from_absent_member() {
    let members = [
        ("assets/empty.webp", b"".as_slice(), 0u32),
        ENTRIES[0],
    ];
    let bytes = make_zip(&members);
    let archive = read_stored_profile_zip(&bytes, limits())
        .expect("empty stored asset has valid CRC-32");
    let manifest = ProfileManifest {
        container_version: PROFILE_CONTAINER_VERSION.into(),
        entries: vec![ProfileManifestEntry {
            byte_length: 0,
            digest: sha256_profile_bytes(b""),
            media_type: "image/webp".into(),
            path: "assets/empty.webp".into(),
        }],
        optional_features: vec![],
        profile_identity: "empty-asset".into(),
        required_features: vec![],
    };
    let names = archive.observed_paths();
    let admission = atrament_handwriting_profile_container::
        validate_profile_manifest_view(&manifest, &[])
        .expect("valid zero-length declaration");
    let inventory = admission.admit_entry_inventory(&names)
        .expect("root and empty asset agree");
    let verified = verify_admitted_profile_zip_payloads(&archive, inventory)
        .expect("empty SHA-256 must match its declaration");
    assert_eq!(verified.verified_entry_count(), 1);
    assert_eq!(
        verified.verified_entry_bytes("assets/empty.webp"),
        Some(&b""[..]),
    );
    assert_eq!(verified.verified_entry_bytes("assets/absent.webp"), None);
    assert_eq!(verified.verified_entry_bytes("manifest.json"), None);
}

#[test]
fn later_member_length_mismatch_precedes_its_invalid_digest() {
    let members = [
        ("assets/source.webp", b"x".as_slice(), 0x8cdc_1683),
        ENTRIES[0],
        ENTRIES[1],
    ];
    let bytes = make_zip(&members);
    let archive = read_stored_profile_zip(&bytes, limits())
        .expect("CRC-correct stored ZIP");
    let manifest = ProfileManifest {
        container_version: PROFILE_CONTAINER_VERSION.into(),
        entries: vec![
            ProfileManifestEntry {
                byte_length: 1,
                digest: sha256_profile_bytes(b"x"),
                media_type: "image/webp".into(),
                path: "assets/source.webp".into(),
            },
            ProfileManifestEntry {
                byte_length: 8,
                digest: Sha256Digest::from_bytes([0u8; 32]),
                media_type: "application/json".into(),
                path: "sections/strokes.json".into(),
            },
        ],
        optional_features: vec![],
        profile_identity: "length-first".into(),
        required_features: vec![],
    };
    let names = archive.observed_paths();
    let admission = atrament_handwriting_profile_container::
        validate_profile_manifest_view(&manifest, &[])
        .expect("valid manifest fields");
    let inventory = admission.admit_entry_inventory(&names)
        .expect("exact ZIP names are admitted");
    assert_eq!(
        verify_admitted_profile_zip_payloads(&archive, inventory).err(),
        Some(ProfileZipPayloadVerificationError::Entry {
            index: 1,
            reason: atrament_handwriting_profile_container::
                ProfileEntryVerificationError::ByteLengthMismatch {
                    declared: 8,
                    observed: 9,
                },
        }),
    );
}

#[test]
fn root_json_syntax_is_checked_on_real_crc_valid_zip_bytes() {
    let manifest = b"{\"entry\":1,\"items\":[true,null]}";
    let bytes = make_zip(&[("manifest.json", manifest, 0x3a80_dd76)]);
    let archive = read_stored_profile_zip(&bytes, limits())
        .expect("correct stored ZIP frame and CRC");
    let caps = ProfileJsonSyntaxLimits {
        maximum_bytes: manifest.len(),
        maximum_nesting: 3,
        maximum_object_members: 8,
        maximum_values: 5,
    };
    let valid = archive.manifest_json_object(caps)
        .expect("JSON grammar succeeds");
    assert_eq!(valid.bytes(), manifest);
    assert_eq!(valid.value_count(), 5);
    assert_eq!(
        archive.manifest_json_object(ProfileJsonSyntaxLimits {
            maximum_values: 4, ..caps
        }),
        Err(ProfileZipManifestSyntaxError::Json(
            ProfileJsonSyntaxError::MaximumValuesExceeded,
        )),
    );
    assert_eq!(
        archive.manifest_json_object(ProfileJsonSyntaxLimits {
            maximum_bytes: manifest.len().saturating_sub(1), ..caps
        }),
        Err(ProfileZipManifestSyntaxError::Zip(
            ProfileZipReadError::ManifestTooLarge,
        )),
    );
}

#[test]
fn crc_valid_but_malformed_or_nonobject_root_is_not_admitted() {
    let caps = ProfileJsonSyntaxLimits {
        maximum_bytes: 128,
        maximum_nesting: 8,
        maximum_object_members: 8,
        maximum_values: 16,
    };
    let fixtures = [
        (b"{\"entry\":}".as_slice(), 0x37e4_7743,
            ProfileJsonSyntaxError::Malformed),
        (b"[]".as_slice(), 0x0d4c_bb29,
            ProfileJsonSyntaxError::RootNotObject),
    ];
    for (root, crc, expected) in fixtures {
        let bytes = make_zip(&[("manifest.json", root, crc)]);
        let archive = read_stored_profile_zip(&bytes, limits())
            .expect("malformed JSON bytes have a valid ZIP CRC");
        assert_eq!(
            archive.manifest_json_object(caps),
            Err(ProfileZipManifestSyntaxError::Json(expected)),
        );
    }
    let missing_bytes = make_zip(&[]);
    let missing = read_stored_profile_zip(&missing_bytes, limits())
        .expect("empty ZIP framing is valid");
    assert_eq!(
        missing.manifest_json_object(caps),
        Err(ProfileZipManifestSyntaxError::Zip(
            ProfileZipReadError::MissingManifest,
        )),
    );
}

#[test]
fn crc_valid_manifest_syntax_preserves_utf8_failure() {
    let invalid_utf8 = [123u8, 255u8, 125u8];
    let bytes = make_zip(&[("manifest.json", &invalid_utf8, 0x1a75_ae50)]);
    let archive = read_stored_profile_zip(&bytes, limits())
        .expect("the ZIP CRC covers the malformed UTF-8 bytes");
    let caps = ProfileJsonSyntaxLimits {
        maximum_bytes: 10,
        maximum_nesting: 4,
        maximum_object_members: 8,
        maximum_values: 16,
    };
    assert_eq!(
        archive.manifest_json_object(caps),
        Err(ProfileZipManifestSyntaxError::Zip(
            ProfileZipReadError::InvalidManifestUtf8,
        )),
    );
}

#[test]
fn crc_correct_zip_root_rejects_duplicate_decoded_json_keys() {
    let fixtures = [
        (br#"{"a":1,"\u0061":2}"#.as_slice(), 0xe447_2e53),
        (br#"{"a":{"x":1,"\u0078":2}}"#.as_slice(), 0xc9bd_7a9b),
        (br#"{"a":1,"a":2}"#.as_slice(), 0x88e1_b453),
    ];
    let caps = ProfileJsonSyntaxLimits {
        maximum_bytes: 128,
        maximum_nesting: 4,
        maximum_object_members: 8,
        maximum_values: 16,
    };
    for (root, crc32) in fixtures {
        let bytes = make_zip(&[("manifest.json", root, crc32)]);
        let archive = read_stored_profile_zip(&bytes, limits())
            .expect("correct CRC-32 cannot prove key uniqueness");
        assert_eq!(
            archive.manifest_json_object(caps),
            Err(ProfileZipManifestSyntaxError::Json(
                ProfileJsonSyntaxError::DuplicateObjectKey,
            )),
        );
    }
}

#[test]
fn crc_correct_zip_root_enforces_caller_object_member_limit() {
    let root = br#"{"a":1,"b":2,"c":3}"#;
    let bytes = make_zip(&[("manifest.json", root, 0x2ed4_5bab)]);
    let archive = read_stored_profile_zip(&bytes, limits())
        .expect("well-formed ZIP with three unique keys");
    let caps = ProfileJsonSyntaxLimits {
        maximum_bytes: root.len(),
        maximum_nesting: 2,
        maximum_object_members: 2,
        maximum_values: 8,
    };
    assert_eq!(
        archive.manifest_json_object(caps),
        Err(ProfileZipManifestSyntaxError::Json(
            ProfileJsonSyntaxError::MaximumObjectMembersExceeded,
        )),
    );
    let valid = archive.manifest_json_object(ProfileJsonSyntaxLimits {
        maximum_object_members: 3, ..caps
    }).expect("exact number of unique members is admitted");
    assert_eq!(valid.bytes(), root);
}

#[test]
fn sealed_zip_root_lookup_borrows_raw_nested_json_values() {
    let root = concat!(
        r#"{"header":{"tags":["x","y"]},"sample":"\u0061","#,
        r#""count":1e+1}"#,
    ).as_bytes();
    let bytes = make_zip(&[("manifest.json", root, 0x0eba_2758)]);
    let archive = read_stored_profile_zip(&bytes, limits())
        .expect("root JSON bytes remain CRC-verifiable ZIP payload");
    let caps = ProfileJsonSyntaxLimits {
        maximum_bytes: root.len(),
        maximum_nesting: 4,
        maximum_object_members: 5,
        maximum_values: 12,
    };
    let admitted = archive.manifest_json_object(caps)
        .expect("the exact root JSON is unambiguous");
    assert_eq!(
        admitted.root_member_json("header"),
        Some(br#"{"tags":["x","y"]}"#.as_slice()),
    );
    assert_eq!(
        admitted.root_member_json("sample"),
        Some(br#""\u0061""#.as_slice()),
    );
    assert_eq!(admitted.root_member_json("count"), Some(b"1e+1".as_slice()));
    assert_eq!(admitted.root_member_json("tags"), None);
    assert_eq!(admitted.root_member_json("Sample"), None);
}
#[test]
fn sealed_crc_checked_zip_root_exposes_nested_read_only_json_values() {
    let root =
        br#"{"nested":{"rows":[{"id":1},[true,false],"\u0061"]},"id":3}"#;
    let bytes = make_zip(&[("manifest.json", root, 0xfb5c_b798)]);
    let archive = read_stored_profile_zip(&bytes, limits())
        .expect("ZIP payload CRC and framing are correct");
    let admitted = archive
        .manifest_json_object(ProfileJsonSyntaxLimits {
            maximum_bytes: root.len(),
            maximum_nesting: 4,
            maximum_object_members: 3,
            maximum_values: 12,
        })
        .expect("ZIP root grammar and unique object keys verified");
    let root_fields = admitted.root_fields().collect::<Vec<_>>();
    assert_eq!(root_fields.len(), 2);
    assert!(root_fields[0].key_matches("nested"));
    assert_eq!(root_fields[0].key_json(), br#""nested""#);
    assert!(root_fields[1].key_matches("id"));
    assert_eq!(root_fields[1].value().unsigned_integer(), Some(3));
    assert!(
        admitted.bytes().windows(root_fields[0].key_json().len())
            .any(|slice| std::ptr::eq(slice.as_ptr(),
                root_fields[0].key_json().as_ptr()))
    );
    let nested = admitted
        .root_member_value("nested")
        .expect("root object projection");
    assert_eq!(nested.kind(), Some(ProfileJsonValueKind::Object));
    let rows = nested.object_member("rows").expect("nested array");
    let row0 = rows.array_element(0).expect("object row");
    assert_eq!(
        row0.object_member("id").expect("integer row value").bytes(),
        b"1"
    );
    let row1 = rows.array_element(1).expect("nested array");
    assert_eq!(
        row1.array_element(0).expect("first boolean").bytes(),
        b"true"
    );
    assert_eq!(
        row1.array_element(1).expect("second boolean").bytes(),
        b"false"
    );
    assert_eq!(row1.array_element(2), None);
    assert_eq!(row1.array_element(0).expect("true").boolean(), Some(true));
    assert_eq!(row1.array_element(1).expect("false").boolean(), Some(false));
    let escaped = rows.array_element(2).expect("escaped string");
    assert_eq!(escaped.bytes(), br#""\u0061""#);
    assert_eq!(
        escaped.string_chars().expect("Unicode string").collect::<String>(),
        "a",
    );
    assert_eq!(rows.array_element(3), None);
    assert_eq!(nested.object_member("id"), None);
    let root_id = admitted.root_member_value("id").expect("root id");
    assert_eq!(root_id.bytes(), b"3");
    assert_eq!(root_id.unsigned_integer(), Some(3));
    assert_eq!(admitted.root_member_value("\u{0072}ows"), None);
    let selected = row0.object_member("id").expect("borrowed value").bytes();
    assert!(
        admitted
            .bytes()
            .windows(selected.len())
            .any(|candidate| std::ptr::eq(
                candidate.as_ptr(),
                selected.as_ptr()
            ))
    );
}
