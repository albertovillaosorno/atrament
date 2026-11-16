// Copyright:
//   - Copyright © 2026 Alberto Villa Osorno.
// SPDX-License-Identifier:
//   - MIT
// Confidential:
//   - false
// License-File:
//   - LICENSE
//
// Boundary-Contract:
// - Owns:
//   - Test evidence for stored ZIP byte framing, resources and CRC integrity.
// - Must-Not:
//   - Import real files, write a profile, decode typed sections or trust CRC as
//     a substitute for manifest SHA-256.
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

use std::fmt::{Debug, Write as _};
use std::io::Cursor;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::{panic, ptr};

use atrament_explicit_file_path_intent::{
    ExplicitFilePathIntent, ExplicitFilePathSource, ExplicitFilePurpose,
    ExplicitOverwriteIntent, classify_explicit_file_path_intent,
};
use atrament_handwriting_profile_container::{
    PROFILE_CONTAINER_VERSION, PROFILE_MANIFEST_PATH,
    ProfileArchiveEncodingEvidence, ProfileManifest, ProfileManifestEntry,
    Sha256Digest, profile_entry_kind,
};
use atrament_profile_entry_sha256::{
    ProfileChunkVerificationError, ProfileSha256State, sha256_profile_bytes,
    verify_profile_entry_bytes, verify_profile_entry_chunks,
};
use atrament_profile_entry_stream_reader::verify_profile_entry_from_reader;
use atrament_profile_import_preflight::{
    ProfileImportBudget, ProfileImportPreflightError,
    preflight_explicit_profile_import,
    preflight_explicit_profile_import_sorted,
};
use atrament_profile_json_syntax::ProfileJsonPathSegment::{Index, Key};
use atrament_profile_json_syntax::{
    ProfileJsonStringCopyError, ProfileJsonSyntaxError,
    ProfileJsonSyntaxLimits, ProfileJsonValueKind, ValidatedProfileJsonObject,
};
use atrament_profile_zip_reader::{
    ProfileZipArchive, ProfileZipManifestSyntaxError, ProfileZipPathCopyError,
    ProfileZipPayloadVerificationError, ProfileZipReadError,
    ProfileZipReadLimits, read_stored_profile_zip,
    verify_admitted_profile_zip_payloads,
};

const ENTRIES: [(&str, &[u8], u32); 2] = [
    ("manifest.json", b"{}", 0xa3a6_bf43),
    ("sections/strokes.json", b"123456789", 0xcbf4_3926),
];

// Fallible fixture setup reports the source context instead of panicking.
trait RequireFixture<T> {
    fn require_fixture(self, context: &str) -> Result<T, String>;
}

impl<T, E: Debug> RequireFixture<T> for Result<T, E> {
    fn require_fixture(self, context: &str) -> Result<T, String> {
        self.map_err(|error| format!("{context}: {error:?}"))
    }
}

impl<T> RequireFixture<T> for Option<T> {
    fn require_fixture(self, context: &str) -> Result<T, String> {
        self.ok_or_else(|| context.to_owned())
    }
}

fn hex_digest(bytes: &[u8]) -> Result<String, String> {
    let mut text = String::new();
    for byte in bytes {
        write!(&mut text, "{byte:02x}")
            .map_err(|error| format!("digest hex formatting: {error:?}"))?;
    }
    Ok(text)
}

fn run_checked_test(test: impl FnOnce() -> Result<(), String>) {
    let result = test();
    assert!(result.is_ok(), "{result:?}");
}

