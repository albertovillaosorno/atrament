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

use std::num::NonZeroUsize;
use std::io::Cursor;
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
    preflight_explicit_profile_import_sorted,
};

use atrament_profile_entry_sha256::{
    ProfileChunkVerificationError, ProfileSha256State, sha256_profile_bytes,
    verify_profile_entry_bytes, verify_profile_entry_chunks,
};
use atrament_profile_entry_stream_reader::verify_profile_entry_from_reader;
use atrament_profile_json_syntax::{
    ProfileJsonPathSegment, ProfileJsonStringCopyError,
    ProfileJsonSyntaxError, ProfileJsonSyntaxLimits, ProfileJsonValueKind,
};
use atrament_profile_zip_reader::{
    ProfileZipManifestSyntaxError,
    ProfileZipPayloadVerificationError, ProfileZipReadError,
    ProfileZipPathCopyError, ProfileZipReadLimits, read_stored_profile_zip,
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
    let mut path_storage = ["unused"; 2];
    let count = archive.copy_observed_paths(&mut path_storage)
        .expect("caller-sized path slots for this checked archive");
    let paths = &path_storage[..count];
    assert!(std::ptr::eq(paths[0].as_ptr(),
        archive.entries()[0].name().as_ptr()));
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
    let encoding = archive.encoding_evidence();
    let inspected = preflight_explicit_profile_import(
        &selected, &manifest, &["strokes"], paths, encoding, budget,
    ).expect("framing and typed inventory agree");
    assert_eq!(inspected.inventory().observed_paths(), paths);
    let optimized = preflight_explicit_profile_import_sorted(
        &selected, &manifest, &["strokes"], paths, encoding, budget,
    ).expect("ZIP-checked sorted names admit without new membership sets");
    assert_eq!(optimized, inspected);
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
    assert_eq!(archive.encoding_evidence(), encoding);
    let mut missing_names = ["unused"; 1];
    assert_eq!(archive.copy_observed_paths(&mut missing_names), Ok(1));
    assert_eq!(
        preflight_explicit_profile_import(
            &selected, &manifest, &["strokes"], &missing_names,
            encoding, budget,
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
    let encoding = archive.encoding_evidence();
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
    let mut root_key = [0xceu8; 10];
    assert_eq!(root_fields[0].copy_key_utf8(&mut root_key[..5]),
        Err(ProfileJsonStringCopyError::BufferTooSmall { required: 6 }));
    assert_eq!(root_key, [0xce; 10]);
    assert_eq!(root_fields[0].copy_key_utf8(&mut root_key), Ok(6));
    assert_eq!(&root_key[..6], b"nested");
    assert_eq!(&root_key[6..], &[0xce; 10][6..]);
    assert!(root_fields[1].key_matches("id"));
    assert_eq!(root_fields[1].value().unsigned_integer(), Some(3));
    assert_eq!(root_fields[0].source_member_json(),
        br#""nested":{"rows":[{"id":1},[true,false],"\u0061"]}"#);
    assert_eq!(root_fields[1].source_member_json(), br#""id":3"#);
    assert_eq!(admitted.root_field("nested").expect("source-backed root")
        .source_member_json(), root_fields[0].source_member_json());
    assert_eq!(admitted.root_field("id").expect("root ID")
        .value().unsigned_integer(), Some(3));
    assert_eq!(admitted.root_field("ID"), None);
    assert!(admitted.bytes().windows(root_fields[0].source_member_json().len())
        .any(|candidate| std::ptr::eq(
            candidate.as_ptr(), root_fields[0].source_member_json().as_ptr())));
    assert!(
        admitted.bytes().windows(root_fields[0].key_json().len())
            .any(|slice| std::ptr::eq(slice.as_ptr(),
                root_fields[0].key_json().as_ptr()))
    );
    let nested = admitted
        .root_member_value("nested")
        .expect("root object projection");
    assert_eq!(nested.kind(), Some(ProfileJsonValueKind::Object));
    let rows_field = nested.object_field("rows").expect("nested field");
    assert_eq!(rows_field.source_member_json(),
        br#""rows":[{"id":1},[true,false],"\u0061"]"#);
    let rows = nested.object_member("rows").expect("nested array");
    assert_eq!(rows_field.value().bytes(), rows.bytes());
    use ProfileJsonPathSegment::{Index, Key};
    let deep_path = [Key("nested"), Key("rows"), Index(1), Index(1)];
    let selected = admitted.value_at_path(&deep_path)
        .expect("CRC-verified ZIP nested scalar");
    assert_eq!(selected.bytes(), b"false");
    assert_eq!(selected.boolean(), Some(false));
    assert!(admitted.bytes().windows(selected.bytes().len())
        .any(|segment| std::ptr::eq(segment.as_ptr(),
            selected.bytes().as_ptr())));
    assert_eq!(admitted.value_at_path(&[Index(0)]), None);
    assert_eq!(admitted.value_at_path(&[Key("nested"), Index(0)]), None);
    assert_eq!(admitted.value_at_path(&[Key("nested"), Key("rows"),
        Index(usize::MAX)]), None);
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
    let mut empty = [];
    assert_eq!(escaped.copy_string_utf8(&mut empty),
        Err(ProfileJsonStringCopyError::BufferTooSmall { required: 1 }));
    let mut output = [0xccu8; 8];
    assert_eq!(escaped.copy_string_utf8(&mut output), Ok(1));
    assert_eq!(&output[..1], b"a");
    assert_eq!(&output[1..], &[0xcc; 8][1..]);
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

#[test]
fn sorted_zip_manifest_lookup_selects_only_exact_middle_root_name() {
    let root = br#"{"id":42}"#;
    let entries: [(&str, &[u8], u32); 3] = [
        ("assets/notes.bin", b"data", 0xadf3_f363),
        ("manifest.json", root, 0x66d6_18ea),
        ("sections/notes.json", b"{}", 0xa3a6_bf43),
    ];
    let bytes = make_zip(&entries);
    let archive = read_stored_profile_zip(&bytes, limits())
        .expect("canonical ZIP with root in middle of sorted index");
    assert_eq!(archive.entries().len(), 3);
    let manifest = archive.manifest_utf8(root.len())
        .expect("only exact root filename has manifest authority");
    assert_eq!(manifest.as_bytes(), root);
    assert_eq!(archive.manifest_utf8(root.len() - 1),
        Err(ProfileZipReadError::ManifestTooLarge));
    let admitted = archive.manifest_json_object(ProfileJsonSyntaxLimits {
        maximum_bytes: root.len(),
        maximum_nesting: 2,
        maximum_object_members: 2,
        maximum_values: 3,
    }).expect("the root JSON, not any adjacent member, is admitted");
    assert_eq!(admitted.root_member_value("id").expect("id")
        .unsigned_integer(), Some(42));
    let value = admitted.root_field("id").expect("root field")
        .source_member_json();
    assert_eq!(value, br#""id":42"#);
    assert!(archive.entries()[1].bytes().windows(value.len())
        .any(|part| std::ptr::eq(part.as_ptr(), value.as_ptr())));
}

#[test]
fn nearby_nested_archive_names_cannot_substitute_for_missing_root() {
    let entries: [(&str, &[u8], u32); 2] = [
        ("assets/manifest.json", br#"{"spoof":true}"#, 0xeb29_ac2c),
        ("sections/manifest.json", b"{}", 0xa3a6_bf43),
    ];
    let bytes = make_zip(&entries);
    let archive = read_stored_profile_zip(&bytes, limits())
        .expect("both near-matching paths have valid ZIP framing");
    assert_eq!(archive.manifest_utf8(1024),
        Err(ProfileZipReadError::MissingManifest));
    assert_eq!(archive.manifest_json_object(ProfileJsonSyntaxLimits {
        maximum_bytes: 1_024,
        maximum_nesting: 3,
        maximum_object_members: 5,
        maximum_values: 10,
    }), Err(ProfileZipManifestSyntaxError::Zip(
        ProfileZipReadError::MissingManifest)));
}

#[test]
fn sealed_zip_root_json_proof_carries_exact_supplying_archive_identity() {
    let first_bytes = make_zip(&[
        ("assets/notes.bin", b"data", 0xadf3_f363),
        ("manifest.json", br#"{"id":42}"#, 0x66d6_18ea),
        ("sections/notes.json", b"{}", 0xa3a6_bf43),
    ]);
    let second_bytes = make_zip(&[
        ("assets/notes.bin", b"data", 0xadf3_f363),
        ("manifest.json", br#"{"id":0}"#, 0x5d46_c984),
        ("sections/notes.json", b"{}", 0xa3a6_bf43),
    ]);
    let first_archive = read_stored_profile_zip(&first_bytes, limits())
        .expect("first CRC-checked archive");
    let second_archive = read_stored_profile_zip(&second_bytes, limits())
        .expect("second CRC-checked archive with same entry names");
    let bounds = ProfileJsonSyntaxLimits {
        maximum_bytes: 16,
        maximum_nesting: 2,
        maximum_object_members: 2,
        maximum_values: 2,
    };
    let first = first_archive.admitted_root_json(bounds)
        .expect("first actual ZIP root admitted");
    let second = second_archive.admitted_root_json(bounds)
        .expect("second actual ZIP root admitted");
    assert!(std::ptr::eq(first.archive(), &first_archive));
    assert!(std::ptr::eq(second.archive(), &second_archive));
    assert!(!std::ptr::eq(first.archive(), second.archive()));
    assert_eq!(first.root().bytes(), br#"{"id":42}"#);
    assert_eq!(second.root().bytes(), br#"{"id":0}"#);
    assert_eq!(first.root().value_count(), 2);
    assert_eq!(second.root().value_count(), 2);
    assert_eq!(first.root().root_member_value("id").expect("first id")
        .unsigned_integer(), Some(42));
    assert_eq!(second.root().root_member_value("id").expect("second id")
        .unsigned_integer(), Some(0));
    for bound in [&first, &second] {
        let manifest = bound.archive().entries().iter()
            .find(|entry| entry.name() == "manifest.json")
            .expect("the same root member is in this archive");
        assert_eq!(bound.root().bytes().len(), manifest.bytes().len());
        assert!(std::ptr::eq(bound.root().bytes().as_ptr(),
            manifest.bytes().as_ptr()));
    }
    assert_eq!(first_archive.manifest_json_object(bounds),
        Ok(first.root()), "legacy syntax query agrees with sealed pairing");
}

#[test]
fn zipped_root_provenance_proof_fails_closed_on_json_syntax_and_budget() {
    let bytes = make_zip(&[("manifest.json", br#"{"broken":]"#,
        0xff90_525f)]);
    let archive = read_stored_profile_zip(&bytes, limits())
        .expect("valid framing and correct CRC but bad JSON");
    let generous = ProfileJsonSyntaxLimits {
        maximum_bytes: 1024,
        maximum_nesting: 10,
        maximum_object_members: 10,
        maximum_values: 10,
    };
    assert_eq!(archive.admitted_root_json(generous).err(),
        Some(ProfileZipManifestSyntaxError::Json(
            ProfileJsonSyntaxError::Malformed)));
    assert_eq!(archive.admitted_root_json(ProfileJsonSyntaxLimits {
        maximum_bytes: 4,
        ..generous
    }).err(), Some(ProfileZipManifestSyntaxError::Zip(
        ProfileZipReadError::ManifestTooLarge)));
}

#[test]
fn crc_verified_zip_member_streams_to_exact_sha256_evidence() {
    let bytes = make_zip(&ENTRIES);
    let archive = read_stored_profile_zip(&bytes, limits())
        .expect("canonical CRC-valid stored archive");
    let payload = archive.entries().iter()
        .find(|entry| entry.name() == "sections/strokes.json")
        .expect("declared stored member")
        .bytes();
    assert_eq!(payload, b"123456789");
    let declaration = ProfileManifestEntry {
        byte_length: 9,
        digest: sha256_profile_bytes(payload),
        media_type: "application/json".into(),
        path: "sections/strokes.json".into(),
    };
    let reference = verify_profile_entry_bytes(&declaration, payload)
        .expect("source-backed member matches declaration");
    for stride in [1usize, 2, 3, 4, 5, 8, 9, 64] {
        assert_eq!(verify_profile_entry_chunks(&declaration,
            payload.chunks(stride)), Ok(reference));
    }
    let mut stream = ProfileSha256State::new();
    for part in payload.chunks(2) {
        assert_eq!(stream.update(part), Ok(()));
    }
    assert_eq!(stream.bytes_hashed(), 9);
    assert_eq!(stream.finish(), declaration.digest);

    let damaged_bytes = make_zip(&[
        ENTRIES[0],
        ("sections/strokes.json", b"123456788", 0xbcf3_09b0),
    ]);
    let damaged_archive = read_stored_profile_zip(&damaged_bytes, limits())
        .expect("corrupted content with matching updated CRC still passes ZIP");
    let damaged_payload = damaged_archive.entries().iter()
        .find(|entry| entry.name() == "sections/strokes.json")
        .expect("same declared path but altered bytes")
        .bytes();
    assert_eq!(verify_profile_entry_chunks(&declaration,
        damaged_payload.chunks(2)),
        Err(ProfileChunkVerificationError::Entry(
            atrament_handwriting_profile_container::
                ProfileEntryVerificationError::DigestMismatch,
        )));
}
#[test]
fn sealed_verified_member_chunks_borrow_exact_crc_and_sha_checked_source() {
    let archive_bytes = make_zip(&ENTRIES);
    let archive = read_stored_profile_zip(&archive_bytes, limits())
        .expect("CRC-checked ZIP root and member");
    let declared = ProfileManifest {
        container_version: PROFILE_CONTAINER_VERSION.into(),
        entries: vec![ProfileManifestEntry {
            byte_length: 9,
            digest: sha256_profile_bytes(b"123456789"),
            media_type: "application/json".into(),
            path: "sections/strokes.json".into(),
        }],
        optional_features: vec![],
        profile_identity: "sealed-chunks-only".into(),
        required_features: vec![],
    };
    let names = archive.observed_paths();
    let admitted =
        atrament_handwriting_profile_container::validate_profile_manifest_view(
            &declared,
            &[],
        )
        .expect("typed declaration shape validated");
    let inventory = admitted
        .admit_entry_inventory(&names)
        .expect("exact member inventory admitted");
    let verified = verify_admitted_profile_zip_payloads(&archive, inventory)
        .expect("every declared member verified before exposing bytes");
    let original = verified
        .verified_entry_bytes("sections/strokes.json")
        .expect("verified exact member exists");
    assert_eq!(original, b"123456789");
    for chunk_size in [1usize, 2, 3, 4, 5, 8, 9, 10, 32] {
        let size = NonZeroUsize::new(chunk_size).expect("nonzero size");
        let chunks = verified
            .verified_entry_chunks("sections/strokes.json", size)
            .expect("same verified member");
        let mut observed = Vec::new();
        for (index, chunk) in chunks.enumerate() {
            observed.extend_from_slice(chunk);
            let start = index.saturating_mul(chunk_size);
            assert!(std::ptr::eq(chunk.as_ptr(), original[start..].as_ptr()));
            assert!(!chunk.is_empty());
            assert!(chunk.len() <= chunk_size);
        }
        assert_eq!(observed, original, "chunk_size={chunk_size}");
    }
    let single = NonZeroUsize::new(1).expect("nonzero size");
    for missing in [
        "manifest.json",
        "sections/STROKES.json",
        "sections/strokes.json/",
        "sections/strokes.json ",
        "sections/missing.json",
        "assets/strokes.json",
    ] {
        assert!(verified.verified_entry_chunks(missing, single).is_none());
    }
}

#[test]
fn sealed_verified_empty_asset_produces_zero_chunks_not_missing() {
    let members = [("assets/empty.webp", b"".as_slice(), 0u32), ENTRIES[0]];
    let bytes = make_zip(&members);
    let archive = read_stored_profile_zip(&bytes, limits())
        .expect("root plus CRC-valid empty asset");
    let manifest = ProfileManifest {
        container_version: PROFILE_CONTAINER_VERSION.into(),
        entries: vec![ProfileManifestEntry {
            byte_length: 0,
            digest: sha256_profile_bytes(b""),
            media_type: "image/webp".into(),
            path: "assets/empty.webp".into(),
        }],
        optional_features: vec![],
        profile_identity: "empty-asset-chunks".into(),
        required_features: vec![],
    };
    let names = archive.observed_paths();
    let admission =
        atrament_handwriting_profile_container::validate_profile_manifest_view(
            &manifest,
            &[],
        )
        .expect("valid declaration");
    let inventory = admission
        .admit_entry_inventory(&names)
        .expect("root and declared asset match");
    let verified = verify_admitted_profile_zip_payloads(&archive, inventory)
        .expect("empty member digest verified");
    let size = NonZeroUsize::new(17).expect("nonzero size");
    let chunks = verified
        .verified_entry_chunks("assets/empty.webp", size)
        .expect("present empty member is not missing");
    assert_eq!(chunks.count(), 0);
    assert!(
        verified
            .verified_entry_chunks("assets/absent.webp", size)
            .is_none()
    );
    assert!(
        verified
            .verified_entry_chunks("manifest.json", size)
            .is_none()
    );
}

#[test]
fn actual_crc_checked_zip_member_matches_bounded_reader_sha256_evidence() {
    let archived = make_zip(&ENTRIES);
    let zip = read_stored_profile_zip(&archived, limits())
        .expect("stored ZIP passes local/central and CRC checks");
    let bytes = zip.entries().iter().find(|entry|
        entry.name() == "sections/strokes.json")
        .expect("stored member present")
        .bytes();
    let manifest_entry = ProfileManifestEntry {
        byte_length: 9,
        digest: sha256_profile_bytes(b"123456789"),
        media_type: "application/json".into(),
        path: "sections/strokes.json".into(),
    };
    let mut source = Cursor::new(bytes);
    let evidence = verify_profile_entry_from_reader(&mut source,
        &manifest_entry, 9).expect("preopened CRC member has correct SHA");
    assert_eq!(evidence.byte_length, 9);
    assert_eq!(evidence.digest, manifest_entry.digest);
    assert_eq!(source.position(), 9);
    assert!(std::ptr::eq(source.get_ref().as_ptr(), bytes.as_ptr()));

    let tampered_zip = make_zip(&[
        ENTRIES[0],
        ("sections/strokes.json", b"123456788", 0xbcf3_09b0),
    ]);
    let changed = read_stored_profile_zip(&tampered_zip, limits())
        .expect("updated CRC still passes weak ZIP integrity");
    let modified = changed.entries().iter().find(|entry|
        entry.name() == "sections/strokes.json")
        .expect("stored member present")
        .bytes();
    let mut source = Cursor::new(modified);
    let observed = verify_profile_entry_from_reader(
        &mut source, &manifest_entry, 9,
    );
    assert!(matches!(observed,
        Err(atrament_profile_entry_stream_reader::
            ProfileEntryStreamReadError::Verification(
                ProfileChunkVerificationError::Entry(
                    atrament_handwriting_profile_container::
                        ProfileEntryVerificationError::DigestMismatch,
                ),
            )),
    ));
}
#[test]
fn admitted_zip_root_fingerprint_preserves_exact_json_source_whitespace() {
    // Both input SHA-256 digests and ZIP CRC32 values were independently
    // obtained from Python hashlib and zlib; the scalar values are equal.
    let cases = [
        (
            br#"{"a":1}"#.as_slice(),
            0x561b_acaf,
            "015abd7f5cc57a2dd94b7590f04ad8084273905ee33ec5cebeae62276a97f862",
        ),
        (
            br#"{ "a" : 1 }"#.as_slice(),
            0xc32b_e995,
            "c94841c938b16d8b21ccfcd43131ec607e39782e486250bcd72341c7a3af0e44",
        ),
    ];
    let caps = ProfileJsonSyntaxLimits {
        maximum_bytes: 128,
        maximum_nesting: 8,
        maximum_object_members: 8,
        maximum_values: 8,
    };
    let mut hashes = Vec::new();
    for (root, crc, golden_hex) in cases {
        let input = make_zip(&[("manifest.json", root, crc)]);
        let archive = read_stored_profile_zip(&input, limits())
            .expect("CRC-correct root framing");
        let admitted = archive
            .admitted_root_json(caps)
            .expect("exact root JSON grammar admitted");
        assert_eq!(admitted.root().bytes(), root);
        assert_eq!(
            admitted
                .root()
                .root_member_value("a")
                .expect("same semantic integer")
                .unsigned_integer(),
            Some(1)
        );
        let digest = admitted.source_sha256();
        let hex = digest
            .bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        assert_eq!(hex, golden_hex);
        assert_eq!(digest, sha256_profile_bytes(root));
        hashes.push(digest);
    }
    assert_ne!(
        hashes[0], hashes[1],
        "byte-distinct syntax sources never silently canonicalize"
    );
}

#[test]
fn admitted_root_fingerprint_does_not_cover_unrelated_zip_members() {
    let root = br#"{"a":1}"#;
    let original = make_zip(&[
        ("assets/mark.webp", b"a", 0xe8b7_be43),
        ("manifest.json", root, 0x561b_acaf),
    ]);
    let changed = make_zip(&[
        ("assets/mark.webp", b"b", 0x71be_eff9),
        ("manifest.json", root, 0x561b_acaf),
    ]);
    let bounds = ProfileJsonSyntaxLimits {
        maximum_bytes: root.len(),
        maximum_nesting: 3,
        maximum_object_members: 3,
        maximum_values: 3,
    };
    assert_ne!(original, changed);
    let first = read_stored_profile_zip(&original, limits())
        .expect("first CRC-valid ZIP");
    let second = read_stored_profile_zip(&changed, limits())
        .expect("changed CRC-valid ZIP");
    let first_root = first
        .admitted_root_json(bounds)
        .expect("source-root admission");
    let second_root = second
        .admitted_root_json(bounds)
        .expect("source-root admission");
    assert_eq!(first_root.source_sha256(), second_root.source_sha256());
    assert_eq!(first_root.source_sha256(), sha256_profile_bytes(root));
    assert_ne!(first.entries()[0].bytes(), second.entries()[0].bytes());
    assert!(std::ptr::eq(
        first_root.root().bytes().as_ptr(),
        first.entries()[1].bytes().as_ptr()
    ));
    assert!(std::ptr::eq(
        second_root.root().bytes().as_ptr(),
        second.entries()[1].bytes().as_ptr()
    ));
}

#[test]
fn checked_zip_archive_keeps_full_source_and_python_hashlib_fingerprint() {
    // Python hashlib.sha256(struct/zlib-assembled root-only stored ZIP)
    // produces 69635ab9...171c394 over exactly 126 bytes.
    let source = make_zip(&[("manifest.json", b"{}", 0xa3a6_bf43)]);
    assert_eq!(source.len(), 126);
    let archive = read_stored_profile_zip(&source, limits())
        .expect("strict CRC-correct ZIP framing");
    assert_eq!(archive.source_bytes(), source);
    assert!(std::ptr::eq(archive.source_bytes().as_ptr(), source.as_ptr()));
    assert_eq!(archive.source_sha256(), sha256_profile_bytes(&source));
    let hex = archive.source_sha256().bytes().iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    assert_eq!(hex,
        "69635ab9c8b0a4dfc5a4e6743607342a0a4e4d6a50d23ac79b0ebe41e171c394");
}

#[test]
fn complete_zip_hash_changes_when_only_a_nonroot_member_changes() {
    let root = br#"{"a":1}"#;
    let first_bytes = make_zip(&[
        ("assets/mark.webp", b"a", 0xe8b7_be43),
        ("manifest.json", root, 0x561b_acaf),
    ]);
    let second_bytes = make_zip(&[
        ("assets/mark.webp", b"b", 0x71be_eff9),
        ("manifest.json", root, 0x561b_acaf),
    ]);
    let first = read_stored_profile_zip(&first_bytes, limits())
        .expect("first full ZIP admitted");
    let second = read_stored_profile_zip(&second_bytes, limits())
        .expect("second full ZIP admitted");
    assert_ne!(first.source_bytes(), second.source_bytes());
    assert_ne!(first.source_sha256(), second.source_sha256());
    let caps = ProfileJsonSyntaxLimits {
        maximum_bytes: root.len(),
        maximum_nesting: 3,
        maximum_object_members: 3,
        maximum_values: 3,
    };
    assert_eq!(first.admitted_root_json(caps)
        .expect("valid first JSON").source_sha256(),
        second.admitted_root_json(caps)
            .expect("valid second JSON").source_sha256());
}

#[test]
fn structurally_checked_archive_hash_is_not_root_syntax_acceptance() {
    let malformed_root = br#"{"bad":]"#;
    // A CRC-valid ZIP can still contain a malformed JSON root.
    let source = make_zip(&[("manifest.json", malformed_root, 0x92aa8b5c)]);
    let archive = read_stored_profile_zip(&source, limits())
        .expect("only stored framing and CRC accepted");
    assert!(archive.matches_stored_writer_framing(),
        "matching envelope never guarantees valid JSON");
    assert_eq!(archive.source_sha256(), sha256_profile_bytes(&source));
    assert_eq!(archive.admitted_root_json(ProfileJsonSyntaxLimits {
        maximum_bytes: 40,
        maximum_nesting: 3,
        maximum_object_members: 3,
        maximum_values: 3,
    }).err(), Some(ProfileZipManifestSyntaxError::Json(
        ProfileJsonSyntaxError::Malformed)));
}

#[test]
fn crc32_collision_cannot_hide_changed_archive_sha256_source() {
    // Two independently generated 8-byte sources collide under Python zlib:
    // CRC32 = 0x8b69ccdd for both, but their SHA-256 must remain distinct.
    let first_asset: [u8; 8] = [
        0x04, 0x2e, 0x53, 0x5d, 0x01, 0xc1, 0x96, 0x7a,
    ];
    let second_asset: [u8; 8] = [
        0xfa, 0xed, 0x87, 0xb3, 0xe8, 0x47, 0x20, 0x2d,
    ];
    let first = make_zip(&[
        ("assets/mark.webp", &first_asset, 0x8b69_ccdd),
        ("manifest.json", b"{}", 0xa3a6_bf43),
    ]);
    let second = make_zip(&[
        ("assets/mark.webp", &second_asset, 0x8b69_ccdd),
        ("manifest.json", b"{}", 0xa3a6_bf43),
    ]);
    assert_ne!(first, second);
    assert_eq!(first.len(), second.len());
    let first_archive = read_stored_profile_zip(&first, limits())
        .expect("first CRC-correct collision frame");
    let second_archive = read_stored_profile_zip(&second, limits())
        .expect("second CRC-correct collision frame");
    assert_eq!(first_archive.entries()[0].crc32(), 0x8b69_ccdd);
    assert_eq!(second_archive.entries()[0].crc32(), 0x8b69_ccdd);
    assert_ne!(first_archive.entries()[0].bytes(),
        second_archive.entries()[0].bytes());
    assert_ne!(first_archive.source_sha256(), second_archive.source_sha256());
    let caps = ProfileJsonSyntaxLimits {
        maximum_bytes: 2,
        maximum_nesting: 1,
        maximum_object_members: 1,
        maximum_values: 1,
    };
    assert_eq!(first_archive.admitted_root_json(caps)
        .expect("same root").source_sha256(),
        second_archive.admitted_root_json(caps)
            .expect("same root").source_sha256());
}

#[test]
fn seeded_zip_mutations_preserve_exact_member_identity_on_any_valid_read() {
    let declared = [
        ("assets/mark.webp", b"\0\xff\x11B".as_slice(), 0x54d7_e85d),
        ("manifest.json", br#"{"id":1}"#.as_slice(), 0x445d_f8c5),
        ("sections/sample.json", b"[1,2,3]".as_slice(), 0xfd6d_b3e9),
    ];
    let original = make_zip(&declared);
    let canonical = read_stored_profile_zip(&original, limits())
        .expect("independent zlib CRC goldens form a valid source archive");
    assert_eq!(canonical.entries().len(), 3);
    let mut seed = 0x4154_5241_4d45_4e54u64;
    let mut attempted = [0usize; 4];
    let mut admitted = 0usize;
    let mut rejected = 0usize;
    let mut failure_classes = Vec::new();
    let mut touched = [false; 3];
    for case in 0usize..8_192 {
        seed = seed.wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let position = usize::try_from((seed >> 20) % original.len() as u64)
            .expect("bounded mutation position fits usize");
        let replacement = u8::try_from((seed >> 40) & 0xff)
            .expect("eight-bit replacement");
        let operation = case % 4;
        attempted[operation] += 1;
        touched[(position * 3) / original.len()] = true;
        let mut bytes = original.clone();
        match operation {
            0 => bytes[position] ^= 1u8 << (replacement % 8),
            1 => bytes.insert(position, replacement),
            2 => { let _removed = bytes.remove(position); },
            _ => bytes.insert(position, bytes[position]),
        }
        let checked = std::panic::catch_unwind(|| {
            read_stored_profile_zip(&bytes, limits())
        }).expect("untrusted ZIP mutation must not panic");
        if let Ok(archive) = checked {
            admitted += 1;
            assert_eq!(archive.entries().len(), declared.len(),
                "admitted mutation operation={operation} case={case}");
            for (member, (path, data, crc)) in
                archive.entries().iter().zip(declared)
            {
                assert_eq!(member.name(), path);
                assert_eq!(member.bytes(), data);
                assert_eq!(member.crc32(), crc);
            }
            assert_eq!(archive.source_bytes(), bytes);
            assert_eq!(archive.source_sha256(), sha256_profile_bytes(&bytes));
            let admitted_root = archive.admitted_root_json(
                ProfileJsonSyntaxLimits {
                    maximum_bytes: 64,
                    maximum_nesting: 3,
                    maximum_object_members: 4,
                    maximum_values: 4,
                },
            ).expect("mutated framing cannot substitute malformed root");
            assert_eq!(admitted_root.root().bytes(), declared[1].1);
        } else if let Err(reason) = checked {
            rejected += 1;
            if !failure_classes.contains(&reason) {
                failure_classes.push(reason);
            }
        }
    }
    assert_eq!(attempted, [2_048; 4]);
    assert!(touched.into_iter().all(|seen| seen));
    assert!(admitted > 0, "irrelevant metadata changes can be admitted");
    assert!(rejected > 0, "framing and CRC mutations are refused");
    assert!(failure_classes.contains(&ProfileZipReadError::BadCrc32));
    assert!(failure_classes.contains(&ProfileZipReadError::InvalidStructure));
    assert!(failure_classes.len() >= 4,
        "multiple independent framing failures must be exercised");
}

#[test]
fn accepted_noncanonical_zip_headers_do_not_claim_fixed_writer_metadata() {
    let input = make_zip(&[("manifest.json", b"{}", 0xa3a6_bf43)]);
    let baseline = read_stored_profile_zip(&input, limits())
        .expect("fixed writer-like archive is valid");
    assert!(baseline.matches_fixed_writer_metadata());
    assert!(baseline.matches_stored_writer_framing());
    let central = central_start(&input);
    for (case, local_field, central_field, bytes) in [
        ("creator version", None, 4, [10, 0]),
        ("extractor version", Some(4), 6, [10, 0]),
        ("ASCII UTF-8 flag", Some(6), 8, [0, 0]),
        ("DOS modified time", Some(10), 12, [1, 0]),
        ("DOS modified date", Some(12), 14, [1, 0]),
    ] {
        let mut changed = input.clone();
        if let Some(local) = local_field {
            changed[local..local + 2].copy_from_slice(&bytes);
        }
        changed[central + central_field..central + central_field + 2]
            .copy_from_slice(&bytes);
        let archive = read_stored_profile_zip(&changed, limits())
            .expect("legacy stored metadata remains readable");
        assert!(!archive.matches_fixed_writer_metadata(), "{case}");
        assert!(!archive.matches_stored_writer_framing(), "{case}");
        assert_eq!(archive.entries()[0].name(), "manifest.json", "{case}");
        assert_eq!(archive.entries()[0].bytes(), b"{}", "{case}");
        assert_eq!(archive.entries()[0].crc32(), 0xa3a6_bf43, "{case}");
        assert_ne!(archive.source_sha256(), baseline.source_sha256(),
            "raw archive metadata still contributes to source identity");
        assert_eq!(archive.admitted_root_json(ProfileJsonSyntaxLimits {
            maximum_bytes: 2,
            maximum_nesting: 1,
            maximum_object_members: 1,
            maximum_values: 1,
        }).expect("valid root syntax").source_sha256(),
        sha256_profile_bytes(b"{}"), "{case}");
    }
}

#[test]
fn fixed_writer_metadata_requires_every_header_not_just_the_root() {
    let input = make_zip(&[
        ("assets/mark.webp", b"a", 0xe8b7_be43),
        ("manifest.json", b"{}", 0xa3a6_bf43),
        ("sections/ink.json", b"{}", 0xa3a6_bf43),
    ]);
    let baseline = read_stored_profile_zip(&input, limits())
        .expect("three-member fixed metadata ZIP");
    assert!(baseline.matches_fixed_writer_metadata());
    assert!(baseline.matches_stored_writer_framing());
    let central = central_start(&input);
    let names = ["assets/mark.webp", "manifest.json", "sections/ink.json"];
    let mut local = 0usize;
    let mut directory = central;
    for (index, name) in names.into_iter().enumerate() {
        let mut changed = input.clone();
        changed[local + 10..local + 12].copy_from_slice(&[3, 0]);
        changed[directory + 12..directory + 14]
            .copy_from_slice(&[3, 0]);
        let archive = read_stored_profile_zip(&changed, limits())
            .expect("metadata agreement still parses");
        assert!(!archive.matches_fixed_writer_metadata(),
            "noncanonical member index {index}");
        assert_eq!(archive.entries(), baseline.entries());
        local += 30 + name.len() + if index == 0 { 1 } else { 2 };
        directory += 46 + name.len();
    }
}

#[test]
fn stored_writer_framing_needs_root_even_with_fixed_zip_headers() {
    for entries in [
        vec![],
        vec![("assets/mark.webp", b"x".as_slice(), 0x8cdc_1683)],
    ] {
        let bytes = make_zip(&entries);
        let archive = read_stored_profile_zip(&bytes, limits())
            .expect("ordinary stored ZIP without a manifest is readable");
        assert!(archive.matches_fixed_writer_metadata(),
            "every present ZIP header has the fixed writer's metadata");
        assert!(!archive.matches_stored_writer_framing(),
            "writer needs an exact root even when ZIP framing is valid");
        assert_eq!(archive.manifest_utf8(100),
            Err(ProfileZipReadError::MissingManifest));
    }
    let bytes = make_zip(&[("manifest.json", b"{}", 0xa3a6_bf43)]);
    let archive = read_stored_profile_zip(&bytes, limits())
        .expect("with exact root ZIP becomes a writer framing candidate");
    assert!(archive.matches_stored_writer_framing());
}

#[test]
fn checked_zip_names_copy_into_bounded_caller_storage_atomically() {
    let entries = [
        ("assets/ñ.webp", b"".as_slice(), 0),
        ("manifest.json", b"{}".as_slice(), 0xa3a6_bf43),
        ("sections/😀.json", b"123456789".as_slice(), 0xcbf4_3926),
    ];
    let source = make_zip(&entries);
    let archive = read_stored_profile_zip(&source, limits())
        .expect("sorted Unicode member names accepted");
    let mut destination = ["untouched"; 5];
    for capacity in [0usize, 1, 2] {
        assert_eq!(
            archive.copy_observed_paths(&mut destination[..capacity]),
            Err(ProfileZipPathCopyError::BufferTooSmall { required: 3 }),
        );
        assert_eq!(destination, ["untouched"; 5],
            "insufficient destination changes no entries");
    }
    assert_eq!(archive.copy_observed_paths(&mut destination[..3]), Ok(3));
    assert_eq!(&destination[..3], &[
        "assets/ñ.webp", "manifest.json", "sections/😀.json",
    ]);
    assert_eq!(&destination[3..], &["untouched"; 2]);
    for (borrowed, entry) in destination.iter().zip(archive.entries()) {
        assert!(std::ptr::eq(borrowed.as_ptr(), entry.name().as_ptr()));
        assert_eq!(borrowed.len(), entry.name().len());
    }
    assert_eq!(archive.copy_observed_paths(&mut destination), Ok(3));
    assert_eq!(&destination[3..], &["untouched"; 2]);
    assert_eq!(&destination[..3], archive.observed_paths());
}

#[test]
fn empty_and_rootless_archives_can_copy_names_without_import_authority() {
    for (entries, expected) in [
        (vec![], vec![]),
        (vec![("assets/missing.webp", b"x".as_slice(), 0x8cdc_1683)],
            vec!["assets/missing.webp"]),
    ] {
        let source = make_zip(&entries);
        let archive = read_stored_profile_zip(&source, limits())
            .expect("framed archive may lack an importable root");
        let mut names = ["unmodified"; 2];
        assert_eq!(archive.copy_observed_paths(&mut names), Ok(expected.len()));
        assert_eq!(&names[..expected.len()], expected);
        assert_eq!(&names[expected.len()..],
            &["unmodified"; 2][expected.len()..]);
        assert_eq!(archive.manifest_utf8(64),
            Err(ProfileZipReadError::MissingManifest));
        assert!(!archive.matches_stored_writer_framing());
    }
}

#[test]
fn sorted_import_preflight_still_requires_real_member_sha256_evidence() {
    let original = make_zip(&ENTRIES);
    let checked = read_stored_profile_zip(&original, limits())
        .expect("root and section have valid ZIP framing and CRC");
    let selected = classify_explicit_file_path_intent(
        ExplicitFilePurpose::ImportProfile,
        ExplicitFilePathSource::CallerSelection,
        PathBuf::from("selected/handwriting.atrament"),
        ExplicitOverwriteIntent::PreserveExisting,
    ).expect("trusted selection is independent of ZIP contents");
    let manifest = ProfileManifest {
        container_version: PROFILE_CONTAINER_VERSION.into(),
        entries: vec![ProfileManifestEntry {
            byte_length: 9,
            digest: sha256_profile_bytes(b"123456789"),
            media_type: "application/json".into(),
            path: "sections/strokes.json".into(),
        }],
        optional_features: vec![],
        profile_identity: "read-only-sha256-evidence".into(),
        required_features: vec!["strokes".into()],
    };
    let budget = ProfileImportBudget {
        maximum_declared_bytes: 9,
        maximum_entries: 1,
        maximum_entry_bytes: 9,
    };
    let encoding = checked.encoding_evidence();
    let mut names = ["unmodified"; 2];
    assert_eq!(checked.copy_observed_paths(&mut names), Ok(2));
    let preflight = preflight_explicit_profile_import_sorted(
        &selected, &manifest, &["strokes"], &names, encoding, budget,
    ).expect("exact sorted names and budget are independent of bytes");
    let verified = verify_admitted_profile_zip_payloads(
        &checked, preflight.inventory(),
    ).expect("all member hashes must match before exposure");
    assert_eq!(verified.verified_entry_count(), 1);
    assert_eq!(verified.verified_entry_bytes("sections/strokes.json"),
        Some(b"123456789".as_slice()));
    assert_eq!(verified.verified_entry_bytes("manifest.json"), None);
    let borrowed = verified.verified_entry_bytes("sections/strokes.json")
        .expect("verified member bytes");
    assert!(std::ptr::eq(borrowed.as_ptr(),
        checked.entries()[1].bytes().as_ptr()));
    let damaged = make_zip(&[
        ENTRIES[0],
        ("sections/strokes.json", b"123456788", 0xbcf3_09b0),
    ]);
    let mutated = read_stored_profile_zip(&damaged, limits())
        .expect("new CRC-valid ZIP has identical admitted paths");
    assert_eq!(mutated.encoding_evidence(), checked.encoding_evidence());
    let mut changed_names = ["unmodified"; 2];
    assert_eq!(mutated.copy_observed_paths(&mut changed_names), Ok(2));
    let changed_preflight = preflight_explicit_profile_import_sorted(
        &selected, &manifest, &["strokes"], &changed_names,
        encoding, budget,
    ).expect("structural Import preflight cannot prove entry hash");
    assert_eq!(verify_admitted_profile_zip_payloads(
        &mutated, changed_preflight.inventory(),
    ).err(), Some(ProfileZipPayloadVerificationError::Entry {
        index: 0,
        reason: atrament_handwriting_profile_container::
            ProfileEntryVerificationError::DigestMismatch,
    }));
    assert_ne!(mutated.source_sha256(), checked.source_sha256());
    assert_eq!(mutated.admitted_root_json(ProfileJsonSyntaxLimits {
        maximum_bytes: 2,
        maximum_nesting: 1,
        maximum_object_members: 1,
        maximum_values: 1,
    }).expect("unchanged root grammar").source_sha256(),
    checked.admitted_root_json(ProfileJsonSyntaxLimits {
        maximum_bytes: 2,
        maximum_nesting: 1,
        maximum_object_members: 1,
        maximum_values: 1,
    }).expect("unchanged root grammar").source_sha256());
}

#[test]
fn checked_zip_encoding_evidence_survives_readable_legacy_header_variants() {
    use atrament_handwriting_profile_container::{
        ProfileArchiveEncodingEvidence, ProfileArchiveEntryEncoding,
        ProfileArchivePlatformExtras, ProfileZip64Requirement, ProfileZip64Use,
        validate_profile_archive_encoding_view,
    };
    let original = make_zip(&ENTRIES);
    let canonical = read_stored_profile_zip(&original, limits())
        .expect("original header bytes have checked stored framing");
    let expected = ProfileArchiveEncodingEvidence {
        entry_encoding: ProfileArchiveEntryEncoding::Stored,
        platform_extras: ProfileArchivePlatformExtras::Absent,
        zip64_requirement: ProfileZip64Requirement::Ordinary,
        zip64_use: ProfileZip64Use::Absent,
    };
    assert_eq!(canonical.encoding_evidence(), expected);
    assert!(validate_profile_archive_encoding_view(
        canonical.encoding_evidence()).is_ok());
    assert!(canonical.matches_stored_writer_framing());
    let first_directory = central_start(&original);
    for (name, local_field, central_field, value) in [
        ("creator version", None, 4usize, [10u8, 0]),
        ("extractor version", Some(4usize), 6, [10, 0]),
        ("ASCII UTF-8 bit", Some(6usize), 8, [0, 0]),
        ("modified time", Some(10usize), 12, [3, 0]),
        ("modified date", Some(12usize), 14, [3, 0]),
    ] {
        let mut changed = original.clone();
        if let Some(offset) = local_field {
            changed[offset..offset + 2].copy_from_slice(&value);
        }
        changed[first_directory + central_field
            ..first_directory + central_field + 2]
            .copy_from_slice(&value);
        let archived = read_stored_profile_zip(&changed, limits())
            .expect("legacy metadata is permitted in ordinary stored ZIP");
        assert_eq!(archived.encoding_evidence(), expected, "{name}");
        assert!(!archived.matches_fixed_writer_metadata(), "{name}");
        assert!(!archived.matches_stored_writer_framing(), "{name}");
        assert_eq!(archived.entries(), canonical.entries(), "{name}");
        assert_ne!(archived.source_sha256(), canonical.source_sha256());
    }
    for (name, local_field, central_field, value) in [
        ("compressed", Some(8usize), 10usize, [8u8, 0]),
        ("encrypted", Some(6usize), 8usize, [1u8, 8]),
        ("foreign creator", None, 4usize, [20u8, 3]),
        ("central extras", None, 30usize, [1u8, 0]),
    ] {
        let mut changed = original.clone();
        if let Some(offset) = local_field {
            changed[offset..offset + 2].copy_from_slice(&value);
        }
        changed[first_directory + central_field
            ..first_directory + central_field + 2]
            .copy_from_slice(&value);
        assert!(read_stored_profile_zip(&changed, limits()).is_err(),
            "{name} must not yield any checked encoding evidence");
    }
}
