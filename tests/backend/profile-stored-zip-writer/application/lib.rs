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
//   - Independent reader round trips, fixed metadata and source budgets.
// - Must-Not:
//   - Freeze portable manifest schema or authorize a filesystem Export.
// - Allows:
//   - Inputs: In-memory named ZIP payloads and malicious member fixtures.
//   - Outputs: Exact frame, reader, checksum and error assertions.
//   - Side effects: Test-owned memory only.
// - Split-When:
//   - Canonical ZIP64 output requires an independent encoding adapter.
// - Merge-When:
//   - Complete profile codec tests own typed serialized manifest bytes.
// - Summary:
//   - Proves deterministic ordinary ZIP framing accepted by strict reader.
// - Description:
//   - Different payload bytes must produce different CRC-correct records.
// - Usage:
//   - Run with Cargo workspace tests and Jig package checks.
// - Defaults:
//   - Reject names and lengths before any bytes are emitted.
//

//! Deterministic stored-only ZIP framing and fail-closed budget checks.

use std::fmt::Write as _;
use std::ptr;

use atrament_handwriting_profile_container::{
    PROFILE_CONTAINER_VERSION, PROFILE_MANIFEST_PATH, ProfileManifest,
    ProfileManifestEntry, profile_entry_kind, validate_profile_manifest_view,
};
use atrament_profile_entry_sha256::sha256_profile_bytes;
use atrament_profile_json_syntax::ProfileJsonSyntaxLimits;
use atrament_profile_stored_zip_writer::{
    ProfileStoredZipMember, ProfileStoredZipWriteError,
    ProfileStoredZipWriteLimits, planned_stored_profile_zip_bytes,
    write_stored_profile_zip, write_stored_profile_zip_into,
};
use atrament_profile_zip_reader::{
    ProfileZipArchive, ProfileZipPathCopyError, ProfileZipReadError,
    ProfileZipReadLimits, read_stored_profile_zip,
    verify_admitted_profile_zip_payloads,
};

fn run_checked_test(test: impl FnOnce() -> Result<(), String>) {
    let result = test();
    assert!(result.is_ok(), "{result:?}");
}

fn hex_digest(bytes: &[u8]) -> Result<String, String> {
    let mut result = String::new();
    for byte in bytes {
        write!(&mut result, "{byte:02x}")
            .map_err(|error| format!("hex digest: {error:?}"))?;
    }
    Ok(result)
}

// Retain all maximum-ZIP32 reader witnesses without one oversized test body.
fn verify_maximum_reader_identity(
    bytes: &[u8],
    planned: usize,
    maximum_entries: usize,
) {
    run_checked_test(|| -> Result<(), String> {
        let read_limits = ProfileZipReadLimits {
            maximum_archive_bytes: planned,
            maximum_entries,
            maximum_entry_bytes: 2,
            maximum_total_bytes: 2,
        };
        let parsed =
            read_stored_profile_zip(bytes, read_limits).map_err(|error| {
                format!(
                    "strict index accepts maximal ordinary ZIP count: {error:?}"
                )
            })?;
        assert!(
            parsed.matches_fixed_writer_metadata(),
            "stored ZIP framing witness"
        );
        assert!(
            parsed.matches_stored_writer_framing(),
            "stored ZIP framing witness"
        );
        assert_eq!(
            parsed.entries().len(),
            maximum_entries,
            "stored ZIP framing witness"
        );
        verify_maximum_storage_paths(&parsed, maximum_entries);
        verify_maximum_record_names(&parsed, maximum_entries);
        assert_eq!(
            parsed.source_sha256(),
            sha256_profile_bytes(bytes),
            "stored ZIP framing witness"
        );
        // Python's independent struct/zlib implementation produced these exact
        // ZIP32 bytes; its standard zipfile reader opened all 65,534 entries.
        assert_eq!(planned, 7_077_690, "stored ZIP framing witness");
        let digest = parsed.source_sha256().bytes();
        let golden = hex_digest(&digest)?;
        assert_eq!(
            golden,
            "c564b867a623511169ae843eac4acf3f08322782fdf7b6b6bb7351b90a52a793",
            "stored ZIP framing witness"
        );
        Ok(())
    });
}

fn verify_maximum_storage_paths(
    parsed: &ProfileZipArchive<'_>,
    maximum_entries: usize,
) {
    run_checked_test(|| -> Result<(), String> {
        let mut slots = vec![
            "unchanged";
            maximum_entries
                .checked_add(1)
                .ok_or("bounded maximum_entries add")?
        ];
        assert_eq!(
            parsed.copy_observed_paths(
                slots
                    .get_mut(
                        ..maximum_entries
                            .checked_sub(1)
                            .ok_or("bounded maximum_entries sub")?
                    )
                    .ok_or("slots mutable range")?
            ),
            Err(ProfileZipPathCopyError::BufferTooSmall {
                required: maximum_entries,
            }),
            "largest supported member count refuses short storage atomically",
        );
        assert!(
            slots.iter().all(|name| *name == "unchanged"),
            "stored ZIP framing witness"
        );
        assert_eq!(
            parsed.copy_observed_paths(&mut slots),
            Ok(maximum_entries),
            "stored ZIP framing witness"
        );
        assert_eq!(
            slots.first().copied().ok_or("missing slots record")?,
            "assets/00000.bin",
            "stored ZIP framing witness"
        );
        assert_eq!(
            slots
                .get(
                    maximum_entries
                        .checked_sub(1)
                        .ok_or("bounded maximum_entries sub")?
                )
                .copied()
                .ok_or("missing slots record")?,
            "manifest.json",
            "stored ZIP framing witness"
        );
        assert_eq!(
            slots
                .get(maximum_entries)
                .copied()
                .ok_or("missing slots record")?,
            "unchanged",
            "stored ZIP framing witness"
        );
        Ok(())
    });
}

fn verify_maximum_record_names(
    parsed: &ProfileZipArchive<'_>,
    maximum_entries: usize,
) {
    run_checked_test(|| -> Result<(), String> {
        let mut slots =
            vec![
                "unchanged";
                maximum_entries.checked_add(1).ok_or("path capacity")?
            ];
        assert_eq!(
            parsed.copy_observed_paths(&mut slots),
            Ok(maximum_entries),
            "returned path storage preserves all ZIP entries"
        );
        assert!(
            ptr::eq(
                slots.first().ok_or("missing slots record")?.as_ptr(),
                parsed
                    .entries()
                    .first()
                    .ok_or("missing parsed record")?
                    .name()
                    .as_ptr()
            ),
            "stored ZIP framing witness"
        );
        assert!(
            ptr::eq(
                slots
                    .get(
                        maximum_entries
                            .checked_sub(1)
                            .ok_or("bounded maximum_entries sub")?
                    )
                    .ok_or("missing slots record")?
                    .as_ptr(),
                parsed
                    .entries()
                    .get(
                        maximum_entries
                            .checked_sub(1)
                            .ok_or("bounded maximum_entries sub")?
                    )
                    .ok_or("missing parsed record")?
                    .name()
                    .as_ptr()
            ),
            "stored ZIP framing witness"
        );
        verify_maximum_record_payloads(parsed, maximum_entries);
        Ok(())
    });
}

fn verify_maximum_record_payloads(
    parsed: &ProfileZipArchive<'_>,
    maximum_entries: usize,
) {
    run_checked_test(|| -> Result<(), String> {
        assert_eq!(
            parsed
                .entries()
                .first()
                .ok_or("missing parsed record")?
                .name(),
            "assets/00000.bin",
            "stored ZIP framing witness"
        );
        assert_eq!(
            parsed
                .entries()
                .get(
                    maximum_entries
                        .checked_sub(2)
                        .ok_or("bounded maximum_entries sub")?
                )
                .ok_or("missing parsed record")?
                .name(),
            "assets/65532.bin",
            "stored ZIP framing witness"
        );
        assert_eq!(
            parsed
                .entries()
                .get(
                    maximum_entries
                        .checked_sub(1)
                        .ok_or("bounded maximum_entries sub")?
                )
                .ok_or("missing parsed record")?
                .name(),
            "manifest.json",
            "stored ZIP framing witness"
        );
        assert_eq!(
            parsed
                .entries()
                .first()
                .ok_or("missing parsed record")?
                .bytes(),
            b"",
            "stored ZIP framing witness"
        );
        assert_eq!(
            parsed
                .entries()
                .get(
                    maximum_entries
                        .checked_sub(1)
                        .ok_or("bounded maximum_entries sub")?
                )
                .ok_or("missing parsed record")?
                .bytes(),
            b"{}",
            "stored ZIP framing witness"
        );
        Ok(())
    });
}