const fn limits() -> ProfileZipReadLimits {
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

fn make_zip(entries: &[(&str, &[u8], u32)]) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    let mut directory = Vec::new();
    for &(name, payload, crc) in entries {
        let offset = u32::try_from(bytes.len())
            .require_fixture("small fixture offset")?;
        let length =
            u32::try_from(payload.len()).require_fixture("small payload")?;
        let name_length =
            u16::try_from(name.len()).require_fixture("small fixture name")?;
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
    let dir_offset =
        u32::try_from(bytes.len()).require_fixture("small fixture size")?;
    let dir_length =
        u32::try_from(directory.len()).require_fixture("small directory")?;
    let count =
        u16::try_from(entries.len()).require_fixture("small entry count")?;
    bytes.extend_from_slice(&directory);
    put32(&mut bytes, 0x0605_4b50);
    put16(&mut bytes, 0);
    put16(&mut bytes, 0);
    put16(&mut bytes, count);
    put16(&mut bytes, count);
    put32(&mut bytes, dir_length);
    put32(&mut bytes, dir_offset);
    put16(&mut bytes, 0);
    Ok(bytes)
}

fn central_start(bytes: &[u8]) -> Result<usize, String> {
    let offset = bytes.len().checked_sub(6).ok_or("ZIP footer too short")?;
    let end = offset.checked_add(4).ok_or("ZIP footer overflow")?;
    let start = bytes.get(offset..end).ok_or("directory offset")?;
    let directory_start = u32::from_le_bytes(
        start.try_into().require_fixture("directory offset")?,
    );
    usize::try_from(directory_start).require_fixture("directory offset usize")
}

#[test]
fn sealed_zip_debug_hides_source_bytes_and_archive_entry_names() {
    run_checked_test(|| -> Result<(), String> {
        let bytes = make_zip(&ENTRIES)?;
        let archive = read_stored_profile_zip(&bytes, limits())
            .require_fixture("CRC-checked fixture")?;
        let root = archive
            .admitted_root_json(ProfileJsonSyntaxLimits {
                maximum_bytes: 1024,
                maximum_nesting: 8,
                maximum_object_members: 16,
                maximum_values: 100,
            })
            .require_fixture("admitted manifest JSON")?;
        let debug = format!(
            "{archive:?} {:?} {root:?}",
            archive.entries().get(1).ok_or("missing ZIP entry")?,
        );
        assert!(debug.contains("ProfileZipArchive"));
        assert!(debug.contains("ProfileZipEntry"));
        assert!(debug.contains("ValidatedProfileZipRootJson"));
        for private in ["manifest.json", "sections/strokes.json"] {
            assert!(!debug.contains(private));
        }
        assert!(!debug.contains("source:"));
        assert!(!debug.contains("bytes:"));
        assert!(!debug.contains(&format!("{bytes:?}")));
        assert!(!debug.contains(&format!("{:?}", ENTRIES[1].1)));
        Ok(())
    });
}

#[test]
fn borrowed_entries_are_exact_crc_verified_and_manifest_readable() {
    run_checked_test(|| -> Result<(), String> {
        let bytes = make_zip(&ENTRIES)?;
        let admitted = read_stored_profile_zip(&bytes, limits())
            .require_fixture("ordinary stored archive is canonical")?;
        assert_eq!(admitted.observed_paths(), [
            "manifest.json",
            "sections/strokes.json",
        ]);
        assert_eq!(
            admitted
                .entries()
                .first()
                .ok_or("missing ZIP entry")?
                .bytes(),
            b"{}"
        );
        assert_eq!(
            admitted
                .entries()
                .first()
                .ok_or("missing ZIP entry")?
                .crc32(),
            0xa3a6_bf43
        );
        assert_eq!(
            admitted
                .entries()
                .get(1)
                .ok_or("missing ZIP entry")?
                .bytes(),
            b"123456789"
        );
        assert_eq!(
            admitted
                .entries()
                .get(1)
                .ok_or("missing ZIP entry")?
                .crc32(),
            0xcbf4_3926
        );
        assert_eq!(
            admitted.entries().get(1).ok_or("missing ZIP entry")?.name(),
            "sections/strokes.json"
        );
        assert!(
            profile_entry_kind(
                admitted.entries().get(1).ok_or("missing ZIP entry")?.name()
            )
            .is_ok()
        );
        Ok(())
    });
}

#[test]
fn truncation_and_trailing_bytes_cannot_be_salvaged_by_zip_scanning() {
    run_checked_test(|| -> Result<(), String> {
        let bytes = make_zip(&ENTRIES)?;
        for truncated in 0..bytes.len() {
            assert!(
                read_stored_profile_zip(
                    bytes.get(..truncated).ok_or("ZIP truncation boundary")?,
                    limits()
                )
                .is_err(),
                "truncated={truncated}"
            );
        }
        let mut with_suffix = bytes.clone();
        with_suffix.push(0);
        assert!(read_stored_profile_zip(&with_suffix, limits()).is_err());
        let mut with_prefix = vec![0];
        with_prefix.extend_from_slice(&bytes);
        assert!(read_stored_profile_zip(&with_prefix, limits()).is_err());
        Ok(())
    });
}

#[test]
fn entire_directory_is_preflighted_before_any_payload_crc() {
    run_checked_test(|| -> Result<(), String> {
        let original = make_zip(&ENTRIES)?;
        let directory = central_start(&original)?;
        let second_central = directory + 46 + "manifest.json".len();
        let first_payload = 30 + "manifest.json".len();
        // A bad earlier CRC must not mask a later record that cannot be safely
        // admitted to the bounded, canonical member index.
        for (case, changes, bounds, expected) in [
            (
                "foreign attribute",
                &[(38usize, 1u32)][..],
                limits(),
                ProfileZipReadError::NonCanonical,
            ),
            (
                "oversized member",
                &[(20usize, 1_025u32), (24, 1_025)][..],
                limits(),
                ProfileZipReadError::ResourceLimit,
            ),
            (
                "forged local offset",
                &[(42usize, 0u32)][..],
                limits(),
                ProfileZipReadError::NonCanonical,
            ),
            (
                "cumulative size limit",
                &[][..],
                ProfileZipReadLimits {
                    maximum_total_bytes: 10,
                    ..limits()
                },
                ProfileZipReadError::ResourceLimit,
            ),
        ] {
            let mut bytes = original.clone();
            *bytes.get_mut(first_payload).ok_or("bytes mutation index")? ^= 1;
            for &(field, replacement) in changes {
                bytes
                    .get_mut(second_central + field..second_central + field + 4)
                    .ok_or("central directory field")?
                    .copy_from_slice(&replacement.to_le_bytes());
            }
            assert_eq!(
                read_stored_profile_zip(&bytes, bounds),
                Err(expected),
                "late central directory {case} must be rejected first",
            );
        }
        Ok(())
    });
}

#[test]
fn local_payload_corruption_cannot_pass_a_matching_directory_checksum() {
    run_checked_test(|| -> Result<(), String> {
        let mut bytes = make_zip(&ENTRIES)?;
        let offset = 30 + "manifest.json".len();
        *bytes.get_mut(offset).ok_or("bytes mutation index")? ^= 1;
        assert_eq!(
            read_stored_profile_zip(&bytes, limits()),
            Err(ProfileZipReadError::BadCrc32),
        );
        Ok(())
    });
}

#[test]
fn resource_bounds_apply_before_member_index_allocation() {
    run_checked_test(|| -> Result<(), String> {
        let bytes = make_zip(&ENTRIES)?;
        let too_small_archive = ProfileZipReadLimits {
            maximum_archive_bytes: bytes.len() - 1,
            ..limits()
        };
        assert_eq!(
            read_stored_profile_zip(&bytes, too_small_archive),
            Err(ProfileZipReadError::ArchiveTooLarge),
        );
        for guarded in [
            ProfileZipReadLimits {
                maximum_entries: 1,
                ..limits()
            },
            ProfileZipReadLimits {
                maximum_entry_bytes: 8,
                ..limits()
            },
            ProfileZipReadLimits {
                maximum_total_bytes: 10,
                ..limits()
            },
        ] {
            assert_eq!(
                read_stored_profile_zip(&bytes, guarded),
                Err(ProfileZipReadError::ResourceLimit),
            );
        }
        Ok(())
    });
}

#[test]
fn compression_encryption_and_data_descriptors_are_rejected() {
    run_checked_test(|| -> Result<(), String> {
        let original = make_zip(&ENTRIES)?;
        let central = central_start(&original)?;
        for (offset, replacement) in [
            (8, [1, 0]),
            (6, [1, 0]),
            (6, [8, 0]),
            (central + 10, [1, 0]),
            (central + 8, [1, 0]),
            (central + 8, [8, 0]),
        ] {
            let mut changed = original.clone();
            changed
                .get_mut(offset..offset + 2)
                .ok_or("changed local header field")?
                .copy_from_slice(&replacement);
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
        Ok(())
    });
}

#[test]
fn split_disks_and_zip64_sentinels_reject_without_silent_fallback() {
    run_checked_test(|| -> Result<(), String> {
        let original = make_zip(&ENTRIES)?;
        let central = central_start(&original)?;
        for (offset, replacement) in [
            (original.len() - 18, [1, 0, 0, 0]),
            (original.len() - 14, [255, 255, 255, 255]),
            (original.len() - 10, [255, 255, 255, 255]),
            (central + 20, [255, 255, 255, 255]),
            (central + 42, [255, 255, 255, 255]),
        ] {
            let mut changed = original.clone();
            changed
                .get_mut(offset..offset + 4)
                .ok_or("changed central field")?
                .copy_from_slice(&replacement);
            assert!(
                read_stored_profile_zip(&changed, limits()).is_err(),
                "offset={offset}"
            );
        }
        Ok(())
    });
}

#[test]
fn unsorted_and_duplicate_entries_are_never_normalized() {
    run_checked_test(|| -> Result<(), String> {
        for entries in
            [&[ENTRIES[1], ENTRIES[0]][..], &[ENTRIES[0], ENTRIES[0]][..]]
        {
            let bytes = make_zip(entries)?;
            assert_eq!(
                read_stored_profile_zip(&bytes, limits()),
                Err(ProfileZipReadError::NonCanonical),
            );
        }
        Ok(())
    });
}

#[test]
fn zero_entry_archive_is_not_a_profile_but_has_valid_stored_zip_frame() {
    run_checked_test(|| -> Result<(), String> {
        let bytes = make_zip(&[])?;
        let read = read_stored_profile_zip(&bytes, limits()).require_fixture(
            "structurally empty ZIP leaves manifest admission to domain",
        )?;
        assert_eq!(read.entries().len(), 0);
        Ok(())
    });
}

#[test]
fn unsafe_member_names_reject_before_bytes_enter_manifest_preflight() {
    run_checked_test(|| -> Result<(), String> {
        for bad in [
            "../outside",
            "sections/../secret.json",
            "sections//empty.json",
            "assets/\0private",
            "assets\\backslash",
            "other/not-a-profile",
            "sections/./dot.json",
            "assets/trailing/",
        ] {
            let entries = [
                ("manifest.json", b"{}".as_slice(), 0xa3a6_bf43),
                (bad, b"{}".as_slice(), 0xa3a6_bf43),
            ];
            let bytes = make_zip(&entries)?;
            assert_eq!(
                read_stored_profile_zip(&bytes, limits()),
                Err(ProfileZipReadError::InvalidName),
                "bad={bad:?}",
            );
        }
        Ok(())
    });
}

#[test]
fn valid_unicode_archive_names_require_unambiguous_utf8_flag() {
    run_checked_test(|| -> Result<(), String> {
        let entries = [
            ("manifest.json", b"{}".as_slice(), 0xa3a6_bf43),
            (
                "sections/cuaderno \u{f1}.json",
                b"{}".as_slice(),
                0xa3a6_bf43,
            ),
        ];
        let bytes = make_zip(&entries)?;
        let archive = read_stored_profile_zip(&bytes, limits())
            .require_fixture("declared UTF-8 member bytes are valid")?;
        assert_eq!(
            archive.entries().get(1).ok_or("missing ZIP entry")?.name(),
            "sections/cuaderno \u{f1}.json"
        );
        let central = central_start(&bytes)?;
        let next = central + 46 + "manifest.json".len();
        let mut without_utf8 = bytes.clone();
        without_utf8
            .get_mut(next + 8..next + 10)
            .ok_or("UTF8 flag field")?
            .copy_from_slice(&[0, 0]);
        assert_eq!(
            read_stored_profile_zip(&without_utf8, limits()),
            Err(ProfileZipReadError::UnsupportedFeature),
        );
        Ok(())
    });
}

#[test]
fn mutated_bytes_never_panic_or_escape_bounded_archive_slice() {
    run_checked_test(|| -> Result<(), String> {
        let original = make_zip(&ENTRIES)?;
        let mut explored = 0usize;
        for index in (0..original.len()).step_by(3) {
            for changed_to in [0u8, 0xffu8] {
                let mut changed = original.clone();
                *changed.get_mut(index).ok_or("changed mutation index")? =
                    changed_to;
                let caught = panic::catch_unwind(|| {
                    read_stored_profile_zip(&changed, limits())
                });
                assert!(caught.is_ok(), "byte index={index}");
                explored += 1;
            }
        }
        assert!(explored > 160);
        Ok(())
    });
}

fn verify_missing_manifest_import_refusal(
    selected: &ExplicitFilePathIntent,
    manifest: &ProfileManifest,
    encoding: ProfileArchiveEncodingEvidence,
    budget: ProfileImportBudget,
) {
    run_checked_test(|| -> Result<(), String> {
        let missing_manifest =
            make_zip(&[("sections/strokes.json", b"123456789", 0xcbf4_3926)])?;
        let missing_archive =
            read_stored_profile_zip(&missing_manifest, limits())
                .require_fixture(
                    "missing manifest is structurally valid ZIP",
                )?;
        assert_eq!(
            missing_archive.encoding_evidence(),
            encoding,
            "rootless ZIP preflight refusal"
        );
        let mut missing_names = ["unused"; 1];
        assert_eq!(
            missing_archive.copy_observed_paths(&mut missing_names),
            Ok(1),
            "rootless ZIP preflight refusal"
        );
        assert_eq!(
        preflight_explicit_profile_import(
            selected, manifest, &["strokes"], &missing_names,
            encoding, budget,
        ),
        Err(ProfileImportPreflightError::EntryInventory(
            atrament_handwriting_profile_container::
                ProfileEntryInventoryError::MissingManifest,
        )),
     "rootless ZIP preflight refusal");
        Ok(())
    });
}

#[test]
fn real_zip_observations_feed_exact_typed_profile_import_preflight() {
    run_checked_test(|| -> Result<(), String> {
        let bytes = make_zip(&ENTRIES)?;
        let archive = read_stored_profile_zip(&bytes, limits())
            .require_fixture("bounded stored ZIP framing passes")?;
        let mut path_storage = ["unused"; 2];
        let count = archive
            .copy_observed_paths(&mut path_storage)
            .require_fixture(
                "caller-sized path slots for this checked archive",
            )?;
        let paths = path_storage.get(..count).ok_or("path slice count")?;
        assert!(ptr::eq(
            paths.first().ok_or("missing copied path")?.as_ptr(),
            archive
                .entries()
                .first()
                .ok_or("missing ZIP entry")?
                .name()
                .as_ptr()
        ));
        let selected = classify_explicit_file_path_intent(
            ExplicitFilePurpose::ImportProfile,
            ExplicitFilePathSource::CallerSelection,
            PathBuf::from("selected/profile.atrament"),
            ExplicitOverwriteIntent::PreserveExisting,
        )
        .require_fixture(
            "trusted path intent is independent of archive text",
        )?;
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
            &selected,
            &manifest,
            &["strokes"],
            paths,
            encoding,
            budget,
        )
        .require_fixture("framing and typed inventory agree")?;
        assert_eq!(inspected.inventory().observed_paths(), paths);
        let optimized = preflight_explicit_profile_import_sorted(
            &selected,
            &manifest,
            &["strokes"],
            paths,
            encoding,
            budget,
        )
        .require_fixture(
            "ZIP-checked sorted names admit without new membership sets",
        )?;
        assert_eq!(optimized, inspected);
        assert_eq!(
            archive.entries().get(1).ok_or("missing ZIP entry")?.bytes(),
            b"123456789"
        );
        // The manifest's synthetic SHA-256 is deliberately not verified here:
        // CRC only proves ZIP framing, never cryptographic profile admission.
        assert_eq!(
            inspected
                .inventory()
                .manifest()
                .entries
                .first()
                .ok_or("missing declared entry")?
                .digest
                .bytes(),
            [42; 32]
        );

        verify_missing_manifest_import_refusal(
            &selected, &manifest, encoding, budget,
        );
        assert!(paths.contains(&PROFILE_MANIFEST_PATH));
        Ok(())
    });
}

#[test]
fn local_header_mismatches_never_replace_directory_authority() {
    run_checked_test(|| -> Result<(), String> {
        let original = make_zip(&ENTRIES)?;
        for offset in [4usize, 10, 12, 14, 18, 22, 26, 30] {
            let mut bytes = original.clone();
            *bytes.get_mut(offset).ok_or("bytes mutation index")? ^= 1;
            assert_eq!(
                read_stored_profile_zip(&bytes, limits()),
                Err(ProfileZipReadError::InvalidStructure),
                "mismatched local byte={offset}",
            );
        }
        Ok(())
    });
}

#[test]
fn canonical_archive_metadata_rejects_extras_comments_and_foreign_hosts() {
    run_checked_test(|| -> Result<(), String> {
        let original = make_zip(&ENTRIES)?;
        let directory = central_start(&original)?;
        for (offset, change) in [
            (directory + 5, 3u8),
            (directory + 30, 1),
            (directory + 32, 1),
            (directory + 34, 1),
            (directory + 36, 1),
            (directory + 38, 1),
        ] {
            let mut bytes = original.clone();
            *bytes.get_mut(offset).ok_or("bytes mutation index")? = change;
            assert_eq!(
                read_stored_profile_zip(&bytes, limits()),
                Err(ProfileZipReadError::NonCanonical),
                "metadata offset={offset}",
            );
        }
        let mut commented = original;
        let last = commented.len() - 2;
        *commented.get_mut(last).ok_or("commented mutation index")? = 1;
        assert_eq!(
            read_stored_profile_zip(&commented, limits()),
            Err(ProfileZipReadError::InvalidStructure),
        );
        Ok(())
    });
}

#[test]
fn trailing_or_unclaimed_local_bytes_never_hide_a_second_payload() {
    run_checked_test(|| -> Result<(), String> {
        let original = make_zip(&ENTRIES)?;
        let directory = central_start(&original)?;
        for patch in [0u32, 1u32, 3u32, 32u32] {
            let mut bytes = original.clone();
            bytes
                .get_mut(18..22)
                .ok_or("local size field")?
                .copy_from_slice(&patch.to_le_bytes());
            assert!(read_stored_profile_zip(&bytes, limits()).is_err());
        }
        let mut broken_offset = original;
        let second = directory + 46 + "manifest.json".len();
        broken_offset
            .get_mut(second + 42..second + 46)
            .ok_or("broken central offset")?
            .copy_from_slice(&0u32.to_le_bytes());
        assert_eq!(
            read_stored_profile_zip(&broken_offset, limits()),
            Err(ProfileZipReadError::NonCanonical),
        );
        Ok(())
    });
}

#[test]
fn corrected_directory_offsets_cannot_hide_unclaimed_local_bytes() {
    run_checked_test(|| -> Result<(), String> {
        let original = make_zip(&ENTRIES)?;
        let first_end = 30 + "manifest.json".len() + ENTRIES[0].1.len();
        let local_end = central_start(&original)?;
        let unexpected = b"PK\x03\x04unclaimed";
        for insertion in [first_end, local_end] {
            let mut bytes = original.clone();
            assert_eq!(
                bytes
                    .splice(insertion..insertion, unexpected.iter().copied())
                    .count(),
                0
            );
            let changed_central = local_end + unexpected.len();
            let second_directory = changed_central + 46 + "manifest.json".len();
            if insertion == first_end {
                let second_offset = u32::try_from(first_end + unexpected.len())
                    .require_fixture("bounded second member offset")?;
                bytes
                    .get_mut(second_directory + 42..second_directory + 46)
                    .ok_or("second central offset")?
                    .copy_from_slice(&second_offset.to_le_bytes());
            }
            let directory_offset = u32::try_from(changed_central)
                .require_fixture("bounded central directory offset")?;
            let end_record_directory = bytes.len() - 6;
            bytes
                .get_mut(end_record_directory..end_record_directory + 4)
                .ok_or("ZIP directory offset")?
                .copy_from_slice(&directory_offset.to_le_bytes());
            assert_eq!(
                read_stored_profile_zip(&bytes, limits()),
                Err(if insertion == first_end {
                    ProfileZipReadError::NonCanonical
                } else {
                    ProfileZipReadError::InvalidStructure
                }),
                "unclaimed bytes at offset {insertion} must not be borrowed",
            );
        }
        Ok(())
    });
}

#[test]
fn root_manifest_utf8_has_a_separate_byte_limit_before_json_parsing() {
    run_checked_test(|| -> Result<(), String> {
        let bytes = make_zip(&ENTRIES)?;
        let archive = read_stored_profile_zip(&bytes, limits())
            .require_fixture("frame must be valid")?;
        assert_eq!(archive.manifest_utf8(2), Ok("{}"));
        assert_eq!(
            archive.manifest_utf8(1),
            Err(ProfileZipReadError::ManifestTooLarge),
        );
        let missing = make_zip(&[])?;
        let empty = read_stored_profile_zip(&missing, limits())
            .require_fixture(
                "empty ZIP framing is valid without Import authority",
            )?;
        assert_eq!(
            empty.manifest_utf8(10),
            Err(ProfileZipReadError::MissingManifest),
        );
        Ok(())
    });
}

#[test]
fn invalid_manifest_utf8_is_not_reinterpreted_as_an_opaque_encoding() {
    run_checked_test(|| -> Result<(), String> {
        let bytes = make_zip(&[("manifest.json", &[0xff], 0xff00_0000)])?;
        let archive = read_stored_profile_zip(&bytes, limits())
            .require_fixture(
                "CRC checks independently of manifest text encoding",
            )?;
        assert_eq!(
            archive.manifest_utf8(1),
            Err(ProfileZipReadError::InvalidManifestUtf8),
        );
        assert_eq!(
            archive.manifest_utf8(0),
            Err(ProfileZipReadError::ManifestTooLarge),
        );
        Ok(())
    });
}

#[test]
fn forged_large_entry_count_is_rejected_before_allocating_entry_index() {
    run_checked_test(|| -> Result<(), String> {
        let original = make_zip(&[])?;
        let trailer = original.len() - 22;
        for bogus_count in [1u16, 10u16, 30_000u16] {
            let mut bytes = original.clone();
            let count = bogus_count.to_le_bytes();
            bytes
                .get_mut(trailer + 8..trailer + 10)
                .ok_or("ZIP end count one")?
                .copy_from_slice(&count);
            bytes
                .get_mut(trailer + 10..trailer + 12)
                .ok_or("ZIP end count two")?
                .copy_from_slice(&count);
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
        Ok(())
    });
}

#[test]
fn valid_crc32_cannot_replace_manifest_sha256_verification() {
    run_checked_test(|| -> Result<(), String> {
        // The declared digest is the independently known SHA-256 of
        // "123456789".
        let declaration = ProfileManifestEntry {
            byte_length: 9,
            digest: Sha256Digest::from_bytes([
                0x15, 0xe2, 0xb0, 0xd3, 0xc3, 0x38, 0x91, 0xeb, 0xb0, 0xf1,
                0xef, 0x60, 0x9e, 0xc4, 0x19, 0x42, 0x0c, 0x20, 0xe3, 0x20,
                0xce, 0x94, 0xc6, 0x5f, 0xbc, 0x8c, 0x33, 0x12, 0x44, 0x8e,
                0xb2, 0x25,
            ]),
            media_type: "application/json".into(),
            path: "sections/strokes.json".into(),
        };
        let original_bytes = make_zip(&ENTRIES)?;
        let original = read_stored_profile_zip(&original_bytes, limits())
            .require_fixture("stored ZIP with matching CRC-32")?;
        assert!(
            verify_profile_entry_bytes(
                &declaration,
                original
                    .entries()
                    .get(1)
                    .ok_or("missing ZIP entry")?
                    .bytes()
            )
            .is_ok(),
        );

        // An attacker can replace bytes and supply a matching CRC-32 in both
        // ZIP headers; this is not proof of the original manifest content.
        let tampered = make_zip(&[
            ("manifest.json", b"{}", 0xa3a6_bf43),
            ("sections/strokes.json", b"123456788", 0xbcf3_09b0),
        ])?;
        let observed = read_stored_profile_zip(&tampered, limits())
            .require_fixture("changed payload has valid ZIP CRC-32")?;
        assert_eq!(
            observed
                .entries()
                .get(1)
                .ok_or("missing ZIP entry")?
                .crc32(),
            0xbcf3_09b0
        );
        assert_eq!(
        verify_profile_entry_bytes(
            &declaration,
            observed.entries().get(1).ok_or("missing ZIP entry")?.bytes(),
        ),
        Err(
            atrament_handwriting_profile_container::
                ProfileEntryVerificationError::DigestMismatch,
        ),
    );
        Ok(())
    });
}

#[test]
fn admitted_zip_payloads_require_all_exact_member_hashes() {
    run_checked_test(|| -> Result<(), String> {
        let bytes = make_zip(&ENTRIES)?;
        let archive = read_stored_profile_zip(&bytes, limits())
            .require_fixture("known stored ZIP framing")?;
        let names = archive.observed_paths();
        let selected = classify_explicit_file_path_intent(
            ExplicitFilePurpose::ImportProfile,
            ExplicitFilePathSource::CallerSelection,
            PathBuf::from("selected/profile.atrament"),
            ExplicitOverwriteIntent::PreserveExisting,
        )
        .require_fixture("independent Import selection")?;
        let manifest = ProfileManifest {
            container_version: PROFILE_CONTAINER_VERSION.into(),
            entries: vec![ProfileManifestEntry {
                byte_length: 9,
                digest: Sha256Digest::from_bytes([
                    0x15, 0xe2, 0xb0, 0xd3, 0xc3, 0x38, 0x91, 0xeb, 0xb0, 0xf1,
                    0xef, 0x60, 0x9e, 0xc4, 0x19, 0x42, 0x0c, 0x20, 0xe3, 0x20,
                    0xce, 0x94, 0xc6, 0x5f, 0xbc, 0x8c, 0x33, 0x12, 0x44, 0x8e,
                    0xb2, 0x25,
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
            &selected,
            &manifest,
            &["strokes"],
            &names,
            encoding,
            budget,
        )
        .require_fixture("manifest and ZIP member names preflight")?;
        let verified = verify_admitted_profile_zip_payloads(
            &archive,
            admitted.inventory(),
        )
        .require_fixture("known entry SHA-256 matches its declared digest")?;
        let debug = format!("{verified:?}");
        assert!(debug.contains("VerifiedProfileZipPayloads"));
        assert!(!debug.contains("typed-unbound-to-root"));
        assert!(!debug.contains("sections/strokes.json"));
        assert!(!debug.contains(&format!("{bytes:?}")));
        assert_eq!(verified.verified_entry_count(), 1);
        assert_eq!(
            verified.verified_entry_bytes("sections/strokes.json"),
            Some(&b"123456789"[..]),
        );
        assert_eq!(verified.verified_entry_bytes("manifest.json"), None);
        assert_eq!(
            verified.verified_entry_bytes("sections/%73trokes.json"),
            None
        );

        let tampered_bytes = make_zip(&[
            ENTRIES[0],
            ("sections/strokes.json", b"123456788", 0xbcf3_09b0),
        ])?;
        let tampered = read_stored_profile_zip(&tampered_bytes, limits())
            .require_fixture("CRC-correct altered ZIP")?;
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
        ])?;
        let changed_names =
            read_stored_profile_zip(&changed_name_bytes, limits())
                .require_fixture("different canonical ZIP names")?;
        assert_eq!(
            verify_admitted_profile_zip_payloads(
                &changed_names,
                admitted.inventory(),
            )
            .err(),
            Some(ProfileZipPayloadVerificationError::InventoryMismatch),
        );
        Ok(())
    });
}

#[test]
fn all_member_hashes_must_pass_even_in_non_zip_manifest_order() {
    run_checked_test(|| -> Result<(), String> {
        let entries = [
            ("assets/source.webp", b"x".as_slice(), 0x8cdc_1683),
            ENTRIES[0],
            ENTRIES[1],
        ];
        let bytes = make_zip(&entries)?;
        let archive = read_stored_profile_zip(&bytes, limits())
            .require_fixture("all three ZIP entries are CRC-correct")?;
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
        .require_fixture("manifest metadata structurally valid")?;
        let inventory = admitted
            .admit_entry_inventory(&names)
            .require_fixture("same exact ZIP entries")?;
        let verified =
            verify_admitted_profile_zip_payloads(&archive, inventory)
                .require_fixture("both cryptographic hashes match")?;
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
        .require_fixture("metadata is still structurally valid")?;
        let changed_inventory = changed_admission
            .admit_entry_inventory(&names)
            .require_fixture("member names still match")?;
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
        ])?;
        let changed_archive = read_stored_profile_zip(&changed_bytes, limits())
            .require_fixture("CRC-correct replacement must still parse")?;
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
        Ok(())
    });
}

#[test]
fn verified_member_lookup_preserves_exact_unicode_and_prefixes() {
    run_checked_test(|| -> Result<(), String> {
        let entries = [
            ("assets/source.webp", b"x".as_slice(), 0x8cdc_1683),
            ENTRIES[0],
            ("sections/cuaderno \u{f1}.json", b"123456789", 0xcbf4_3926),
            ENTRIES[1],
        ];
        let bytes = make_zip(&entries)?;
        let archive = read_stored_profile_zip(&bytes, limits())
            .require_fixture("canonical UTF-8 member names")?;
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
                    path: "sections/cuaderno \u{f1}.json".into(),
                },
            ],
            optional_features: vec![],
            profile_identity: "unicode-lookup".into(),
            required_features: vec![],
        };
        let names = archive.observed_paths();
        let admission = atrament_handwriting_profile_container::
        validate_profile_manifest_view(&manifest, &[])
        .require_fixture("valid typed declaration")?;
        let inventory = admission
            .admit_entry_inventory(&names)
            .require_fixture("exact declared ZIP names")?;
        let verified =
            verify_admitted_profile_zip_payloads(&archive, inventory)
                .require_fixture("all member digests are valid")?;
        assert_eq!(verified.verified_entry_count(), 3);
        assert_eq!(
            verified.verified_entry_bytes("sections/cuaderno \u{f1}.json"),
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
            "manifest.json",
            "",
            "sections/cuaderno n.json",
            "sections/cuaderno \u{f1}.JSON",
            "sections/cuaderno \u{f1}",
            "sections/cuaderno %C3%B1.json",
            "sections/cuaderno \u{f1}.json/",
            "sections/strokes.json ",
            "sections/strokes.json/other",
            "assets/source.web",
            "assets/source.webp.bak",
        ] {
            assert_eq!(
                verified.verified_entry_bytes(near_miss),
                None,
                "exact name={near_miss:?}"
            );
        }
        Ok(())
    });
}

#[test]
fn empty_verified_asset_remains_distinct_from_absent_member() {
    run_checked_test(|| -> Result<(), String> {
        let members = [("assets/empty.webp", b"".as_slice(), 0u32), ENTRIES[0]];
        let bytes = make_zip(&members)?;
        let archive = read_stored_profile_zip(&bytes, limits())
            .require_fixture("empty stored asset has valid CRC-32")?;
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
        .require_fixture("valid zero-length declaration")?;
        let inventory = admission
            .admit_entry_inventory(&names)
            .require_fixture("root and empty asset agree")?;
        let verified =
            verify_admitted_profile_zip_payloads(&archive, inventory)
                .require_fixture("empty SHA-256 must match its declaration")?;
        assert_eq!(verified.verified_entry_count(), 1);
        assert_eq!(
            verified.verified_entry_bytes("assets/empty.webp"),
            Some(&b""[..]),
        );
        assert_eq!(verified.verified_entry_bytes("assets/absent.webp"), None);
        assert_eq!(verified.verified_entry_bytes("manifest.json"), None);
        Ok(())
    });
}

#[test]
fn later_member_length_mismatch_precedes_its_invalid_digest() {
    run_checked_test(|| -> Result<(), String> {
        let members = [
            ("assets/source.webp", b"x".as_slice(), 0x8cdc_1683),
            ENTRIES[0],
            ENTRIES[1],
        ];
        let bytes = make_zip(&members)?;
        let archive = read_stored_profile_zip(&bytes, limits())
            .require_fixture("CRC-correct stored ZIP")?;
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
        .require_fixture("valid manifest fields")?;
        let inventory = admission
            .admit_entry_inventory(&names)
            .require_fixture("exact ZIP names are admitted")?;
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
        Ok(())
    });
}