fn verify_maximum_reader_limits(
    bytes: &[u8],
    planned: usize,
    maximum_entries: usize,
) {
    run_checked_test(|| -> Result<(), String> {
        let read_limits = ProfileZipReadLimits {
            maximum_archive_bytes: planned,
            maximum_entries,
            maximum_entry_bytes: 2,
            maximum_total_bytes: 2,
        };
        assert_eq!(
            read_stored_profile_zip(bytes, ProfileZipReadLimits {
                maximum_entries: maximum_entries
                    .checked_sub(1)
                    .ok_or("bounded maximum_entries sub")?,
                ..read_limits
            }),
            Err(ProfileZipReadError::ResourceLimit),
            "too many members must reject before index reservation",
        );
        assert_eq!(
            read_stored_profile_zip(bytes, ProfileZipReadLimits {
                maximum_archive_bytes: planned
                    .checked_sub(1)
                    .ok_or("bounded planned sub")?,
                ..read_limits
            }),
            Err(ProfileZipReadError::ArchiveTooLarge),
            "one byte less than exact archive budget must reject",
        );
        assert_eq!(
            read_stored_profile_zip(bytes, ProfileZipReadLimits {
                maximum_total_bytes: 1,
                ..read_limits
            }),
            Err(ProfileZipReadError::ResourceLimit),
            "many empty entries cannot bypass nonempty root payload limit",
        );
        Ok(())
    });
}

fn verify_generated_frame(
    selected: &[ProfileStoredZipMember<'_>],
    generous: ProfileStoredZipWriteLimits,
    case: usize,
) {
    run_checked_test(|| -> Result<(), String> {
        let planned = planned_stored_profile_zip_bytes(selected, generous)
            .map_err(|error| {
                format!("finite inputs fit deterministic ZIP32 plan: {error:?}")
            })?;
        let canonical =
            write_stored_profile_zip(selected, generous).map_err(|error| {
                format!(
                    "all generated members have safe sorted paths: {error:?}"
                )
            })?;
        assert_eq!(canonical.len(), planned, "case={case}");
        let mut caller_storage =
            vec![0x6du8; planned.checked_add(3).ok_or("bounded planned add")?];
        assert_eq!(
            write_stored_profile_zip_into(
                selected,
                generous,
                &mut caller_storage,
            ),
            Ok(planned),
            "case={case}"
        );
        assert_eq!(
            caller_storage
                .get(..planned)
                .ok_or("caller_storage range")?,
            canonical.as_slice(),
            "stored ZIP framing witness"
        );
        assert_eq!(
            caller_storage
                .get(planned..)
                .ok_or("caller_storage range")?,
            &[0x6d; 3],
            "stored ZIP framing witness"
        );
        let archive = read_stored_profile_zip(
            &canonical,
            ProfileZipReadLimits {
                maximum_archive_bytes: planned,
                maximum_entries: selected.len(),
                maximum_entry_bytes: generous.maximum_entry_bytes,
                maximum_total_bytes: generous.maximum_total_bytes,
            },
        )
        .map_err(|error| {
            format!(
                "independent strict reader must accept writer output: {error:?}"
            )
        })?;
        assert!(archive.matches_stored_writer_framing(), "case={case}");
        assert!(archive.matches_fixed_writer_metadata(), "case={case}");
        assert_eq!(archive.source_bytes(), canonical, "case={case}");
        let recovered = archive
            .entries()
            .iter()
            .map(|entry| member(entry.name(), entry.bytes()))
            .collect::<Vec<_>>();
        assert_eq!(recovered.as_slice(), selected, "case={case}");
        let encoded_again = write_stored_profile_zip(&recovered, generous)
            .map_err(|error| format!("re-encode drift: {error:?}"))?;
        assert_eq!(encoded_again, canonical, "case={case}");
        assert_eq!(
            archive.source_sha256(),
            sha256_profile_bytes(&canonical),
            "stored ZIP framing witness"
        );
        verify_generated_legacy_metadata(
            &canonical,
            &archive,
            generous,
            selected.len(),
            case,
        );
        Ok(())
    });
}

fn verify_generated_legacy_metadata(
    canonical: &[u8],
    archive: &ProfileZipArchive<'_>,
    generous: ProfileStoredZipWriteLimits,
    entry_count: usize,
    case: usize,
) {
    run_checked_test(|| -> Result<(), String> {
        // One matching DOS date edit in the first local and central headers
        // retains CRC-verified payloads but can never claim writer framing.
        let directory: usize = u32::from_le_bytes(
            canonical
                .get(
                    canonical.len().checked_sub(6).ok_or("end record prefix")?
                        ..canonical
                            .len()
                            .checked_sub(2)
                            .ok_or("end record suffix")?,
                )
                .ok_or("canonical range")?
                .try_into()
                .map_err(|error| {
                    format!("central directory byte offset: {error:?}")
                })?,
        )
        .try_into()
        .map_err(|error| format!("ZIP offset: {error:?}"))?;
        let mut legacy = canonical.to_vec();
        legacy
            .get_mut(12..14)
            .ok_or("legacy range")?
            .copy_from_slice(&[3, 0]);
        legacy
            .get_mut(
                directory.checked_add(14).ok_or("bounded directory add")?
                    ..directory
                        .checked_add(16)
                        .ok_or("bounded directory add")?,
            )
            .ok_or("legacy range")?
            .copy_from_slice(&[3, 0]);
        let observed = read_stored_profile_zip(&legacy, ProfileZipReadLimits {
            maximum_archive_bytes: legacy.len(),
            maximum_entries: entry_count,
            maximum_entry_bytes: generous.maximum_entry_bytes,
            maximum_total_bytes: generous.maximum_total_bytes,
        })
        .map_err(|error| format!("legacy metadata: {error:?}"))?;
        assert!(!observed.matches_stored_writer_framing(), "case={case}");
        assert_eq!(observed.entries(), archive.entries(), "case={case}");
        assert_ne!(
            observed.source_sha256(),
            archive.source_sha256(),
            "stored ZIP framing witness"
        );
        Ok(())
    });
}

const fn limits() -> ProfileStoredZipWriteLimits {
    ProfileStoredZipWriteLimits {
        maximum_archive_bytes: 100_000,
        maximum_entries: 256,
        maximum_entry_bytes: 65_536,
        maximum_total_bytes: 100_000,
    }
}

fn parse_bytes(bytes: &[u8]) -> Result<ProfileZipArchive<'_>, String> {
    read_stored_profile_zip(bytes, ProfileZipReadLimits {
        maximum_archive_bytes: bytes.len(),
        maximum_entries: 256,
        maximum_entry_bytes: 65_536,
        maximum_total_bytes: 100_000,
    })
    .map_err(|error| {
        format!("strict ZIP reader refused emitted bytes: {error:?}")
    })
}

const fn member<'member>(
    name: &'member str,
    bytes: &'member [u8],
) -> ProfileStoredZipMember<'member> {
    ProfileStoredZipMember { bytes, name }
}

#[test]
fn stable_root_only_archive_has_correct_fixed_header_and_directory() {
    run_checked_test(|| -> Result<(), String> {
        let members = [member(PROFILE_MANIFEST_PATH, b"{}")];
        let first = write_stored_profile_zip(&members, limits())
            .map_err(|error| format!("single ZIP root: {error:?}"))?;
        let second = write_stored_profile_zip(&members, limits())
            .map_err(|error| format!("deterministic ZIP output: {error:?}"))?;
        assert_eq!(first, second);
        assert_eq!(first.len(), 30 + 13 + 2 + 46 + 13 + 22);
        assert_eq!(first.get(..4), Some(&[0x50, 0x4b, 0x03, 0x04][..]));
        assert_eq!(first.get(4..6), Some(&[20, 0][..]));
        assert_eq!(first.get(6..8), Some(&[0, 8][..]));
        assert_eq!(first.get(8..14), Some(&[0; 6][..]));
        assert_eq!(first.get(14..18), Some(&[0x43, 0xbf, 0xa6, 0xa3][..]));
        assert_eq!(first.get(26..30), Some(&[13, 0, 0, 0][..]));
        let archive = parse_bytes(&first)?;
        assert!(archive.matches_fixed_writer_metadata());
        assert!(archive.matches_stored_writer_framing());
        assert_eq!(archive.entries().len(), 1);
        assert_eq!(archive.manifest_utf8(2), Ok("{}"));
        assert_eq!(
            archive
                .entries()
                .first()
                .ok_or("missing archive record")?
                .crc32(),
            0xa3a6_bf43
        );
        Ok(())
    });
}

#[test]
fn sorted_utf8_names_and_empty_asset_round_trip_byte_for_byte() {
    run_checked_test(|| -> Result<(), String> {
        let entries = [
            member("assets/empty.webp", b""),
            member("assets/\u{f1}.webp", b"\x01\x00\xff"),
            member("manifest.json", b"{\"example\":true}"),
            member("sections/\u{1f600}.json", b"[0,1,2]"),
        ];
        let output = write_stored_profile_zip(&entries, limits())
            .map_err(|error| format!("canonical name order: {error:?}"))?;
        let parsed = parse_bytes(&output)?;
        assert!(parsed.matches_fixed_writer_metadata());
        assert!(parsed.matches_stored_writer_framing());
        assert_eq!(parsed.entries().len(), entries.len());
        for (admitted, original) in parsed.entries().iter().zip(entries) {
            assert_eq!(admitted.name(), original.name);
            assert_eq!(admitted.bytes(), original.bytes);
        }
        assert_eq!(
            parsed
                .entries()
                .first()
                .ok_or("missing parsed record")?
                .bytes(),
            b""
        );
        let duplicate =
            write_stored_profile_zip(&entries, limits()).map_err(|error| {
                format!("same inputs produce same output bytes: {error:?}")
            })?;
        assert_eq!(output, duplicate);
        Ok(())
    });
}

#[test]
fn missing_root_is_not_a_complete_stored_archive_candidate() {
    run_checked_test(|| -> Result<(), String> {
        for candidates in
            [vec![], vec![member("assets/valid.webp", b"abc")], vec![
                member("sections/x.json", b"{}"),
            ]]
        {
            assert_eq!(
                write_stored_profile_zip(&candidates, limits()),
                Err(ProfileStoredZipWriteError::MissingManifest)
            );
        }
        Ok(())
    });
}

#[test]
fn repeated_reverse_and_near_duplicate_names_are_rejected() {
    run_checked_test(|| -> Result<(), String> {
        let duplicates =
            [member("manifest.json", b"a"), member("manifest.json", b"b")];
        let reversed = [
            member("sections/x.json", b"{}"),
            member("manifest.json", b"{}"),
        ];
        for candidates in [&duplicates[..], &reversed[..]] {
            assert_eq!(
                write_stored_profile_zip(candidates, limits()),
                Err(ProfileStoredZipWriteError::NonCanonicalOrder)
            );
        }
        // Percent signs are literal ZIP member bytes, never URI-decoded into
        // a traversal segment by this framing-only adapter.
        let literal_percent = [
            member("manifest.json", b"{}"),
            member("sections/%2e%2e/x", b"opaque"),
        ];
        assert!(write_stored_profile_zip(&literal_percent, limits()).is_ok());
        let aliases = [
            member("manifest.json", b"{}"),
            member("sections/X.json", b"{}"),
            member("sections/x.json", b"{}"),
        ];
        assert!(
            write_stored_profile_zip(&aliases, limits()).is_ok(),
            "byte-distinct case-sensitive names are not aliases"
        );
        Ok(())
    });
}

#[test]
fn unsafe_member_names_are_rejected_without_canonicalizing_them() {
    run_checked_test(|| -> Result<(), String> {
        for name in [
            "",
            "/",
            "manifest.json/",
            "assets/",
            "./assets/x",
            "assets/../secret",
            "sections\\bad",
            "sections/x\0.json",
            "sections/x\n.json",
            "sections/x\r.json",
            "sections/x\t.json",
            "other/x",
            "MANIFEST.JSON",
            "assets//duplicate",
            "sections/x/../y",
            "sections/x.json/",
        ] {
            let entries = [member("manifest.json", b"{}"), member(name, b"x")];
            let result = write_stored_profile_zip(&entries, limits());
            assert!(
                matches!(
                    result,
                    Err(ProfileStoredZipWriteError::InvalidName
                        | ProfileStoredZipWriteError::NonCanonicalOrder)
                ),
                "unsafe member {name:?} was admitted: {result:?}"
            );
        }
        Ok(())
    });
}

#[test]
fn exact_budget_ceiling_accepts_and_one_byte_short_rejects() {
    run_checked_test(|| -> Result<(), String> {
        let members = [member("manifest.json", b"123456789")];
        let bytes = write_stored_profile_zip(&members, limits())
            .map_err(|error| format!("bounded source: {error:?}"))?;
        let exact = ProfileStoredZipWriteLimits {
            maximum_archive_bytes: bytes.len(),
            maximum_entries: 1,
            maximum_entry_bytes: 9,
            maximum_total_bytes: 9,
        };
        assert_eq!(
            write_stored_profile_zip(&members, exact),
            Ok(bytes.clone())
        );
        for smaller in [
            ProfileStoredZipWriteLimits {
                maximum_archive_bytes: bytes.len() - 1,
                ..exact
            },
            ProfileStoredZipWriteLimits {
                maximum_entries: 0,
                ..exact
            },
            ProfileStoredZipWriteLimits {
                maximum_entry_bytes: 8,
                ..exact
            },
            ProfileStoredZipWriteLimits {
                maximum_total_bytes: 8,
                ..exact
            },
        ] {
            assert!(write_stored_profile_zip(&members, smaller).is_err());
        }
        Ok(())
    });
}

#[test]
fn fixed_output_crc_changes_when_bytes_change_even_if_name_does_not() {
    run_checked_test(|| -> Result<(), String> {
        let first = write_stored_profile_zip(
            &[member("manifest.json", b"{\"id\":1}")],
            limits(),
        )
        .map_err(|error| format!("original: {error:?}"))?;
        let second = write_stored_profile_zip(
            &[member("manifest.json", b"{\"id\":2}")],
            limits(),
        )
        .map_err(|error| format!("changed root: {error:?}"))?;
        assert_eq!(first.len(), second.len());
        assert_ne!(first, second);
        assert_ne!(
            parse_bytes(&first)?
                .entries()
                .first()
                .ok_or("first ZIP entry")?
                .crc32(),
            parse_bytes(&second)?
                .entries()
                .first()
                .ok_or("second ZIP entry")?
                .crc32()
        );
        Ok(())
    });
}

#[test]
fn maximum_member_count_is_preflighted_before_allocating_archive() {
    run_checked_test(|| -> Result<(), String> {
        let members = [member("manifest.json", b"{}")];
        assert_eq!(
            write_stored_profile_zip(&members, ProfileStoredZipWriteLimits {
                maximum_entries: 0,
                ..limits()
            }),
            Err(ProfileStoredZipWriteError::ResourceLimit)
        );
        Ok(())
    });
}

#[test]
fn golden_root_zip_matches_independent_struct_and_crc_reference() {
    run_checked_test(|| -> Result<(), String> {
        // Generated separately using Python's struct, zlib and zipfile modules.
        // No ZIP code or digest helpers from this crate produced the oracle.
        const GOLDEN: &str = concat!(
            "504b03041400000800000000000043bfa6a302000000020000000d0000006d61",
            "6e69666573742e6a736f6e7b7d504b010214001400000800000000000043bfa6",
            "a302000000020000000d00000000000000000000000000000000006d616e6966",
            "6573742e6a736f6e504b050600000000010001003b0000002d0000000000",
        );
        let bytes = write_stored_profile_zip(
            &[member("manifest.json", b"{}")],
            limits(),
        )
        .map_err(|error| format!("root-only fixed ZIP: {error:?}"))?;
        let encoded = hex_digest(&bytes)?;
        assert_eq!(encoded, GOLDEN);
        Ok(())
    });
}