#[test]
fn root_json_syntax_is_checked_on_real_crc_valid_zip_bytes() {
    run_checked_test(|| -> Result<(), String> {
        let manifest = b"{\"entry\":1,\"items\":[true,null]}";
        let bytes = make_zip(&[("manifest.json", manifest, 0x3a80_dd76)])?;
        let archive = read_stored_profile_zip(&bytes, limits())
            .require_fixture("correct stored ZIP frame and CRC")?;
        let caps = ProfileJsonSyntaxLimits {
            maximum_bytes: manifest.len(),
            maximum_nesting: 3,
            maximum_object_members: 8,
            maximum_values: 5,
        };
        let valid = archive
            .manifest_json_object(caps)
            .require_fixture("JSON grammar succeeds")?;
        assert_eq!(valid.bytes(), manifest);
        assert_eq!(valid.value_count(), 5);
        assert_eq!(
            archive.manifest_json_object(ProfileJsonSyntaxLimits {
                maximum_values: 4,
                ..caps
            }),
            Err(ProfileZipManifestSyntaxError::Json(
                ProfileJsonSyntaxError::MaximumValuesExceeded,
            )),
        );
        assert_eq!(
            archive.manifest_json_object(ProfileJsonSyntaxLimits {
                maximum_bytes: manifest.len().saturating_sub(1),
                ..caps
            }),
            Err(ProfileZipManifestSyntaxError::Zip(
                ProfileZipReadError::ManifestTooLarge,
            )),
        );
        Ok(())
    });
}

#[test]
fn crc_valid_but_malformed_or_nonobject_root_is_not_admitted() {
    run_checked_test(|| -> Result<(), String> {
        let caps = ProfileJsonSyntaxLimits {
            maximum_bytes: 128,
            maximum_nesting: 8,
            maximum_object_members: 8,
            maximum_values: 16,
        };
        let fixtures = [
            (
                b"{\"entry\":}".as_slice(),
                0x37e4_7743,
                ProfileJsonSyntaxError::Malformed,
            ),
            (
                b"[]".as_slice(),
                0x0d4c_bb29,
                ProfileJsonSyntaxError::RootNotObject,
            ),
        ];
        for (root, crc, expected) in fixtures {
            let bytes = make_zip(&[("manifest.json", root, crc)])?;
            let archive = read_stored_profile_zip(&bytes, limits())
                .require_fixture("malformed JSON bytes have a valid ZIP CRC")?;
            assert_eq!(
                archive.manifest_json_object(caps),
                Err(ProfileZipManifestSyntaxError::Json(expected)),
            );
        }
        let missing_bytes = make_zip(&[])?;
        let missing = read_stored_profile_zip(&missing_bytes, limits())
            .require_fixture("empty ZIP framing is valid")?;
        assert_eq!(
            missing.manifest_json_object(caps),
            Err(ProfileZipManifestSyntaxError::Zip(
                ProfileZipReadError::MissingManifest,
            )),
        );
        Ok(())
    });
}

#[test]
fn crc_valid_manifest_syntax_preserves_utf8_failure() {
    run_checked_test(|| -> Result<(), String> {
        let invalid_utf8 = [123u8, 255u8, 125u8];
        let bytes = make_zip(&[("manifest.json", &invalid_utf8, 0x1a75_ae50)])?;
        let archive = read_stored_profile_zip(&bytes, limits())
            .require_fixture("the ZIP CRC covers the malformed UTF-8 bytes")?;
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
        Ok(())
    });
}

#[test]
fn crc_correct_zip_root_rejects_duplicate_decoded_json_keys() {
    run_checked_test(|| -> Result<(), String> {
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
            let bytes = make_zip(&[("manifest.json", root, crc32)])?;
            let archive = read_stored_profile_zip(&bytes, limits())
                .require_fixture(
                    "correct CRC-32 cannot prove key uniqueness",
                )?;
            assert_eq!(
                archive.manifest_json_object(caps),
                Err(ProfileZipManifestSyntaxError::Json(
                    ProfileJsonSyntaxError::DuplicateObjectKey,
                )),
            );
        }
        Ok(())
    });
}

#[test]
fn crc_correct_zip_root_enforces_caller_object_member_limit() {
    run_checked_test(|| -> Result<(), String> {
        let root = br#"{"a":1,"b":2,"c":3}"#;
        let bytes = make_zip(&[("manifest.json", root, 0x2ed4_5bab)])?;
        let archive = read_stored_profile_zip(&bytes, limits())
            .require_fixture("well-formed ZIP with three unique keys")?;
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
        let valid = archive
            .manifest_json_object(ProfileJsonSyntaxLimits {
                maximum_object_members: 3,
                ..caps
            })
            .require_fixture("exact number of unique members is admitted")?;
        assert_eq!(valid.bytes(), root);
        Ok(())
    });
}

#[test]
fn sealed_zip_root_lookup_borrows_raw_nested_json_values() {
    run_checked_test(|| -> Result<(), String> {
        let root = concat!(
            r#"{"header":{"tags":["x","y"]},"sample":"\u0061","#,
            r#""count":1e+1}"#,
        )
        .as_bytes();
        let bytes = make_zip(&[("manifest.json", root, 0x0eba_2758)])?;
        let archive = read_stored_profile_zip(&bytes, limits())
            .require_fixture(
                "root JSON bytes remain CRC-verifiable ZIP payload",
            )?;
        let caps = ProfileJsonSyntaxLimits {
            maximum_bytes: root.len(),
            maximum_nesting: 4,
            maximum_object_members: 5,
            maximum_values: 12,
        };
        let admitted = archive
            .manifest_json_object(caps)
            .require_fixture("the exact root JSON is unambiguous")?;
        assert_eq!(
            admitted.root_member_json("header"),
            Some(br#"{"tags":["x","y"]}"#.as_slice()),
        );
        assert_eq!(
            admitted.root_member_json("sample"),
            Some(br#""\u0061""#.as_slice()),
        );
        assert_eq!(
            admitted.root_member_json("count"),
            Some(b"1e+1".as_slice())
        );
        assert_eq!(admitted.root_member_json("tags"), None);
        assert_eq!(admitted.root_member_json("Sample"), None);
        Ok(())
    });
}
fn verify_zip_root_key_copy(admitted: &ValidatedProfileJsonObject<'_>) {
    run_checked_test(|| -> Result<(), String> {
        let root_fields = admitted.root_fields().collect::<Vec<_>>();
        assert_eq!(root_fields.len(), 2, "checked ZIP JSON source view");
        let first_root_field =
            root_fields.first().ok_or("missing root field")?;
        assert!(
            first_root_field.key_matches("nested"),
            "checked ZIP JSON source view"
        );
        assert_eq!(
            first_root_field.key_json(),
            br#""nested""#,
            "checked ZIP JSON source view"
        );
        let mut root_key = [0xceu8; 10];
        assert_eq!(
            first_root_field.copy_key_utf8(&mut root_key[..5]),
            Err(ProfileJsonStringCopyError::BufferTooSmall { required: 6 }),
            "checked ZIP JSON source view"
        );
        assert_eq!(root_key, [0xce; 10], "checked ZIP JSON source view");
        assert_eq!(
            first_root_field.copy_key_utf8(&mut root_key),
            Ok(6),
            "checked ZIP JSON source view"
        );
        assert_eq!(&root_key[..6], b"nested", "checked ZIP JSON source view");
        assert_eq!(
            &root_key[6..],
            &[0xce; 10][6..],
            "checked ZIP JSON source view"
        );
        Ok(())
    });
}

fn verify_zip_root_field_views(admitted: &ValidatedProfileJsonObject<'_>) {
    verify_zip_root_key_copy(admitted);
    run_checked_test(|| -> Result<(), String> {
        let root_fields = admitted.root_fields().collect::<Vec<_>>();
        let first_root_field =
            root_fields.first().ok_or("missing root field")?;
        assert!(
            root_fields
                .get(1)
                .ok_or("missing root field")?
                .key_matches("id"),
            "checked ZIP JSON source view"
        );
        assert_eq!(
            root_fields
                .get(1)
                .ok_or("missing root field")?
                .value()
                .unsigned_integer(),
            Some(3),
            "checked ZIP JSON source view"
        );
        assert_eq!(
            first_root_field.source_member_json(),
            br#""nested":{"rows":[{"id":1},[true,false],"\u0061"]}"#,
            "checked ZIP JSON source view"
        );
        assert_eq!(
            root_fields
                .get(1)
                .ok_or("missing root field")?
                .source_member_json(),
            br#""id":3"#,
            "checked ZIP JSON source view"
        );
        assert_eq!(
            admitted
                .root_field("nested")
                .require_fixture("source-backed root")?
                .source_member_json(),
            first_root_field.source_member_json(),
            "checked ZIP JSON source view"
        );
        assert_eq!(
            admitted
                .root_field("id")
                .require_fixture("root ID")?
                .value()
                .unsigned_integer(),
            Some(3),
            "checked ZIP JSON source view"
        );
        assert_eq!(
            admitted.root_field("ID"),
            None,
            "checked ZIP JSON source view"
        );
        assert!(
            admitted
                .bytes()
                .windows(first_root_field.source_member_json().len())
                .any(|candidate| ptr::eq(
                    candidate.as_ptr(),
                    first_root_field.source_member_json().as_ptr()
                )),
            "checked ZIP JSON source view"
        );
        assert!(
            admitted
                .bytes()
                .windows(first_root_field.key_json().len())
                .any(|slice| ptr::eq(
                    slice.as_ptr(),
                    first_root_field.key_json().as_ptr()
                )),
            "checked ZIP JSON source view"
        );
        Ok(())
    });
}

fn verify_zip_nested_value_views(admitted: &ValidatedProfileJsonObject<'_>) {
    run_checked_test(|| -> Result<(), String> {
        let nested = admitted
            .root_member_value("nested")
            .require_fixture("root object projection")?;
        assert_eq!(
            nested.kind(),
            Some(ProfileJsonValueKind::Object),
            "checked ZIP JSON source view"
        );
        let rows_field = nested
            .object_field("rows")
            .require_fixture("nested field")?;
        assert_eq!(
            rows_field.source_member_json(),
            br#""rows":[{"id":1},[true,false],"\u0061"]"#,
            "checked ZIP JSON source view"
        );
        let rows = nested
            .object_member("rows")
            .require_fixture("nested array")?;
        assert_eq!(
            rows_field.value().bytes(),
            rows.bytes(),
            "checked ZIP JSON source view"
        );
        let deep_path = [Key("nested"), Key("rows"), Index(1), Index(1)];
        let selected = admitted
            .value_at_path(&deep_path)
            .require_fixture("CRC-verified ZIP nested scalar")?;
        assert_eq!(selected.bytes(), b"false", "checked ZIP JSON source view");
        assert_eq!(
            selected.boolean(),
            Some(false),
            "checked ZIP JSON source view"
        );
        assert!(
            admitted
                .bytes()
                .windows(selected.bytes().len())
                .any(|segment| ptr::eq(
                    segment.as_ptr(),
                    selected.bytes().as_ptr()
                )),
            "checked ZIP JSON source view"
        );
        assert_eq!(
            admitted.value_at_path(&[Index(0)]),
            None,
            "checked ZIP JSON source view"
        );
        assert_eq!(
            admitted.value_at_path(&[Key("nested"), Index(0)]),
            None,
            "checked ZIP JSON source view"
        );
        assert_eq!(
            admitted.value_at_path(&[
                Key("nested"),
                Key("rows"),
                Index(usize::MAX)
            ]),
            None,
            "checked ZIP JSON source view"
        );
        Ok(())
    });
    verify_zip_nested_row_values(admitted);
    verify_zip_nested_escaped_values(admitted);
}

fn verify_zip_nested_row_values(admitted: &ValidatedProfileJsonObject<'_>) {
    run_checked_test(|| -> Result<(), String> {
        let nested = admitted
            .root_member_value("nested")
            .require_fixture("root object projection")?;
        let rows = nested
            .object_member("rows")
            .require_fixture("nested array")?;
        let first_row = rows.array_element(0).require_fixture("object row")?;
        assert_eq!(
            first_row
                .object_member("id")
                .require_fixture("integer row value")?
                .bytes(),
            b"1",
            "checked ZIP JSON source view"
        );
        let second_row =
            rows.array_element(1).require_fixture("nested array")?;
        assert_eq!(
            second_row
                .array_element(0)
                .require_fixture("first boolean")?
                .bytes(),
            b"true",
            "checked ZIP JSON source view"
        );
        assert_eq!(
            second_row
                .array_element(1)
                .require_fixture("second boolean")?
                .bytes(),
            b"false",
            "checked ZIP JSON source view"
        );
        assert_eq!(
            second_row.array_element(2),
            None,
            "checked ZIP JSON source view"
        );
        assert_eq!(
            second_row
                .array_element(0)
                .require_fixture("true")?
                .boolean(),
            Some(true),
            "checked ZIP JSON source view"
        );
        assert_eq!(
            second_row
                .array_element(1)
                .require_fixture("false")?
                .boolean(),
            Some(false),
            "checked ZIP JSON source view"
        );
        Ok(())
    });
}

fn verify_zip_nested_escaped_values(admitted: &ValidatedProfileJsonObject<'_>) {
    run_checked_test(|| -> Result<(), String> {
        let nested = admitted
            .root_member_value("nested")
            .require_fixture("root object projection")?;
        let rows = nested
            .object_member("rows")
            .require_fixture("nested array")?;
        let first_row = rows.array_element(0).require_fixture("object row")?;
        let escaped =
            rows.array_element(2).require_fixture("escaped string")?;
        let mut empty = [];
        assert_eq!(
            escaped.copy_string_utf8(&mut empty),
            Err(ProfileJsonStringCopyError::BufferTooSmall { required: 1 }),
            "checked ZIP JSON source view"
        );
        let mut output = [0xccu8; 8];
        assert_eq!(
            escaped.copy_string_utf8(&mut output),
            Ok(1),
            "checked ZIP JSON source view"
        );
        assert_eq!(&output[..1], b"a", "checked ZIP JSON source view");
        assert_eq!(
            &output[1..],
            &[0xcc; 8][1..],
            "checked ZIP JSON source view"
        );
        assert_eq!(
            escaped.bytes(),
            br#""\u0061""#,
            "checked ZIP JSON source view"
        );
        assert_eq!(
            escaped
                .string_chars()
                .require_fixture("Unicode string")?
                .collect::<String>(),
            "a",
            "checked ZIP JSON source view"
        );
        assert_eq!(rows.array_element(3), None, "checked ZIP JSON source view");
        assert_eq!(
            nested.object_member("id"),
            None,
            "checked ZIP JSON source view"
        );
        let root_id = admitted
            .root_member_value("id")
            .require_fixture("root id")?;
        assert_eq!(root_id.bytes(), b"3", "checked ZIP JSON source view");
        assert_eq!(
            root_id.unsigned_integer(),
            Some(3),
            "checked ZIP JSON source view"
        );
        assert_eq!(
            admitted.root_member_value("\u{0072}ows"),
            None,
            "checked ZIP JSON source view"
        );
        let borrowed_row_value = first_row
            .object_member("id")
            .require_fixture("borrowed value")?
            .bytes();
        assert!(
            admitted.bytes().windows(borrowed_row_value.len()).any(
                |candidate| ptr::eq(
                    candidate.as_ptr(),
                    borrowed_row_value.as_ptr()
                )
            ),
            "checked ZIP JSON source view"
        );
        Ok(())
    });
}

#[test]
fn sealed_crc_checked_zip_root_exposes_nested_read_only_json_values() {
    run_checked_test(|| -> Result<(), String> {
        let root =
            br#"{"nested":{"rows":[{"id":1},[true,false],"\u0061"]},"id":3}"#;
        let bytes = make_zip(&[("manifest.json", root, 0xfb5c_b798)])?;
        let archive = read_stored_profile_zip(&bytes, limits())
            .require_fixture("ZIP payload CRC and framing are correct")?;
        let admitted = archive
            .manifest_json_object(ProfileJsonSyntaxLimits {
                maximum_bytes: root.len(),
                maximum_nesting: 4,
                maximum_object_members: 3,
                maximum_values: 12,
            })
            .require_fixture(
                "ZIP root grammar and unique object keys verified",
            )?;
        verify_zip_root_field_views(&admitted);
        verify_zip_nested_value_views(&admitted);
        Ok(())
    });
}

#[test]
fn sorted_zip_manifest_lookup_selects_only_exact_middle_root_name() {
    run_checked_test(|| -> Result<(), String> {
        let root = br#"{"id":42}"#;
        let entries: [(&str, &[u8], u32); 3] = [
            ("assets/notes.bin", b"data", 0xadf3_f363),
            ("manifest.json", root, 0x66d6_18ea),
            ("sections/notes.json", b"{}", 0xa3a6_bf43),
        ];
        let bytes = make_zip(&entries)?;
        let archive = read_stored_profile_zip(&bytes, limits())
            .require_fixture(
                "canonical ZIP with root in middle of sorted index",
            )?;
        assert_eq!(archive.entries().len(), 3);
        let manifest = archive.manifest_utf8(root.len()).require_fixture(
            "only exact root filename has manifest authority",
        )?;
        assert_eq!(manifest.as_bytes(), root);
        assert_eq!(
            archive.manifest_utf8(root.len() - 1),
            Err(ProfileZipReadError::ManifestTooLarge)
        );
        let admitted = archive
            .manifest_json_object(ProfileJsonSyntaxLimits {
                maximum_bytes: root.len(),
                maximum_nesting: 2,
                maximum_object_members: 2,
                maximum_values: 3,
            })
            .require_fixture(
                "the root JSON, not any adjacent member, is admitted",
            )?;
        assert_eq!(
            admitted
                .root_member_value("id")
                .require_fixture("id")?
                .unsigned_integer(),
            Some(42)
        );
        let value = admitted
            .root_field("id")
            .require_fixture("root field")?
            .source_member_json();
        assert_eq!(value, br#""id":42"#);
        assert!(
            archive
                .entries()
                .get(1)
                .ok_or("missing ZIP entry")?
                .bytes()
                .windows(value.len())
                .any(|part| ptr::eq(part.as_ptr(), value.as_ptr()))
        );
        Ok(())
    });
}

#[test]
fn nearby_nested_archive_names_cannot_substitute_for_missing_root() {
    run_checked_test(|| -> Result<(), String> {
        let entries: [(&str, &[u8], u32); 2] = [
            ("assets/manifest.json", br#"{"spoof":true}"#, 0xeb29_ac2c),
            ("sections/manifest.json", b"{}", 0xa3a6_bf43),
        ];
        let bytes = make_zip(&entries)?;
        let archive = read_stored_profile_zip(&bytes, limits())
            .require_fixture(
                "both near-matching paths have valid ZIP framing",
            )?;
        assert_eq!(
            archive.manifest_utf8(1024),
            Err(ProfileZipReadError::MissingManifest)
        );
        assert_eq!(
            archive.manifest_json_object(ProfileJsonSyntaxLimits {
                maximum_bytes: 1_024,
                maximum_nesting: 3,
                maximum_object_members: 5,
                maximum_values: 10,
            }),
            Err(ProfileZipManifestSyntaxError::Zip(
                ProfileZipReadError::MissingManifest
            ))
        );
        Ok(())
    });
}

#[test]
fn sealed_zip_root_json_proof_carries_exact_supplying_archive_identity() {
    run_checked_test(|| -> Result<(), String> {
        let first_bytes = make_zip(&[
            ("assets/notes.bin", b"data", 0xadf3_f363),
            ("manifest.json", br#"{"id":42}"#, 0x66d6_18ea),
            ("sections/notes.json", b"{}", 0xa3a6_bf43),
        ])?;
        let second_bytes = make_zip(&[
            ("assets/notes.bin", b"data", 0xadf3_f363),
            ("manifest.json", br#"{"id":0}"#, 0x5d46_c984),
            ("sections/notes.json", b"{}", 0xa3a6_bf43),
        ])?;
        let first_archive = read_stored_profile_zip(&first_bytes, limits())
            .require_fixture("first CRC-checked archive")?;
        let second_archive = read_stored_profile_zip(&second_bytes, limits())
            .require_fixture(
            "second CRC-checked archive with same entry names",
        )?;
        let bounds = ProfileJsonSyntaxLimits {
            maximum_bytes: 16,
            maximum_nesting: 2,
            maximum_object_members: 2,
            maximum_values: 2,
        };
        let first = first_archive
            .admitted_root_json(bounds)
            .require_fixture("first actual ZIP root admitted")?;
        let second = second_archive
            .admitted_root_json(bounds)
            .require_fixture("second actual ZIP root admitted")?;
        assert!(ptr::eq(first.archive(), &raw const first_archive));
        assert!(ptr::eq(second.archive(), &raw const second_archive));
        assert!(!ptr::eq(first.archive(), second.archive()));
        assert_eq!(first.root().bytes(), br#"{"id":42}"#);
        assert_eq!(second.root().bytes(), br#"{"id":0}"#);
        assert_eq!(first.root().value_count(), 2);
        assert_eq!(second.root().value_count(), 2);
        assert_eq!(
            first
                .root()
                .root_member_value("id")
                .require_fixture("first id")?
                .unsigned_integer(),
            Some(42)
        );
        assert_eq!(
            second
                .root()
                .root_member_value("id")
                .require_fixture("second id")?
                .unsigned_integer(),
            Some(0)
        );
        for bound in [&first, &second] {
            let manifest = bound
                .archive()
                .entries()
                .iter()
                .find(|entry| entry.name() == "manifest.json")
                .require_fixture("the same root member is in this archive")?;
            assert_eq!(bound.root().bytes().len(), manifest.bytes().len());
            assert!(ptr::eq(
                bound.root().bytes().as_ptr(),
                manifest.bytes().as_ptr()
            ));
        }
        assert_eq!(
            first_archive.manifest_json_object(bounds),
            Ok(first.root()),
            "legacy syntax query agrees with sealed pairing"
        );
        Ok(())
    });
}

#[test]
fn zipped_root_provenance_proof_fails_closed_on_json_syntax_and_budget() {
    run_checked_test(|| -> Result<(), String> {
        let bytes =
            make_zip(&[("manifest.json", br#"{"broken":]"#, 0xff90_525f)])?;
        let archive = read_stored_profile_zip(&bytes, limits())
            .require_fixture("valid framing and correct CRC but bad JSON")?;
        let generous = ProfileJsonSyntaxLimits {
            maximum_bytes: 1024,
            maximum_nesting: 10,
            maximum_object_members: 10,
            maximum_values: 10,
        };
        assert_eq!(
            archive.admitted_root_json(generous).err(),
            Some(ProfileZipManifestSyntaxError::Json(
                ProfileJsonSyntaxError::Malformed
            ))
        );
        assert_eq!(
            archive
                .admitted_root_json(ProfileJsonSyntaxLimits {
                    maximum_bytes: 4,
                    ..generous
                })
                .err(),
            Some(ProfileZipManifestSyntaxError::Zip(
                ProfileZipReadError::ManifestTooLarge
            ))
        );
        Ok(())
    });
}

#[test]
fn crc_verified_zip_member_streams_to_exact_sha256_evidence() {
    run_checked_test(|| -> Result<(), String> {
        let bytes = make_zip(&ENTRIES)?;
        let archive = read_stored_profile_zip(&bytes, limits())
            .require_fixture("canonical CRC-valid stored archive")?;
        let payload = archive
            .entries()
            .iter()
            .find(|entry| entry.name() == "sections/strokes.json")
            .require_fixture("declared stored member")?
            .bytes();
        assert_eq!(payload, b"123456789");
        let declaration = ProfileManifestEntry {
            byte_length: 9,
            digest: sha256_profile_bytes(payload),
            media_type: "application/json".into(),
            path: "sections/strokes.json".into(),
        };
        let reference = verify_profile_entry_bytes(&declaration, payload)
            .require_fixture("source-backed member matches declaration")?;
        for stride in [1usize, 2, 3, 4, 5, 8, 9, 64] {
            assert_eq!(
                verify_profile_entry_chunks(
                    &declaration,
                    payload.chunks(stride)
                ),
                Ok(reference)
            );
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
        ])?;
        let damaged_archive = read_stored_profile_zip(&damaged_bytes, limits())
            .require_fixture(
                "corrupted content with matching updated CRC still passes ZIP",
            )?;
        let damaged_payload = damaged_archive
            .entries()
            .iter()
            .find(|entry| entry.name() == "sections/strokes.json")
            .require_fixture("same declared path but altered bytes")?
            .bytes();
        assert_eq!(verify_profile_entry_chunks(&declaration,
        damaged_payload.chunks(2)),
        Err(ProfileChunkVerificationError::Entry(
            atrament_handwriting_profile_container::
                ProfileEntryVerificationError::DigestMismatch,
        )));
        Ok(())
    });
}
#[test]
fn sealed_verified_member_chunks_borrow_exact_crc_and_sha_checked_source() {
    run_checked_test(|| -> Result<(), String> {
        let archive_bytes = make_zip(&ENTRIES)?;
        let archive = read_stored_profile_zip(&archive_bytes, limits())
            .require_fixture("CRC-checked ZIP root and member")?;
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
        .require_fixture("typed declaration shape validated")?;
        let inventory = admitted
            .admit_entry_inventory(&names)
            .require_fixture("exact member inventory admitted")?;
        let verified =
            verify_admitted_profile_zip_payloads(&archive, inventory)
                .require_fixture(
                    "every declared member verified before exposing bytes",
                )?;
        let original = verified
            .verified_entry_bytes("sections/strokes.json")
            .require_fixture("verified exact member exists")?;
        assert_eq!(original, b"123456789");
        for chunk_size in [1usize, 2, 3, 4, 5, 8, 9, 10, 32] {
            let size = NonZeroUsize::new(chunk_size)
                .require_fixture("nonzero size")?;
            let chunks = verified
                .verified_entry_chunks("sections/strokes.json", size)
                .require_fixture("same verified member")?;
            let mut observed = Vec::new();
            for (index, chunk) in chunks.enumerate() {
                observed.extend_from_slice(chunk);
                let start = index.saturating_mul(chunk_size);
                assert!(ptr::eq(
                    chunk.as_ptr(),
                    original.get(start..).ok_or("chunk start")?.as_ptr()
                ));
                assert_ne!(chunk.len(), 0);
                assert!(chunk.len() <= chunk_size);
            }
            assert_eq!(observed, original, "chunk_size={chunk_size}");
        }
        let single = NonZeroUsize::new(1).require_fixture("nonzero size")?;
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
        Ok(())
    });
}

#[test]
fn sealed_verified_empty_asset_produces_zero_chunks_not_missing() {
    run_checked_test(|| -> Result<(), String> {
        let members = [("assets/empty.webp", b"".as_slice(), 0u32), ENTRIES[0]];
        let bytes = make_zip(&members)?;
        let archive = read_stored_profile_zip(&bytes, limits())
            .require_fixture("root plus CRC-valid empty asset")?;
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
        .require_fixture("valid declaration")?;
        let inventory = admission
            .admit_entry_inventory(&names)
            .require_fixture("root and declared asset match")?;
        let verified =
            verify_admitted_profile_zip_payloads(&archive, inventory)
                .require_fixture("empty member digest verified")?;
        let size = NonZeroUsize::new(17).require_fixture("nonzero size")?;
        let chunks = verified
            .verified_entry_chunks("assets/empty.webp", size)
            .require_fixture("present empty member is not missing")?;
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
        Ok(())
    });
}

#[test]
fn actual_crc_checked_zip_member_matches_bounded_reader_sha256_evidence() {
    run_checked_test(|| -> Result<(), String> {
        let archived = make_zip(&ENTRIES)?;
        let zip = read_stored_profile_zip(&archived, limits())
            .require_fixture(
                "stored ZIP passes local/central and CRC checks",
            )?;
        let bytes = zip
            .entries()
            .iter()
            .find(|entry| entry.name() == "sections/strokes.json")
            .require_fixture("stored member present")?
            .bytes();
        let manifest_entry = ProfileManifestEntry {
            byte_length: 9,
            digest: sha256_profile_bytes(b"123456789"),
            media_type: "application/json".into(),
            path: "sections/strokes.json".into(),
        };
        let mut source = Cursor::new(bytes);
        let evidence =
            verify_profile_entry_from_reader(&mut source, &manifest_entry, 9)
                .require_fixture("preopened CRC member has correct SHA")?;
        assert_eq!(evidence.byte_length, 9);
        assert_eq!(evidence.digest, manifest_entry.digest);
        assert_eq!(source.position(), 9);
        assert!(ptr::eq(source.get_ref().as_ptr(), bytes.as_ptr()));

        let tampered_zip = make_zip(&[
            ENTRIES[0],
            ("sections/strokes.json", b"123456788", 0xbcf3_09b0),
        ])?;
        let changed = read_stored_profile_zip(&tampered_zip, limits())
            .require_fixture("updated CRC still passes weak ZIP integrity")?;
        let modified = changed
            .entries()
            .iter()
            .find(|entry| entry.name() == "sections/strokes.json")
            .require_fixture("stored member present")?
            .bytes();
        let mut modified_source = Cursor::new(modified);
        let observed = verify_profile_entry_from_reader(
            &mut modified_source,
            &manifest_entry,
            9,
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
        Ok(())
    });
}
#[test]
fn admitted_zip_root_fingerprint_preserves_exact_json_source_whitespace() {
    run_checked_test(|| -> Result<(), String> {
        // Both input SHA-256 digests and ZIP CRC32 values were independently
        // obtained from Python hashlib and zlib; the scalar values are equal.
        let cases = [
            (
                br#"{"a":1}"#.as_slice(),
                0x561b_acaf,
                concat!(
                    "015abd7f5cc57a2dd94b7590f04ad808",
                    "4273905ee33ec5cebeae62276a97f862"
                ),
            ),
            (
                br#"{ "a" : 1 }"#.as_slice(),
                0xc32b_e995,
                concat!(
                    "c94841c938b16d8b21ccfcd43131ec60",
                    "7e39782e486250bcd72341c7a3af0e44"
                ),
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
            let input = make_zip(&[("manifest.json", root, crc)])?;
            let archive = read_stored_profile_zip(&input, limits())
                .require_fixture("CRC-correct root framing")?;
            let admitted = archive
                .admitted_root_json(caps)
                .require_fixture("exact root JSON grammar admitted")?;
            assert_eq!(admitted.root().bytes(), root);
            assert_eq!(
                admitted
                    .root()
                    .root_member_value("a")
                    .require_fixture("same semantic integer")?
                    .unsigned_integer(),
                Some(1)
            );
            let digest = admitted.source_sha256();
            let hex = hex_digest(&digest.bytes())?;
            assert_eq!(hex, golden_hex);
            assert_eq!(digest, sha256_profile_bytes(root));
            hashes.push(digest);
        }
        assert_ne!(
            hashes.first().ok_or("missing hash witness")?,
            hashes.get(1).ok_or("missing hash witness")?,
            "byte-distinct syntax sources never silently canonicalize"
        );
        Ok(())
    });
}

#[test]
fn admitted_root_fingerprint_does_not_cover_unrelated_zip_members() {
    run_checked_test(|| -> Result<(), String> {
        let root = br#"{"a":1}"#;
        let original = make_zip(&[
            ("assets/mark.webp", b"a", 0xe8b7_be43),
            ("manifest.json", root, 0x561b_acaf),
        ])?;
        let changed = make_zip(&[
            ("assets/mark.webp", b"b", 0x71be_eff9),
            ("manifest.json", root, 0x561b_acaf),
        ])?;
        let bounds = ProfileJsonSyntaxLimits {
            maximum_bytes: root.len(),
            maximum_nesting: 3,
            maximum_object_members: 3,
            maximum_values: 3,
        };
        assert_ne!(original, changed);
        let first = read_stored_profile_zip(&original, limits())
            .require_fixture("first CRC-valid ZIP")?;
        let second = read_stored_profile_zip(&changed, limits())
            .require_fixture("changed CRC-valid ZIP")?;
        let first_root = first
            .admitted_root_json(bounds)
            .require_fixture("source-root admission")?;
        let second_root = second
            .admitted_root_json(bounds)
            .require_fixture("source-root admission")?;
        assert_eq!(first_root.source_sha256(), second_root.source_sha256());
        assert_eq!(first_root.source_sha256(), sha256_profile_bytes(root));
        assert_ne!(
            first.entries().first().ok_or("missing ZIP entry")?.bytes(),
            second.entries().first().ok_or("missing ZIP entry")?.bytes()
        );
        assert!(ptr::eq(
            first_root.root().bytes().as_ptr(),
            first
                .entries()
                .get(1)
                .ok_or("missing ZIP entry")?
                .bytes()
                .as_ptr()
        ));
        assert!(ptr::eq(
            second_root.root().bytes().as_ptr(),
            second
                .entries()
                .get(1)
                .ok_or("missing ZIP entry")?
                .bytes()
                .as_ptr()
        ));
        Ok(())
    });
}

#[test]
fn checked_zip_archive_keeps_full_source_and_python_hashlib_fingerprint() {
    run_checked_test(|| -> Result<(), String> {
        // Python hashlib.sha256(struct/zlib-assembled root-only stored ZIP)
        // produces 69635ab9...171c394 over exactly 126 bytes.
        let source = make_zip(&[("manifest.json", b"{}", 0xa3a6_bf43)])?;
        assert_eq!(source.len(), 126);
        let archive = read_stored_profile_zip(&source, limits())
            .require_fixture("strict CRC-correct ZIP framing")?;
        assert_eq!(archive.source_bytes(), source);
        assert!(ptr::eq(archive.source_bytes().as_ptr(), source.as_ptr()));
        assert_eq!(archive.source_sha256(), sha256_profile_bytes(&source));
        let hex = hex_digest(&archive.source_sha256().bytes())?;
        assert_eq!(
            hex,
            "69635ab9c8b0a4dfc5a4e6743607342a0a4e4d6a50d23ac79b0ebe41e171c394"
        );
        Ok(())
    });
}

#[test]
fn complete_zip_hash_changes_when_only_a_nonroot_member_changes() {
    run_checked_test(|| -> Result<(), String> {
        let root = br#"{"a":1}"#;
        let first_bytes = make_zip(&[
            ("assets/mark.webp", b"a", 0xe8b7_be43),
            ("manifest.json", root, 0x561b_acaf),
        ])?;
        let second_bytes = make_zip(&[
            ("assets/mark.webp", b"b", 0x71be_eff9),
            ("manifest.json", root, 0x561b_acaf),
        ])?;
        let first = read_stored_profile_zip(&first_bytes, limits())
            .require_fixture("first full ZIP admitted")?;
        let second = read_stored_profile_zip(&second_bytes, limits())
            .require_fixture("second full ZIP admitted")?;
        assert_ne!(first.source_bytes(), second.source_bytes());
        assert_ne!(first.source_sha256(), second.source_sha256());
        let caps = ProfileJsonSyntaxLimits {
            maximum_bytes: root.len(),
            maximum_nesting: 3,
            maximum_object_members: 3,
            maximum_values: 3,
        };
        assert_eq!(
            first
                .admitted_root_json(caps)
                .require_fixture("valid first JSON")?
                .source_sha256(),
            second
                .admitted_root_json(caps)
                .require_fixture("valid second JSON")?
                .source_sha256()
        );
        Ok(())
    });
}

#[test]
fn structurally_checked_archive_hash_is_not_root_syntax_acceptance() {
    run_checked_test(|| -> Result<(), String> {
        let malformed_root = br#"{"bad":]"#;
        // A CRC-valid ZIP can still contain a malformed JSON root.
        let source =
            make_zip(&[("manifest.json", malformed_root, 0x92aa_8b5c)])?;
        let archive = read_stored_profile_zip(&source, limits())
            .require_fixture("only stored framing and CRC accepted")?;
        assert!(
            archive.matches_stored_writer_framing(),
            "matching envelope never guarantees valid JSON"
        );
        assert_eq!(archive.source_sha256(), sha256_profile_bytes(&source));
        assert_eq!(
            archive
                .admitted_root_json(ProfileJsonSyntaxLimits {
                    maximum_bytes: 40,
                    maximum_nesting: 3,
                    maximum_object_members: 3,
                    maximum_values: 3,
                })
                .err(),
            Some(ProfileZipManifestSyntaxError::Json(
                ProfileJsonSyntaxError::Malformed
            ))
        );
        Ok(())
    });
}

#[test]
fn crc32_collision_cannot_hide_changed_archive_sha256_source() {
    run_checked_test(|| -> Result<(), String> {
        // Two independently generated 8-byte sources collide under Python zlib:
        // CRC32 = 0x8b69ccdd for both, but their SHA-256 must remain distinct.
        let first_asset: [u8; 8] =
            [0x04, 0x2e, 0x53, 0x5d, 0x01, 0xc1, 0x96, 0x7a];
        let second_asset: [u8; 8] =
            [0xfa, 0xed, 0x87, 0xb3, 0xe8, 0x47, 0x20, 0x2d];
        let first = make_zip(&[
            ("assets/mark.webp", &first_asset, 0x8b69_ccdd),
            ("manifest.json", b"{}", 0xa3a6_bf43),
        ])?;
        let second = make_zip(&[
            ("assets/mark.webp", &second_asset, 0x8b69_ccdd),
            ("manifest.json", b"{}", 0xa3a6_bf43),
        ])?;
        assert_ne!(first, second);
        assert_eq!(first.len(), second.len());
        let first_archive = read_stored_profile_zip(&first, limits())
            .require_fixture("first CRC-correct collision frame")?;
        let second_archive = read_stored_profile_zip(&second, limits())
            .require_fixture("second CRC-correct collision frame")?;
        assert_eq!(
            first_archive
                .entries()
                .first()
                .ok_or("missing ZIP entry")?
                .crc32(),
            0x8b69_ccdd
        );
        assert_eq!(
            second_archive
                .entries()
                .first()
                .ok_or("missing ZIP entry")?
                .crc32(),
            0x8b69_ccdd
        );
        assert_ne!(
            first_archive
                .entries()
                .first()
                .ok_or("missing ZIP entry")?
                .bytes(),
            second_archive
                .entries()
                .first()
                .ok_or("missing ZIP entry")?
                .bytes()
        );
        assert_ne!(
            first_archive.source_sha256(),
            second_archive.source_sha256()
        );
        let caps = ProfileJsonSyntaxLimits {
            maximum_bytes: 2,
            maximum_nesting: 1,
            maximum_object_members: 1,
            maximum_values: 1,
        };
        assert_eq!(
            first_archive
                .admitted_root_json(caps)
                .require_fixture("same root")?
                .source_sha256(),
            second_archive
                .admitted_root_json(caps)
                .require_fixture("same root")?
                .source_sha256()
        );
        Ok(())
    });
}

fn verify_stress_mutation_admitted_source(
    archive: &ProfileZipArchive<'_>,
    bytes: &[u8],
    declared: &[(&str, &[u8], u32)],
    operation: usize,
    case: usize,
) {
    run_checked_test(|| -> Result<(), String> {
        assert_eq!(
            archive.entries().len(),
            declared.len(),
            "admitted mutation operation={operation} case={case}"
        );
        for (member, (path, data, crc)) in
            archive.entries().iter().zip(declared.iter().copied())
        {
            assert_eq!(
                member.name(),
                path,
                "accepted ZIP stress mutation identity"
            );
            assert_eq!(
                member.bytes(),
                data,
                "accepted ZIP stress mutation identity"
            );
            assert_eq!(
                member.crc32(),
                crc,
                "accepted ZIP stress mutation identity"
            );
        }
        assert_eq!(
            archive.source_bytes(),
            bytes,
            "accepted ZIP stress mutation identity"
        );
        assert_eq!(
            archive.source_sha256(),
            sha256_profile_bytes(bytes),
            "accepted ZIP stress mutation identity"
        );
        let admitted_root = archive
            .admitted_root_json(ProfileJsonSyntaxLimits {
                maximum_bytes: 64,
                maximum_nesting: 3,
                maximum_object_members: 4,
                maximum_values: 4,
            })
            .require_fixture(
                "mutated framing cannot substitute malformed root",
            )?;
        assert_eq!(
            admitted_root.root().bytes(),
            declared.get(1).ok_or("missing declared ZIP root")?.1,
            "accepted ZIP stress mutation identity"
        );
        Ok(())
    });
}

#[test]
fn seeded_zip_mutations_preserve_exact_member_identity_on_any_valid_read() {
    run_checked_test(|| -> Result<(), String> {
        let declared = [
            ("assets/mark.webp", b"\0\xff\x11B".as_slice(), 0x54d7_e85d),
            ("manifest.json", br#"{"id":1}"#.as_slice(), 0x445d_f8c5),
            ("sections/sample.json", b"[1,2,3]".as_slice(), 0xfd6d_b3e9),
        ];
        let original = make_zip(&declared)?;
        let canonical = read_stored_profile_zip(&original, limits())
            .require_fixture(
                "independent zlib CRC goldens form a valid source archive",
            )?;
        assert_eq!(canonical.entries().len(), 3);
        let mut seed = 0x4154_5241_4d45_4e54u64;
        let mut attempted = [0usize; 4];
        let mut admitted = 0usize;
        let mut rejected = 0usize;
        let mut failure_classes = Vec::new();
        let mut touched = [false; 3];
        for case in 0usize..8_192 {
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let length = u64::try_from(original.len())
                .require_fixture("source length for random position")?;
            let position = usize::try_from(
                (seed >> 20).checked_rem(length).ok_or("position divisor")?,
            )
            .require_fixture("bounded mutation position fits usize")?;
            let replacement = u8::try_from((seed >> 40u32) & 0xffu64)
                .require_fixture("eight-bit replacement")?;
            let operation = case
                .checked_rem(4usize)
                .ok_or("mutation operation divisor")?;
            *attempted
                .get_mut(operation)
                .ok_or("attempted mutation index")? += 1;
            let bucket = position
                .checked_mul(3)
                .ok_or("mutation bucket multiplication")?
                .checked_div(original.len())
                .ok_or("mutation bucket division")?;
            *touched.get_mut(bucket).ok_or("mutation bucket range")? = true;
            let mut bytes = original.clone();
            match operation {
                0 => {
                    *bytes
                        .get_mut(position)
                        .ok_or("bytes mutation index")? ^= 1u8
                        << (replacement
                            .checked_rem(8u8)
                            .ok_or("bit index divisor")?);
                },
                1 => bytes.insert(position, replacement),
                2 => {
                    let _removed = bytes.remove(position);
                },
                _ => {
                    let copied =
                        *bytes.get(position).ok_or("bytes mutation index")?;
                    bytes.insert(position, copied);
                },
            }
            let checked = panic::catch_unwind(|| {
                read_stored_profile_zip(&bytes, limits())
            })
            .require_fixture("untrusted ZIP mutation must not panic")?;
            if let Ok(archive) = checked {
                admitted += 1;
                verify_stress_mutation_admitted_source(
                    &archive, &bytes, &declared, operation, case,
                );
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
        assert!(
            failure_classes.contains(&ProfileZipReadError::InvalidStructure)
        );
        assert!(
            failure_classes.len() >= 4,
            "multiple independent framing failures must be exercised"
        );
        Ok(())
    });
}

#[test]
fn accepted_noncanonical_zip_headers_do_not_claim_fixed_writer_metadata() {
    run_checked_test(|| -> Result<(), String> {
        let input = make_zip(&[("manifest.json", b"{}", 0xa3a6_bf43)])?;
        let baseline = read_stored_profile_zip(&input, limits())
            .require_fixture("fixed writer-like archive is valid")?;
        assert!(baseline.matches_fixed_writer_metadata());
        assert!(baseline.matches_stored_writer_framing());
        let central = central_start(&input)?;
        for (case, local_field, central_field, bytes) in [
            ("creator version", None, 4, [10, 0]),
            ("extractor version", Some(4), 6, [10, 0]),
            ("ASCII UTF-8 flag", Some(6), 8, [0, 0]),
            ("DOS modified time", Some(10), 12, [1, 0]),
            ("DOS modified date", Some(12), 14, [1, 0]),
        ] {
            let mut changed = input.clone();
            if let Some(local) = local_field {
                changed
                    .get_mut(local..local + 2)
                    .ok_or("local flag field")?
                    .copy_from_slice(&bytes);
            }
            changed
                .get_mut(central + central_field..central + central_field + 2)
                .ok_or("central flag field")?
                .copy_from_slice(&bytes);
            let archive = read_stored_profile_zip(&changed, limits())
                .require_fixture("legacy stored metadata remains readable")?;
            assert!(!archive.matches_fixed_writer_metadata(), "{case}");
            assert!(!archive.matches_stored_writer_framing(), "{case}");
            assert_eq!(
                archive.entries().first().ok_or("missing ZIP entry")?.name(),
                "manifest.json",
                "{case}"
            );
            assert_eq!(
                archive
                    .entries()
                    .first()
                    .ok_or("missing ZIP entry")?
                    .bytes(),
                b"{}",
                "{case}"
            );
            assert_eq!(
                archive
                    .entries()
                    .first()
                    .ok_or("missing ZIP entry")?
                    .crc32(),
                0xa3a6_bf43,
                "{case}"
            );
            assert_ne!(
                archive.source_sha256(),
                baseline.source_sha256(),
                "raw archive metadata still contributes to source identity"
            );
            assert_eq!(
                archive
                    .admitted_root_json(ProfileJsonSyntaxLimits {
                        maximum_bytes: 2,
                        maximum_nesting: 1,
                        maximum_object_members: 1,
                        maximum_values: 1,
                    })
                    .require_fixture("valid root syntax")?
                    .source_sha256(),
                sha256_profile_bytes(b"{}"),
                "{case}"
            );
        }
        Ok(())
    });
}

#[test]
fn fixed_writer_metadata_requires_every_header_not_just_the_root() {
    run_checked_test(|| -> Result<(), String> {
        let input = make_zip(&[
            ("assets/mark.webp", b"a", 0xe8b7_be43),
            ("manifest.json", b"{}", 0xa3a6_bf43),
            ("sections/ink.json", b"{}", 0xa3a6_bf43),
        ])?;
        let baseline = read_stored_profile_zip(&input, limits())
            .require_fixture("three-member fixed metadata ZIP")?;
        assert!(baseline.matches_fixed_writer_metadata());
        assert!(baseline.matches_stored_writer_framing());
        let central = central_start(&input)?;
        let names = ["assets/mark.webp", "manifest.json", "sections/ink.json"];
        let mut local = 0usize;
        let mut directory = central;
        for (index, name) in names.into_iter().enumerate() {
            let mut changed = input.clone();
            changed
                .get_mut(local + 10..local + 12)
                .ok_or("local method field")?
                .copy_from_slice(&[3, 0]);
            changed
                .get_mut(directory + 12..directory + 14)
                .ok_or("central method field")?
                .copy_from_slice(&[3, 0]);
            let archive = read_stored_profile_zip(&changed, limits())
                .require_fixture("metadata agreement still parses")?;
            assert!(
                !archive.matches_fixed_writer_metadata(),
                "noncanonical member index {index}"
            );
            assert_eq!(archive.entries(), baseline.entries());
            local += 30
                + name.len()
                + if index == 0 {
                    1
                } else {
                    2
                };
            directory += 46 + name.len();
        }
        Ok(())
    });
}

#[test]
fn stored_writer_framing_needs_root_even_with_fixed_zip_headers() {
    run_checked_test(|| -> Result<(), String> {
        for entries in [vec![], vec![(
            "assets/mark.webp",
            b"x".as_slice(),
            0x8cdc_1683,
        )]] {
            let bytes = make_zip(&entries)?;
            let archive = read_stored_profile_zip(&bytes, limits())
                .require_fixture(
                    "ordinary stored ZIP without a manifest is readable",
                )?;
            assert!(
                archive.matches_fixed_writer_metadata(),
                "every present ZIP header has the fixed writer's metadata"
            );
            assert!(
                !archive.matches_stored_writer_framing(),
                "writer needs an exact root even when ZIP framing is valid"
            );
            assert_eq!(
                archive.manifest_utf8(100),
                Err(ProfileZipReadError::MissingManifest)
            );
        }
        let bytes = make_zip(&[("manifest.json", b"{}", 0xa3a6_bf43)])?;
        let archive = read_stored_profile_zip(&bytes, limits())
            .require_fixture(
                "with exact root ZIP becomes a writer framing candidate",
            )?;
        assert!(archive.matches_stored_writer_framing());
        Ok(())
    });
}

#[test]
fn checked_zip_names_copy_into_bounded_caller_storage_atomically() {
    run_checked_test(|| -> Result<(), String> {
        let entries = [
            ("assets/\u{f1}.webp", b"".as_slice(), 0),
            ("manifest.json", b"{}".as_slice(), 0xa3a6_bf43),
            (
                "sections/\u{1f600}.json",
                b"123456789".as_slice(),
                0xcbf4_3926,
            ),
        ];
        let source = make_zip(&entries)?;
        let archive = read_stored_profile_zip(&source, limits())
            .require_fixture("sorted Unicode member names accepted")?;
        let mut destination = ["untouched"; 5];
        for capacity in [0usize, 1, 2] {
            assert_eq!(
                archive.copy_observed_paths(
                    destination
                        .get_mut(..capacity)
                        .ok_or("destination capacity")?
                ),
                Err(ProfileZipPathCopyError::BufferTooSmall { required: 3 }),
            );
            assert_eq!(
                destination, ["untouched"; 5],
                "insufficient destination changes no entries"
            );
        }
        assert_eq!(archive.copy_observed_paths(&mut destination[..3]), Ok(3));
        assert_eq!(&destination[..3], &[
            "assets/\u{f1}.webp",
            "manifest.json",
            "sections/\u{1f600}.json",
        ]);
        assert_eq!(&destination[3..], &["untouched"; 2]);
        for (borrowed, entry) in destination.iter().zip(archive.entries()) {
            assert!(ptr::eq(borrowed.as_ptr(), entry.name().as_ptr()));
            assert_eq!(borrowed.len(), entry.name().len());
        }
        assert_eq!(archive.copy_observed_paths(&mut destination), Ok(3));
        assert_eq!(&destination[3..], &["untouched"; 2]);
        assert_eq!(&destination[..3], archive.observed_paths());
        Ok(())
    });
}

#[test]
fn empty_and_rootless_archives_can_copy_names_without_import_authority() {
    run_checked_test(|| -> Result<(), String> {
        for (entries, expected) in [
            (vec![], vec![]),
            (
                vec![("assets/missing.webp", b"x".as_slice(), 0x8cdc_1683)],
                vec!["assets/missing.webp"],
            ),
        ] {
            let source = make_zip(&entries)?;
            let archive = read_stored_profile_zip(&source, limits())
                .require_fixture(
                    "framed archive may lack an importable root",
                )?;
            let mut names = ["unmodified"; 2];
            assert_eq!(
                archive.copy_observed_paths(&mut names),
                Ok(expected.len())
            );
            assert_eq!(
                names.get(..expected.len()).ok_or("copied names prefix")?,
                expected
            );
            assert_eq!(
                names.get(expected.len()..).ok_or("copied names suffix")?,
                ["unmodified"; 2]
                    .get(expected.len()..)
                    .ok_or("expected names suffix")?
            );
            assert_eq!(
                archive.manifest_utf8(64),
                Err(ProfileZipReadError::MissingManifest)
            );
            assert!(!archive.matches_stored_writer_framing());
        }
        Ok(())
    });
}

fn verify_sorted_preflight_crc_valid_digest_mismatch(
    selected: &ExplicitFilePathIntent,
    manifest: &ProfileManifest,
    encoding: ProfileArchiveEncodingEvidence,
    budget: ProfileImportBudget,
    checked: &ProfileZipArchive<'_>,
) {
    run_checked_test(|| -> Result<(), String> {
        let damaged = make_zip(&[
            ENTRIES[0],
            ("sections/strokes.json", b"123456788", 0xbcf3_09b0),
        ])?;
        let mutated = read_stored_profile_zip(&damaged, limits())
            .require_fixture(
                "new CRC-valid ZIP has identical admitted paths",
            )?;
        assert_eq!(
            mutated.encoding_evidence(),
            checked.encoding_evidence(),
            "CRC-valid mutation must fail SHA admission"
        );
        let mut changed_names = ["unmodified"; 2];
        assert_eq!(
            mutated.copy_observed_paths(&mut changed_names),
            Ok(2),
            "CRC-valid mutation must fail SHA admission"
        );
        let changed_preflight = preflight_explicit_profile_import_sorted(
            selected,
            manifest,
            &["strokes"],
            &changed_names,
            encoding,
            budget,
        )
        .require_fixture(
            "structural Import preflight cannot prove entry hash",
        )?;
        assert_eq!(verify_admitted_profile_zip_payloads(
        &mutated, changed_preflight.inventory(),
    ).err(), Some(ProfileZipPayloadVerificationError::Entry {
        index: 0,
        reason: atrament_handwriting_profile_container::
            ProfileEntryVerificationError::DigestMismatch,
    }), "CRC-valid mutation must fail SHA admission");
        assert_ne!(
            mutated.source_sha256(),
            checked.source_sha256(),
            "CRC-valid mutation must fail SHA admission"
        );
        assert_eq!(
            mutated
                .admitted_root_json(ProfileJsonSyntaxLimits {
                    maximum_bytes: 2,
                    maximum_nesting: 1,
                    maximum_object_members: 1,
                    maximum_values: 1,
                })
                .require_fixture("unchanged root grammar")?
                .source_sha256(),
            checked
                .admitted_root_json(ProfileJsonSyntaxLimits {
                    maximum_bytes: 2,
                    maximum_nesting: 1,
                    maximum_object_members: 1,
                    maximum_values: 1,
                })
                .require_fixture("unchanged root grammar")?
                .source_sha256(),
            "CRC-valid mutation must fail SHA admission"
        );
        Ok(())
    });
}

#[test]
fn sorted_import_preflight_still_requires_real_member_sha256_evidence() {
    run_checked_test(|| -> Result<(), String> {
        let original = make_zip(&ENTRIES)?;
        let checked = read_stored_profile_zip(&original, limits())
            .require_fixture(
                "root and section have valid ZIP framing and CRC",
            )?;
        let selected = classify_explicit_file_path_intent(
            ExplicitFilePurpose::ImportProfile,
            ExplicitFilePathSource::CallerSelection,
            PathBuf::from("selected/handwriting.atrament"),
            ExplicitOverwriteIntent::PreserveExisting,
        )
        .require_fixture("trusted selection is independent of ZIP contents")?;
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
            &selected,
            &manifest,
            &["strokes"],
            &names,
            encoding,
            budget,
        )
        .require_fixture(
            "exact sorted names and budget are independent of bytes",
        )?;
        let verified = verify_admitted_profile_zip_payloads(
            &checked,
            preflight.inventory(),
        )
        .require_fixture("all member hashes must match before exposure")?;
        assert_eq!(verified.verified_entry_count(), 1);
        assert_eq!(
            verified.verified_entry_bytes("sections/strokes.json"),
            Some(b"123456789".as_slice())
        );
        assert_eq!(verified.verified_entry_bytes("manifest.json"), None);
        let borrowed = verified
            .verified_entry_bytes("sections/strokes.json")
            .require_fixture("verified member bytes")?;
        assert!(ptr::eq(
            borrowed.as_ptr(),
            checked
                .entries()
                .get(1)
                .ok_or("missing ZIP entry")?
                .bytes()
                .as_ptr()
        ));
        verify_sorted_preflight_crc_valid_digest_mismatch(
            &selected, &manifest, encoding, budget, &checked,
        );
        Ok(())
    });
}

#[test]
fn checked_zip_encoding_evidence_survives_readable_legacy_header_variants() {
    run_checked_test(|| -> Result<(), String> {
        use atrament_handwriting_profile_container::{
            ProfileArchiveEncodingEvidence, ProfileArchiveEntryEncoding,
            ProfileArchivePlatformExtras, ProfileZip64Requirement,
            ProfileZip64Use, validate_profile_archive_encoding_view,
        };
        let original = make_zip(&ENTRIES)?;
        let canonical = read_stored_profile_zip(&original, limits())
            .require_fixture(
                "original header bytes have checked stored framing",
            )?;
        let expected = ProfileArchiveEncodingEvidence {
            entry_encoding: ProfileArchiveEntryEncoding::Stored,
            platform_extras: ProfileArchivePlatformExtras::Absent,
            zip64_requirement: ProfileZip64Requirement::Ordinary,
            zip64_use: ProfileZip64Use::Absent,
        };
        assert_eq!(canonical.encoding_evidence(), expected);
        assert!(
            validate_profile_archive_encoding_view(
                canonical.encoding_evidence()
            )
            .is_ok()
        );
        assert!(canonical.matches_stored_writer_framing());
        let first_directory = central_start(&original)?;
        for (name, local_field, central_field, value) in [
            ("creator version", None, 4usize, [10u8, 0]),
            ("extractor version", Some(4usize), 6, [10, 0]),
            ("ASCII UTF-8 bit", Some(6usize), 8, [0, 0]),
            ("modified time", Some(10usize), 12, [3, 0]),
            ("modified date", Some(12usize), 14, [3, 0]),
        ] {
            let mut changed = original.clone();
            if let Some(offset) = local_field {
                changed
                    .get_mut(offset..offset + 2)
                    .ok_or("changed local header field")?
                    .copy_from_slice(&value);
            }
            changed
                .get_mut(
                    first_directory + central_field
                        ..first_directory + central_field + 2,
                )
                .ok_or("central field patch")?
                .copy_from_slice(&value);
            let archived = read_stored_profile_zip(&changed, limits())
                .require_fixture(
                    "legacy metadata is permitted in ordinary stored ZIP",
                )?;
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
                changed
                    .get_mut(offset..offset + 2)
                    .ok_or("changed local header field")?
                    .copy_from_slice(&value);
            }
            changed
                .get_mut(
                    first_directory + central_field
                        ..first_directory + central_field + 2,
                )
                .ok_or("central field patch")?
                .copy_from_slice(&value);
            assert!(
                read_stored_profile_zip(&changed, limits()).is_err(),
                "{name} must not yield any checked encoding evidence"
            );
        }
        Ok(())
    });
}