#[test]
fn zip_entry_count_sentinel_refuses_65_535_members_without_output() {
    run_checked_test(|| -> Result<(), String> {
        let members =
            vec![member("manifest.json", b"{}"); usize::from(u16::MAX)];
        let limit = ProfileStoredZipWriteLimits {
            maximum_entries: members.len(),
            ..limits()
        };
        assert_eq!(
            write_stored_profile_zip(&members, limit),
            Err(ProfileStoredZipWriteError::UnsupportedZip64)
        );
        Ok(())
    });
}

#[test]
fn utf8_member_name_field_limit_is_enforced_on_byte_length() {
    run_checked_test(|| -> Result<(), String> {
        let maximum_name = format!("assets/{}", "x".repeat(65_535 - 7));
        let too_long = format!("{maximum_name}x");
        assert_eq!(maximum_name.len(), usize::from(u16::MAX));
        let limit = ProfileStoredZipWriteLimits {
            maximum_archive_bytes: 200_000,
            maximum_entry_bytes: 100,
            maximum_total_bytes: 100,
            ..limits()
        };
        let admissible =
            [member(&maximum_name, b"x"), member("manifest.json", b"{}")];
        let archive_bytes = write_stored_profile_zip(&admissible, limit)
            .map_err(|error| {
                format!("maximal ZIP filename fits in u16: {error:?}")
            })?;
        let parsed =
            read_stored_profile_zip(&archive_bytes, ProfileZipReadLimits {
                maximum_archive_bytes: 200_000,
                maximum_entries: 2,
                maximum_entry_bytes: 100,
                maximum_total_bytes: 100,
            })
            .map_err(|error| {
                format!("strict reader accepts 65535 UTF-8 bytes: {error:?}")
            })?;
        assert_eq!(
            parsed
                .entries()
                .first()
                .ok_or("missing parsed record")?
                .name(),
            maximum_name
        );
        let overflow =
            [member(&too_long, b"x"), member("manifest.json", b"{}")];
        assert_eq!(
            write_stored_profile_zip(&overflow, limit),
            Err(ProfileStoredZipWriteError::InvalidName)
        );
        Ok(())
    });
}

#[test]
fn utf8_multibyte_names_obey_zip16_byte_limit_and_preserve_storage() {
    run_checked_test(|| -> Result<(), String> {
        let accepted_limit = ProfileStoredZipWriteLimits {
            maximum_archive_bytes: 200_000,
            maximum_entries: 2,
            maximum_entry_bytes: 100,
            maximum_total_bytes: 100,
        };
        let read_limit = ProfileZipReadLimits {
            maximum_archive_bytes: 200_000,
            maximum_entries: 2,
            maximum_entry_bytes: 100,
            maximum_total_bytes: 100,
        };
        let prefix = "assets/";
        let body_budget = usize::from(u16::MAX).saturating_sub(prefix.len());
        for (character, expected_width) in
            [("\u{f1}", 2usize), ("\u{20ac}", 3), ("\u{1f600}", 4)]
        {
            assert_eq!(character.len(), expected_width);
            let repetitions =
                body_budget.checked_div(expected_width).unwrap_or(0);
            let spare_bytes = body_budget.rem_euclid(expected_width);
            let name = format!(
                "{prefix}{}{}",
                character.repeat(repetitions),
                "x".repeat(spare_bytes),
            );
            assert_eq!(name.len(), usize::from(u16::MAX));
            assert!(name.chars().count() < name.len());
            let entries = [
                member(&name, b"asset"),
                member(PROFILE_MANIFEST_PATH, b"{}"),
            ];
            let plan_outcome =
                planned_stored_profile_zip_bytes(&entries, accepted_limit);
            let write_outcome =
                write_stored_profile_zip(&entries, accepted_limit);
            assert!(
                plan_outcome.is_ok(),
                "byte-exact name must fit: {plan_outcome:?}"
            );
            assert!(
                write_outcome.is_ok(),
                "byte-exact name must fit: {write_outcome:?}"
            );
            let (Ok(planned), Ok(encoded)) = (plan_outcome, write_outcome)
            else {
                return Ok(());
            };
            assert_eq!(encoded.len(), planned);
            let read_outcome = read_stored_profile_zip(&encoded, read_limit);
            assert!(
                read_outcome.is_ok(),
                "strict reader must admit name: {read_outcome:?}"
            );
            let Ok(parsed) = read_outcome else {
                return Ok(());
            };
            assert_eq!(
                parsed.entries().first().map(|entry| entry.name()),
                Some(name.as_str())
            );
            assert_eq!(
                parsed.entries().first().map(|entry| entry.bytes()),
                Some(&b"asset"[..])
            );
            assert_eq!(
                parsed.entries().last().map(|entry| entry.name()),
                Some(PROFILE_MANIFEST_PATH)
            );
            assert!(parsed.matches_stored_writer_framing());

            let overflow = format!("{name}z");
            assert_eq!(overflow.len(), usize::from(u16::MAX) + 1);
            let invalid = [
                member(&overflow, b"asset"),
                member(PROFILE_MANIFEST_PATH, b"{}"),
            ];
            assert_eq!(
                planned_stored_profile_zip_bytes(&invalid, accepted_limit),
                Err(ProfileStoredZipWriteError::InvalidName),
            );
            assert_eq!(
                write_stored_profile_zip(&invalid, accepted_limit),
                Err(ProfileStoredZipWriteError::InvalidName),
            );
            let mut destination = [0xa5u8; 128];
            assert_eq!(
                write_stored_profile_zip_into(
                    &invalid,
                    accepted_limit,
                    &mut destination
                ),
                Err(ProfileStoredZipWriteError::InvalidName),
            );
            assert_eq!(destination, [0xa5; 128]);
        }
        Ok(())
    });
}

#[test]
fn many_generated_sorted_payloads_are_stable_and_reader_admitted() {
    run_checked_test(|| -> Result<(), String> {
        for size in [1usize, 2, 3, 7, 17, 31, 64, 127] {
            let names = (0..size)
                .map(|index| format!("assets/{index:04}.bin"))
                .collect::<Vec<_>>();
            let contents = (0..size)
                .map(|index| {
                    (0..(index.rem_euclid(129)))
                        .map(|value| {
                            u8::try_from(value)
                                .map(|narrowed| narrowed.wrapping_mul(17))
                        })
                        .collect::<Result<Vec<_>, _>>()
                })
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| format!("pattern byte: {error:?}"))?;
            let mut members = names
                .iter()
                .zip(contents.iter())
                .map(|(name, content)| {
                    member(name.as_str(), content.as_slice())
                })
                .collect::<Vec<_>>();
            members.push(member("manifest.json", b"{}"));
            let output = write_stored_profile_zip(&members, limits()).map_err(
                |error| format!("bounded sorted members: {error:?}"),
            )?;
            let parsed = parse_bytes(&output)?;
            assert_eq!(parsed.source_bytes(), output);
            assert_eq!(parsed.source_sha256(), sha256_profile_bytes(&output));
            assert_eq!(parsed.entries().len(), members.len());
            for (expected, observed) in members.iter().zip(parsed.entries()) {
                assert_eq!(observed.name(), expected.name);
                assert_eq!(observed.bytes(), expected.bytes);
            }
            assert_eq!(
                write_stored_profile_zip(&members, limits()),
                Ok(output)
            );
        }
        Ok(())
    });
}

#[test]
fn emitted_archive_can_verify_complete_typed_inventory_and_sha256() {
    run_checked_test(|| -> Result<(), String> {
        let members = [
            member("assets/image.webp", b"\x00\xff\x01\x80"),
            member("manifest.json", b"{}"),
            member("sections/sample.json", br#"{"strokes":[1,2,3]}"#),
        ];
        let byte_archive = write_stored_profile_zip(&members, limits())
            .map_err(|error| format!("deterministic ZIP framing: {error:?}"))?;
        let mut output = vec![0xe3u8; byte_archive.len() + 7];
        assert_eq!(
            write_stored_profile_zip_into(&members, limits(), &mut output,),
            Ok(byte_archive.len())
        );
        assert_eq!(
            output.get(..byte_archive.len()).ok_or("output range")?,
            byte_archive.as_slice()
        );
        assert_eq!(
            output.get(byte_archive.len()..).ok_or("output range")?,
            &[0xe3; 7]
        );
        let parsed = parse_bytes(
            output.get(..byte_archive.len()).ok_or("output range")?,
        )?;
        assert!(atrament_handwriting_profile_container::
            validate_profile_archive_encoding_view(
                parsed.encoding_evidence(),
            ).is_ok());
        let typed = ProfileManifest {
            container_version: PROFILE_CONTAINER_VERSION.into(),
            entries: vec![
                ProfileManifestEntry {
                    byte_length: 4,
                    digest: sha256_profile_bytes(
                        members.first().ok_or("missing members record")?.bytes,
                    ),
                    media_type: "image/webp".into(),
                    path: "assets/image.webp".into(),
                },
                ProfileManifestEntry {
                    byte_length: u64::try_from(
                        members
                            .get(2)
                            .ok_or("missing members record")?
                            .bytes
                            .len(),
                    )
                    .map_err(|error| {
                        format!("fixed fixture fits u64: {error:?}")
                    })?,
                    digest: sha256_profile_bytes(
                        members.get(2).ok_or("missing members record")?.bytes,
                    ),
                    media_type: "application/json".into(),
                    path: "sections/sample.json".into(),
                },
            ],
            optional_features: vec![],
            profile_identity: "writer-reader-sha-fixture".into(),
            required_features: vec![],
        };
        let declared =
            validate_profile_manifest_view(&typed, &[]).map_err(|error| {
                format!("typed declaration is valid independently: {error:?}")
            })?;
        let mut names = ["unmodified"; 3];
        assert_eq!(parsed.copy_observed_paths(&mut names), Ok(3));
        let inventory =
            declared
                .admit_sorted_entry_inventory(&names)
                .map_err(|error| {
                    format!(
                        "exact source-borrowed sorted ZIP inventory: {error:?}"
                    )
                })?;
        let verified = verify_admitted_profile_zip_payloads(&parsed, inventory)
            .map_err(|error| {
                format!("all emitted members match typed digests: {error:?}")
            })?;
        assert_eq!(verified.verified_entry_count(), 2);
        assert_eq!(
            verified.verified_entry_bytes("assets/image.webp"),
            Some(members.first().ok_or("missing members record")?.bytes)
        );
        assert_eq!(
            verified.verified_entry_bytes("sections/sample.json"),
            Some(members.get(2).ok_or("missing members record")?.bytes)
        );
        assert_eq!(verified.verified_entry_bytes("manifest.json"), None);
        assert_eq!(verified.verified_entry_bytes("sections/SAMPLE.json"), None);
        Ok(())
    });
}

#[test]
fn framing_validity_does_not_imply_manifest_json_syntax_admission() {
    run_checked_test(|| -> Result<(), String> {
        let source = [member("manifest.json", b"{broken syntax...")];
        let bytes =
            write_stored_profile_zip(&source, limits()).map_err(|error| {
                format!("opaque root allowed at ZIP framing level: {error:?}")
            })?;
        let parsed = parse_bytes(&bytes)?;
        assert_eq!(parsed.manifest_utf8(20), Ok("{broken syntax..."));
        let json = parsed.manifest_json_object(ProfileJsonSyntaxLimits {
            maximum_bytes: 20,
            maximum_nesting: 10,
            maximum_object_members: 10,
            maximum_values: 10,
        });
        assert!(json.is_err(), "ZIP framing is not typed JSON admission");
        Ok(())
    });
}

#[test]
fn exact_framing_size_plan_matches_every_written_output() {
    run_checked_test(|| -> Result<(), String> {
        for count in [1usize, 2, 3, 4, 7, 23, 64, 127, 255] {
            let names = (0..count.saturating_sub(1))
                .map(|i| format!("assets/{i:05}.bin"))
                .collect::<Vec<_>>();
            let bytes = (0..count.saturating_sub(1))
                .map(|i| {
                    u8::try_from(i.rem_euclid(255))
                        .map(|value| vec![value; i.rem_euclid(99)])
                })
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| format!("generated bytes: {error:?}"))?;
            let mut members = names
                .iter()
                .zip(bytes.iter())
                .map(|(name, data)| member(name, data))
                .collect::<Vec<_>>();
            members.push(member("manifest.json", b"{}"));
            let bound = ProfileStoredZipWriteLimits {
                maximum_archive_bytes: 100_000,
                maximum_entries: count,
                maximum_entry_bytes: 100,
                maximum_total_bytes: 100_000,
            };
            let expected_len = planned_stored_profile_zip_bytes(
                &members, bound,
            )
            .map_err(|error| {
                format!("all fixed records fit admitted budgets: {error:?}")
            })?;
            let actual =
                write_stored_profile_zip(&members, bound).map_err(|error| {
                    format!("preflighted writer admits same members: {error:?}")
                })?;
            assert_eq!(actual.len(), expected_len, "count={count}");
            assert_eq!(parse_bytes(&actual)?.entries().len(), count);
            let exact_bound = ProfileStoredZipWriteLimits {
                maximum_archive_bytes: expected_len,
                ..bound
            };
            assert_eq!(
                planned_stored_profile_zip_bytes(&members, exact_bound),
                Ok(expected_len)
            );
            assert_eq!(
                write_stored_profile_zip(&members, exact_bound),
                Ok(actual)
            );
        }
        Ok(())
    });
}

#[test]
fn dry_run_and_writer_share_fail_closed_preflight_error_categories() {
    run_checked_test(|| -> Result<(), String> {
        let good = [member("manifest.json", b"{}")];
        let duplicates = [
            member("manifest.json", b"{}"),
            member("manifest.json", b"{}"),
        ];
        let invalid_path = [
            member("manifest.json", b"{}"),
            member("sections/../forbidden", b"content"),
        ];
        let reversed = [
            member("sections/x.json", b""),
            member("manifest.json", b"{}"),
        ];
        let missing = [member("assets/a.webp", b"1")];
        let empty: [ProfileStoredZipMember<'_>; 0] = [];
        let cases: [(
            &[ProfileStoredZipMember<'_>],
            ProfileStoredZipWriteLimits,
        ); 10] = [
            (&good, ProfileStoredZipWriteLimits {
                maximum_archive_bytes: 0,
                ..limits()
            }),
            (&good, ProfileStoredZipWriteLimits {
                maximum_entries: 0,
                ..limits()
            }),
            (&good, ProfileStoredZipWriteLimits {
                maximum_entry_bytes: 1,
                ..limits()
            }),
            (&good, ProfileStoredZipWriteLimits {
                maximum_total_bytes: 1,
                ..limits()
            }),
            (&duplicates, limits()),
            (&invalid_path, limits()),
            (&reversed, limits()),
            (&missing, limits()),
            (&empty, limits()),
            (&good, ProfileStoredZipWriteLimits {
                maximum_archive_bytes: 1,
                ..limits()
            }),
        ];
        for (input, bound) in cases {
            let planned = planned_stored_profile_zip_bytes(input, bound);
            let written = write_stored_profile_zip(input, bound);
            let mut sentinel = [0xb7u8; 256];
            let bounded =
                write_stored_profile_zip_into(input, bound, &mut sentinel);
            assert!(planned.is_err());
            assert_eq!(planned, written.map(|bytes| bytes.len()));
            assert_eq!(
                bounded, planned,
                "byte storage cannot mask a preflight failure"
            );
            assert_eq!(
                sentinel, [0xb7; 256],
                "failed inputs never partially overwrite caller storage"
            );
        }
        Ok(())
    });
}

#[test]
fn deterministic_generated_path_corpus_matches_shared_path_admission() {
    run_checked_test(|| -> Result<(), String> {
        let pieces = [
            "alpha",
            "B",
            "\u{f1}",
            "n\u{303}",
            "\u{1f600}",
            "-",
            "%2e",
            "folder",
            ".",
            "..",
            "",
            "/",
            "\\",
            "\0",
            "\n",
            "\t",
            "\r",
            "nested/path",
        ];
        let mut checked = 0usize;
        for index in 0..1_024usize {
            let first = *pieces
                .get(index.rem_euclid(pieces.len()))
                .ok_or("path piece index")?;
            let second = *pieces
                .get(
                    index
                        .checked_div(17)
                        .ok_or("finite stride")?
                        .rem_euclid(pieces.len()),
                )
                .ok_or("second piece")?;
            let prefix = if index.rem_euclid(2) == 0 {
                "assets/"
            } else {
                "sections/"
            };
            let name = format!("{prefix}{first}{second}.bin");
            let root = member("manifest.json", b"{}");
            let content = member(&name, b"data");
            let candidates = if name.as_str() < "manifest.json" {
                [content, root]
            } else {
                [root, content]
            };
            let expected_admission = profile_entry_kind(&name).is_ok();
            let actual = write_stored_profile_zip(&candidates, limits());
            let mut output = [0x5cu8; 1_024];
            let bounded = write_stored_profile_zip_into(
                &candidates,
                limits(),
                &mut output,
            );
            if expected_admission {
                let encoded = actual.map_err(|error| {
                    format!("shared path validator admitted name: {error:?}")
                })?;
                assert_eq!(bounded, Ok(encoded.len()), "candidate {name:?}");
                assert_eq!(
                    output.get(..encoded.len()).ok_or("output range")?,
                    encoded.as_slice()
                );
                assert!(
                    output
                        .get(encoded.len()..)
                        .ok_or("output suffix")?
                        .iter()
                        .all(|byte| *byte == 0x5c)
                );
                let parsed = parse_bytes(&encoded)?;
                assert!(
                    parsed.entries().iter().any(|entry| entry.name() == name
                        && entry.bytes() == b"data")
                );
            } else {
                assert_eq!(
                    actual,
                    Err(ProfileStoredZipWriteError::InvalidName),
                    "invalid candidate {name:?}"
                );
                assert_eq!(
                    bounded,
                    Err(ProfileStoredZipWriteError::InvalidName),
                    "bounded candidate {name:?}"
                );
                assert_eq!(
                    output, [0x5c; 1_024],
                    "unsafe path cannot change caller-owned destination"
                );
            }
            checked = checked.saturating_add(1);
        }
        assert_eq!(checked, 1_024);
        Ok(())
    });
}

#[test]
fn seeded_one_byte_zip_mutations_cannot_change_admitted_payload_identity() {
    run_checked_test(|| -> Result<(), String> {
        let source = [
            member("assets/zeros.webp", &[0, 0, 0, 0, 0]),
            member("manifest.json", b"{\"unknown\":true}"),
            member("sections/strokes.json", b"[0,1,2,3,4,5]"),
        ];
        let original =
            write_stored_profile_zip(&source, limits()).map_err(|error| {
                format!("original canonical ZIP framing: {error:?}")
            })?;
        let original_reader = parse_bytes(&original)?;
        let bounds = ProfileZipReadLimits {
            maximum_archive_bytes: original.len(),
            maximum_entries: source.len(),
            maximum_entry_bytes: 100,
            maximum_total_bytes: 100,
        };
        let mut seed = 0x5354_4f52_4544_5a49u64;
        let mut checked = 0usize;
        for _ in 0..4_096usize {
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let position = usize::try_from(
                seed.rem_euclid(
                    u64::try_from(original.len())
                        .map_err(|error| format!("source length: {error:?}"))?,
                ),
            )
            .map_err(|error| {
                format!("bounded seed position fits usize: {error:?}")
            })?;
            let mut changed = original.clone();
            let bit = u8::try_from((seed >> 24).rem_euclid(8))
                .map_err(|error| format!("bit index: {error:?}"))?;
            let before =
                *changed.get(position).ok_or("bounded mutated offset")?;
            if let Some(slot) = changed.get_mut(position) {
                *slot = before ^ (1u8 << bit);
            }
            if let Ok(parsed) = read_stored_profile_zip(&changed, bounds) {
                for (original_entry, after) in
                    original_reader.entries().iter().zip(parsed.entries())
                {
                    assert_eq!(after.name(), original_entry.name());
                    assert_eq!(after.bytes(), original_entry.bytes());
                }
                assert_eq!(
                    parsed.entries().len(),
                    original_reader.entries().len()
                );
            }
            checked = checked.saturating_add(1);
        }
        assert_eq!(checked, 4_096);
        Ok(())
    });
}

#[test]
fn independent_python_multimember_zipfile_golden_sha256_is_stable() {
    run_checked_test(|| -> Result<(), String> {
        // An external Python zipfile reader verified the names, content, CRC,
        // UTF-8 flag, stored method and absence of extras/comments. Its hashlib
        // digest freezes this exact multi-member encoding independently.
        let image: &[u8] = &[0, 0xff, 1, 0x80, 0x7f, 0];
        let source = [
            member("assets/empty.webp", b""),
            member("assets/\u{f1}.webp", image),
            member("manifest.json", b"{\"test\":true}"),
            member("sections/\u{1f600}.json", b"[12,34,56]"),
        ];
        let archived =
            write_stored_profile_zip(&source, limits()).map_err(|error| {
                format!("fixed deterministic four-member ZIP: {error:?}")
            })?;
        assert_eq!(archived.len(), 479);
        let digest = sha256_profile_bytes(&archived).bytes();
        let encoded = hex_digest(&digest)?;
        assert_eq!(
            encoded,
            "1451e12dd97e666c89c7f752c92b3530520b1bbdc07df3cc244959b52d986553"
        );
        let parsed = parse_bytes(&archived)?;
        assert_eq!(parsed.source_bytes(), archived);
        assert!(ptr::eq(parsed.source_bytes().as_ptr(), archived.as_ptr()));
        assert_eq!(parsed.source_sha256().bytes(), digest);
        let root = parsed
            .admitted_root_json(ProfileJsonSyntaxLimits {
                maximum_bytes: 100,
                maximum_nesting: 3,
                maximum_object_members: 4,
                maximum_values: 4,
            })
            .map_err(|error| {
                format!("source-borrowed root grammar is valid: {error:?}")
            })?;
        assert_eq!(
            root.source_sha256(),
            sha256_profile_bytes(b"{\"test\":true}")
        );
        assert_ne!(root.source_sha256(), parsed.source_sha256());
        assert!(ptr::eq(
            ptr::from_ref(root.archive()),
            ptr::from_ref(&parsed)
        ));
        for (record, original) in parsed.entries().iter().zip(source) {
            assert_eq!(record.name(), original.name);
            assert_eq!(record.bytes(), original.bytes);
        }
        Ok(())
    });
}

#[test]
fn large_payload_local_and_central_crc_match_independent_zlib_oracle() {
    run_checked_test(|| -> Result<(), String> {
        // zlib.crc32(bytes(index.rem_euclid(2)56 for index in
        // range(1_048_576))) is 0x04d0_e435; Python's implementation is
        // independent of the writer.
        const PYTHON_ZLIB_CRC32: u32 = 0x04d0_e435;
        let bytes = (0usize..1_048_576)
            .map(|index| u8::try_from(index.rem_euclid(256)))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("bounded pattern: {error:?}"))?;
        let entry = [member("manifest.json", &bytes)];
        let bound = ProfileStoredZipWriteLimits {
            maximum_archive_bytes: 1_049_000,
            maximum_entries: 1,
            maximum_entry_bytes: 1_048_576,
            maximum_total_bytes: 1_048_576,
        };
        let planned = planned_stored_profile_zip_bytes(&entry, bound).map_err(
            |error| {
                format!(
                    "large stored payload fits finite ZIP32 budget: {error:?}"
                )
            },
        )?;
        let written =
            write_stored_profile_zip(&entry, bound).map_err(|error| {
                format!("exact preflighted ZIP output: {error:?}")
            })?;
        assert_eq!(written.len(), planned);
        assert_eq!(planned, bytes.len() + 124);
        let local_crc = u32::from_le_bytes(
            written
                .get(14..18)
                .ok_or("complete local CRC header")?
                .try_into()
                .map_err(|error| {
                    format!("exact 4-byte CRC field: {error:?}")
                })?,
        );
        let eocd = written.len() - 22;
        let central_start: usize = u32::from_le_bytes(
            written
                .get(eocd + 16..eocd + 20)
                .ok_or("complete end record central offset")?
                .try_into()
                .map_err(|error| {
                    format!("exact central offset field: {error:?}")
                })?,
        )
        .try_into()
        .map_err(|error| format!("ZIP offset: {error:?}"))?;
        let central_crc = u32::from_le_bytes(
            written
                .get(central_start + 16..central_start + 20)
                .ok_or("complete central CRC header")?
                .try_into()
                .map_err(|error| {
                    format!("exact 4-byte CRC field: {error:?}")
                })?,
        );
        assert_eq!(local_crc, PYTHON_ZLIB_CRC32);
        assert_eq!(central_crc, PYTHON_ZLIB_CRC32);
        let parsed = read_stored_profile_zip(&written, ProfileZipReadLimits {
            maximum_archive_bytes: written.len(),
            maximum_entries: 1,
            maximum_entry_bytes: u64::try_from(bytes.len())
                .map_err(|error| format!("byte length: {error:?}"))?,
            maximum_total_bytes: u64::try_from(bytes.len())
                .map_err(|error| format!("byte length: {error:?}"))?,
        })
        .map_err(|error| {
            format!("strict parser accepts both matching CRC fields: {error:?}")
        })?;
        assert_eq!(
            parsed
                .entries()
                .first()
                .ok_or("missing parsed record")?
                .bytes(),
            bytes
        );
        Ok(())
    });
}

#[test]
fn largest_ordinary_zip32_entry_count_round_trips_without_zip64() {
    run_checked_test(|| -> Result<(), String> {
        let maximum_ordinary_entries = usize::from(u16::MAX) - 1;
        let names = (0..(maximum_ordinary_entries - 1))
            .map(|index| format!("assets/{index:05}.bin"))
            .collect::<Vec<_>>();
        let mut members = names
            .iter()
            .map(|name| member(name, b""))
            .collect::<Vec<_>>();
        members.push(member("manifest.json", b"{}"));
        let limits = ProfileStoredZipWriteLimits {
            maximum_archive_bytes: 8_000_000,
            maximum_entries: maximum_ordinary_entries,
            maximum_entry_bytes: 2,
            maximum_total_bytes: 2,
        };
        let planned = planned_stored_profile_zip_bytes(&members, limits)
            .map_err(|error| {
                format!(
                    "maximal ordinary count is still representable: {error:?}"
                )
            })?;
        let bytes = write_stored_profile_zip(&members, limits)
            .map_err(|error| format!("maximum ZIP32 count: {error:?}"))?;
        assert_eq!(bytes.len(), planned);
        let mut caller_owned = vec![0x91u8; planned + 1];
        assert_eq!(
            write_stored_profile_zip_into(
                &members,
                limits,
                caller_owned
                    .get_mut(..planned - 1)
                    .ok_or("caller_owned mutable range")?,
            ),
            Err(ProfileStoredZipWriteError::BufferTooSmall {
                required: planned
            })
        );
        assert!(caller_owned.iter().all(|byte| *byte == 0x91));
        assert_eq!(
            write_stored_profile_zip_into(&members, limits, &mut caller_owned,),
            Ok(planned)
        );
        assert_eq!(
            caller_owned.get(..planned).ok_or("caller_owned range")?,
            bytes.as_slice()
        );
        assert_eq!(
            *caller_owned.get(planned).ok_or("caller output sentinel")?,
            0x91
        );
        let eocd = bytes.len() - 22;
        for offset in [8usize, 10] {
            assert_eq!(
                bytes
                    .get(eocd + offset..eocd + offset + 2)
                    .ok_or("bytes range")?,
                &(u16::MAX - 1).to_le_bytes()
            );
        }
        verify_maximum_reader_identity(
            &bytes,
            planned,
            maximum_ordinary_entries,
        );
        verify_maximum_reader_limits(&bytes, planned, maximum_ordinary_entries);
        assert_eq!(
            planned_stored_profile_zip_bytes(
                &members,
                ProfileStoredZipWriteLimits {
                    maximum_archive_bytes: planned - 1,
                    ..limits
                }
            ),
            Err(ProfileStoredZipWriteError::ArchiveTooLarge),
        );
        assert_eq!(
            write_stored_profile_zip(&members, ProfileStoredZipWriteLimits {
                maximum_entries: maximum_ordinary_entries - 1,
                ..limits
            }),
            Err(ProfileStoredZipWriteError::ResourceLimit),
        );
        Ok(())
    });
}

#[test]
fn legacy_stored_headers_round_trip_to_writer_framing_without_payload_edits() {
    run_checked_test(|| -> Result<(), String> {
        let members = [
            member("assets/source.webp", b"\x00\x81\xff"),
            member("manifest.json", br#"{"kept":true}"#),
        ];
        let canonical = write_stored_profile_zip(&members, limits())
            .map_err(|error| format!("fixed-metadata ZIP output: {error:?}"))?;
        let central: usize = u32::from_le_bytes(
            canonical
                .get(canonical.len() - 6..canonical.len() - 2)
                .ok_or("canonical range")?
                .try_into()
                .map_err(|error| format!("EOCD central offset: {error:?}"))?,
        )
        .try_into()
        .map_err(|error| format!("ZIP offset: {error:?}"))?;
        let mut legacy = canonical.clone();
        // Both fields describe the same first member, so the strict reader may
        // admit this legacy DOS timestamp without treating it as writer output.
        legacy
            .get_mut(10..12)
            .ok_or("legacy range")?
            .copy_from_slice(&[0x15, 0x00]);
        legacy
            .get_mut(central + 12..central + 14)
            .ok_or("legacy range")?
            .copy_from_slice(&[0x15, 0x00]);
        let checked = parse_bytes(&legacy)?;
        assert!(!checked.matches_fixed_writer_metadata());
        assert!(!checked.matches_stored_writer_framing());
        assert_eq!(checked.entries().len(), members.len());
        let recovered = checked
            .entries()
            .iter()
            .map(|entry| member(entry.name(), entry.bytes()))
            .collect::<Vec<_>>();
        let canonical_reframe = write_stored_profile_zip(&recovered, limits())
            .map_err(|error| format!("reframed ZIP metadata: {error:?}"))?;
        assert_eq!(canonical_reframe, canonical);
        assert_ne!(canonical_reframe, legacy);
        assert_eq!(checked.source_sha256(), sha256_profile_bytes(&legacy));
        assert_ne!(checked.source_sha256(), sha256_profile_bytes(&canonical));
        assert_eq!(
            checked
                .entries()
                .get(1)
                .ok_or("missing checked record")?
                .bytes(),
            members.get(1).ok_or("missing members record")?.bytes
        );
        assert!(
            parse_bytes(&canonical_reframe)?.matches_stored_writer_framing()
        );
        Ok(())
    });
}

#[test]
fn every_accepted_legacy_header_variant_reframes_to_exact_writer_bytes() {
    run_checked_test(|| -> Result<(), String> {
        let selected = [
            member("assets/mark.webp", b"\0\x81\xff"),
            member("manifest.json", b"{}"),
            member("sections/ink.json", b"[1,2,3]"),
        ];
        let canonical =
            write_stored_profile_zip(&selected, limits()).map_err(|error| {
                format!("three-member exact stored ZIP output: {error:?}")
            })?;
        let canonical_archive = parse_bytes(&canonical)?;
        assert!(canonical_archive.matches_stored_writer_framing());
        let central: usize = u32::from_le_bytes(
            canonical
                .get(canonical.len() - 6..canonical.len() - 2)
                .ok_or("canonical range")?
                .try_into()
                .map_err(|error| format!("EOCD central offset: {error:?}"))?,
        )
        .try_into()
        .map_err(|error| format!("ZIP offset: {error:?}"))?;
        let mut local_offset = 0usize;
        let mut central_offset = central;
        let mut checked = 0usize;
        for (index, entry) in selected.iter().enumerate() {
            for (case, local_field, central_field, value) in [
                ("creator", None, 4usize, [10, 0]),
                ("extractor", Some(4usize), 6, [10, 0]),
                ("ASCII UTF-8 flag", Some(6), 8, [0, 0]),
                ("DOS time", Some(10), 12, [3, 0]),
                ("DOS date", Some(12), 14, [3, 0]),
            ] {
                let mut changed = canonical.clone();
                if let Some(field) = local_field {
                    changed
                        .get_mut(local_offset + field..local_offset + field + 2)
                        .ok_or("changed range")?
                        .copy_from_slice(&value);
                }
                changed
                    .get_mut(
                        central_offset + central_field
                            ..central_offset + central_field + 2,
                    )
                    .ok_or("central metadata field")?
                    .copy_from_slice(&value);
                let archive = parse_bytes(&changed)?;
                assert_eq!(
                    archive.entries(),
                    canonical_archive.entries(),
                    "case={case} member={index}"
                );
                assert!(
                    !archive.matches_fixed_writer_metadata(),
                    "case={case} member={index}"
                );
                assert!(
                    !archive.matches_stored_writer_framing(),
                    "case={case} member={index}"
                );
                assert_ne!(
                    archive.source_sha256(),
                    canonical_archive.source_sha256()
                );
                let members = archive
                    .entries()
                    .iter()
                    .map(|item| member(item.name(), item.bytes()))
                    .collect::<Vec<_>>();
                let output = write_stored_profile_zip(&members, limits())
                    .map_err(|error| {
                        format!(
                            "admitted original payloads can reframe: {error:?}"
                        )
                    })?;
                assert_eq!(output, canonical, "case={case} member={index}");
                assert!(parse_bytes(&output)?.matches_stored_writer_framing());
                checked += 1;
            }
            local_offset += 30 + entry.name.len() + entry.bytes.len();
            central_offset += 46 + entry.name.len();
        }
        assert_eq!(checked, 15);
        assert_eq!(local_offset, central);
        Ok(())
    });
}

#[test]
fn generated_stored_writer_framing_equivalence_is_byte_exact() {
    run_checked_test(|| -> Result<(), String> {
        const PATHS: [&str; 7] = [
            "assets/000.bin",
            "assets/\u{f1}.webp",
            "assets/\u{1f58c}\u{fe0f}.webp",
            "manifest.json",
            "sections/000.json",
            "sections/\u{df}.json",
            "sections/\u{1f600}.json",
        ];
        let generous = ProfileStoredZipWriteLimits {
            maximum_archive_bytes: 100_000,
            maximum_entries: PATHS.len(),
            maximum_entry_bytes: 1_024,
            maximum_total_bytes: 3_000,
        };
        let mut seed = 0x4154_5241_4d45_4e54u64;
        let mut cases = 0usize;
        for case in 0..1_024usize {
            let mut selected = Vec::new();
            let mut owned = Vec::new();
            for (index, path) in PATHS.iter().enumerate() {
                seed = seed
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                if index != 3 && seed.rem_euclid(4) == 0 {
                    continue;
                }
                let size = if index == 3 {
                    // The byte writer never parses root JSON: some roots are
                    // intentionally invalid while their ZIP framing stays
                    // exact.
                    *[0usize, 2, 7, 31]
                        .get(case.rem_euclid(4))
                        .ok_or("root size")?
                } else {
                    usize::try_from((seed >> 19).rem_euclid(161))
                        .map_err(|error| format!("payload length: {error:?}"))?
                };
                let mut payload = Vec::with_capacity(size);
                for _ in 0..size {
                    seed = seed
                        .wrapping_mul(2_862_933_555_777_941_757)
                        .wrapping_add(3_037_000_493);
                    payload.push(
                        u8::try_from((seed >> 48).rem_euclid(256)).map_err(
                            |error| format!("payload byte: {error:?}"),
                        )?,
                    );
                }
                owned.push((path, payload));
            }
            for (path, payload) in &owned {
                selected.push(member(path, payload));
            }
            assert!(selected.iter().any(|item| item.name == "manifest.json"));
            verify_generated_frame(&selected, generous, case);
            cases += 1;
        }
        assert_eq!(cases, 1_024);
        Ok(())
    });
}

#[test]
fn caller_owned_zip_output_is_atomic_for_too_small_storage() {
    run_checked_test(|| -> Result<(), String> {
        let members = [
            member("assets/\u{f1}.webp", b"\x00\xff\x80"),
            member("manifest.json", br#"{"spacing":true}"#),
            member("sections/\u{1f600}.json", b"[1,2,3]"),
        ];
        let expected =
            write_stored_profile_zip(&members, limits()).map_err(|error| {
                format!("owned reference emits one exact ZIP frame: {error:?}")
            })?;
        let required = planned_stored_profile_zip_bytes(&members, limits())
            .map_err(|error| format!("exact ZIP32 byte planning: {error:?}"))?;
        assert_eq!(expected.len(), required);
        let mut storage = vec![0xabu8; required + 17];
        for available in [0usize, 1, 22, required - 1] {
            assert_eq!(
                write_stored_profile_zip_into(
                    &members,
                    limits(),
                    storage
                        .get_mut(..available)
                        .ok_or("storage mutable range")?,
                ),
                Err(ProfileStoredZipWriteError::BufferTooSmall { required }),
                "available bytes={available}"
            );
            assert!(
                storage.iter().all(|byte| *byte == 0xab),
                "undersized storage must not change any byte"
            );
        }
        assert_eq!(
            write_stored_profile_zip_into(
                &members,
                limits(),
                storage.get_mut(..required).ok_or("storage mutable range")?,
            ),
            Ok(required)
        );
        assert_eq!(
            storage.get(..required).ok_or("storage range")?,
            expected.as_slice()
        );
        assert!(
            storage
                .get(required..)
                .ok_or("storage suffix")?
                .iter()
                .all(|byte| *byte == 0xab)
        );
        storage.fill(0xcd);
        assert_eq!(
            write_stored_profile_zip_into(&members, limits(), &mut storage,),
            Ok(required)
        );
        assert_eq!(
            storage.get(..required).ok_or("storage range")?,
            expected.as_slice()
        );
        assert_eq!(
            storage.get(required..).ok_or("storage range")?,
            &[0xcd; 17]
        );
        let archive =
            parse_bytes(storage.get(..required).ok_or("storage range")?)?;
        assert!(archive.matches_stored_writer_framing());
        assert_eq!(archive.source_sha256(), sha256_profile_bytes(&expected));
        Ok(())
    });
}

#[test]
fn caller_owned_zip_output_checks_input_before_destination_capacity() {
    run_checked_test(|| -> Result<(), String> {
        let missing = [member("sections/alone.json", b"[]")];
        let unordered = [
            member("manifest.json", b"{}"),
            member("assets/file.webp", b"x"),
        ];
        let invalid = [
            member("manifest.json", b"{}"),
            member("sections/../unsafe.json", b"bad"),
        ];
        let valid = [member("manifest.json", b"{}")];
        let cases = [
            (
                &missing[..],
                limits(),
                ProfileStoredZipWriteError::MissingManifest,
            ),
            (
                &unordered[..],
                limits(),
                ProfileStoredZipWriteError::NonCanonicalOrder,
            ),
            (
                &invalid[..],
                limits(),
                ProfileStoredZipWriteError::InvalidName,
            ),
            (
                &valid[..],
                ProfileStoredZipWriteLimits {
                    maximum_entries: 0,
                    ..limits()
                },
                ProfileStoredZipWriteError::ResourceLimit,
            ),
            (
                &valid[..],
                ProfileStoredZipWriteLimits {
                    maximum_archive_bytes: 1,
                    ..limits()
                },
                ProfileStoredZipWriteError::ArchiveTooLarge,
            ),
        ];
        for (members, bound, expected) in cases {
            let mut sentinel = [0x8cu8; 64];
            assert_eq!(
                write_stored_profile_zip_into(members, bound, &mut sentinel,),
                Err(expected)
            );
            assert_eq!(
                sentinel, [0x8c; 64],
                "invalid input must never reach destination writing"
            );
        }
        Ok(())
    });
}

#[test]
fn input_member_debug_never_prints_private_archive_paths_or_payload() {
    run_checked_test(|| -> Result<(), String> {
        let entry = member(
            "sections/private-path-marker-8571.json",
            b"{\"private\":\"payload-secret-2765\"}",
        );
        for displayed in [format!("{entry:?}"), format!("{entry:#?}")] {
            assert!(displayed.contains("ProfileStoredZipMember"));
            assert!(!displayed.contains("private-path-marker-8571"));
            assert!(!displayed.contains("payload-secret-2765"));
            assert!(!displayed.contains("112, 97, 121, 108, 111, 97, 100"));
        }
        assert_eq!(entry.name, "sections/private-path-marker-8571.json");
        assert_eq!(entry.bytes, b"{\"private\":\"payload-secret-2765\"}");
        Ok(())
    });
}
