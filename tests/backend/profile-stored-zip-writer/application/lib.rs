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

use atrament_handwriting_profile_container::{
    PROFILE_CONTAINER_VERSION, PROFILE_MANIFEST_PATH, ProfileManifest,
    ProfileManifestEntry, profile_entry_kind, validate_profile_manifest_view,
};
use atrament_profile_entry_sha256::sha256_profile_bytes;
use atrament_profile_json_syntax::ProfileJsonSyntaxLimits;
use atrament_profile_stored_zip_writer::{
    ProfileStoredZipMember, ProfileStoredZipWriteError,
    ProfileStoredZipWriteLimits, planned_stored_profile_zip_bytes,
    write_stored_profile_zip,
};
use atrament_profile_zip_reader::{
    ProfileZipReadLimits, read_stored_profile_zip,
    verify_admitted_profile_zip_payloads,
};

fn limits() -> ProfileStoredZipWriteLimits {
    ProfileStoredZipWriteLimits {
        maximum_archive_bytes: 100_000,
        maximum_entries: 256,
        maximum_entry_bytes: 65_536,
        maximum_total_bytes: 100_000,
    }
}

fn parse_bytes(
    bytes: &[u8],
) -> atrament_profile_zip_reader::ProfileZipArchive<'_> {
    read_stored_profile_zip(
        bytes,
        ProfileZipReadLimits {
            maximum_archive_bytes: bytes.len(),
            maximum_entries: 256,
            maximum_entry_bytes: 65_536,
            maximum_total_bytes: 100_000,
        },
    )
    .expect("emitted ordinary framing accepted by independent strict reader")
}

fn member<'a>(name: &'a str, bytes: &'a [u8]) -> ProfileStoredZipMember<'a> {
    ProfileStoredZipMember { bytes, name }
}

#[test]
fn stable_root_only_archive_has_correct_fixed_header_and_directory() {
    let members = [member(PROFILE_MANIFEST_PATH, b"{}")];
    let first =
        write_stored_profile_zip(&members, limits()).expect("single ZIP root");
    let second = write_stored_profile_zip(&members, limits())
        .expect("deterministic ZIP output");
    assert_eq!(first, second);
    assert_eq!(first.len(), 30 + 13 + 2 + 46 + 13 + 22);
    assert_eq!(first.get(..4), Some(&[0x50, 0x4b, 0x03, 0x04][..]));
    assert_eq!(first.get(4..6), Some(&[20, 0][..]));
    assert_eq!(first.get(6..8), Some(&[0, 8][..]));
    assert_eq!(first.get(8..14), Some(&[0; 6][..]));
    assert_eq!(first.get(14..18), Some(&[0x43, 0xbf, 0xa6, 0xa3][..]));
    assert_eq!(first.get(26..30), Some(&[13, 0, 0, 0][..]));
    let archive = parse_bytes(&first);
    assert_eq!(archive.entries().len(), 1);
    assert_eq!(archive.manifest_utf8(2), Ok("{}"));
    assert_eq!(archive.entries()[0].crc32(), 0xa3a6_bf43);
}

#[test]
fn sorted_utf8_names_and_empty_asset_round_trip_byte_for_byte() {
    let entries = [
        member("assets/empty.webp", b""),
        member("assets/ñ.webp", b"\x01\x00\xff"),
        member("manifest.json", b"{\"example\":true}"),
        member("sections/😀.json", br#"[0,1,2]"#),
    ];
    let output = write_stored_profile_zip(&entries, limits())
        .expect("canonical name order");
    let parsed = parse_bytes(&output);
    assert_eq!(parsed.entries().len(), entries.len());
    for (admitted, original) in parsed.entries().iter().zip(entries) {
        assert_eq!(admitted.name(), original.name);
        assert_eq!(admitted.bytes(), original.bytes);
    }
    assert_eq!(parsed.entries()[0].bytes(), b"");
    let duplicate = write_stored_profile_zip(&entries, limits())
        .expect("same inputs produce same output bytes");
    assert_eq!(output, duplicate);
}

#[test]
fn missing_root_is_not_a_complete_stored_archive_candidate() {
    for candidates in [
        vec![],
        vec![member("assets/valid.webp", b"abc")],
        vec![member("sections/x.json", b"{}")],
    ] {
        assert_eq!(
            write_stored_profile_zip(&candidates, limits()),
            Err(ProfileStoredZipWriteError::MissingManifest)
        );
    }
}

#[test]
fn repeated_reverse_and_near_duplicate_names_are_rejected() {
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
}

#[test]
fn unsafe_member_names_are_rejected_without_canonicalizing_them() {
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
                Err(ProfileStoredZipWriteError::InvalidName)
                    | Err(ProfileStoredZipWriteError::NonCanonicalOrder)
            ),
            "unsafe member {name:?} was admitted: {result:?}"
        );
    }
}

#[test]
fn exact_budget_ceiling_accepts_and_one_byte_short_rejects() {
    let members = [member("manifest.json", b"123456789")];
    let bytes =
        write_stored_profile_zip(&members, limits()).expect("bounded source");
    let exact = ProfileStoredZipWriteLimits {
        maximum_archive_bytes: bytes.len(),
        maximum_entries: 1,
        maximum_entry_bytes: 9,
        maximum_total_bytes: 9,
    };
    assert_eq!(write_stored_profile_zip(&members, exact), Ok(bytes.clone()));
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
}

#[test]
fn fixed_output_crc_changes_when_bytes_change_even_if_name_does_not() {
    let first = write_stored_profile_zip(
        &[member("manifest.json", b"{\"id\":1}")],
        limits(),
    )
    .expect("original");
    let second = write_stored_profile_zip(
        &[member("manifest.json", b"{\"id\":2}")],
        limits(),
    )
    .expect("changed root");
    assert_eq!(first.len(), second.len());
    assert_ne!(first, second);
    assert_ne!(
        parse_bytes(&first).entries()[0].crc32(),
        parse_bytes(&second).entries()[0].crc32()
    );
}

#[test]
fn maximum_member_count_is_preflighted_before_allocating_archive() {
    let members = [member("manifest.json", b"{}")];
    assert_eq!(
        write_stored_profile_zip(
            &members,
            ProfileStoredZipWriteLimits {
                maximum_entries: 0,
                ..limits()
            }
        ),
        Err(ProfileStoredZipWriteError::ResourceLimit)
    );
}

#[test]
fn golden_root_zip_matches_independent_struct_and_crc_reference() {
    // Generated separately using Python's struct, zlib and zipfile modules.
    // No ZIP code or digest helpers from this crate produced the oracle.
    const GOLDEN: &str = concat!(
        "504b03041400000800000000000043bfa6a302000000020000000d0000006d61",
        "6e69666573742e6a736f6e7b7d504b010214001400000800000000000043bfa6",
        "a302000000020000000d00000000000000000000000000000000006d616e6966",
        "6573742e6a736f6e504b050600000000010001003b0000002d0000000000",
    );
    let bytes =
        write_stored_profile_zip(&[member("manifest.json", b"{}")], limits())
            .expect("root-only fixed ZIP");
    let encoded = bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    assert_eq!(encoded, GOLDEN);
}

#[test]
fn zip_entry_count_sentinel_refuses_65_535_members_without_output() {
    let members = vec![member("manifest.json", b"{}"); usize::from(u16::MAX)];
    let limit = ProfileStoredZipWriteLimits {
        maximum_entries: members.len(),
        ..limits()
    };
    assert_eq!(
        write_stored_profile_zip(&members, limit),
        Err(ProfileStoredZipWriteError::UnsupportedZip64)
    );
}

#[test]
fn utf8_member_name_field_limit_is_enforced_on_byte_length() {
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
        .expect("maximal ZIP filename fits in u16");
    let parsed = read_stored_profile_zip(
        &archive_bytes,
        ProfileZipReadLimits {
            maximum_archive_bytes: 200_000,
            maximum_entries: 2,
            maximum_entry_bytes: 100,
            maximum_total_bytes: 100,
        },
    )
    .expect("strict reader accepts 65535 UTF-8 bytes");
    assert_eq!(parsed.entries()[0].name(), maximum_name);
    let overflow = [member(&too_long, b"x"), member("manifest.json", b"{}")];
    assert_eq!(
        write_stored_profile_zip(&overflow, limit),
        Err(ProfileStoredZipWriteError::InvalidName)
    );
}

#[test]
fn many_generated_sorted_payloads_are_stable_and_reader_admitted() {
    for size in [1usize, 2, 3, 7, 17, 31, 64, 127] {
        let names = (0..size)
            .map(|index| format!("assets/{index:04}.bin"))
            .collect::<Vec<_>>();
        let contents = (0..size)
            .map(|index| {
                (0..(index % 129))
                    .map(|value| (value as u8).wrapping_mul(17))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let mut members = names
            .iter()
            .zip(contents.iter())
            .map(|(name, content)| member(name.as_str(), content.as_slice()))
            .collect::<Vec<_>>();
        members.push(member("manifest.json", b"{}"));
        let output = write_stored_profile_zip(&members, limits())
            .expect("bounded sorted members");
        let parsed = parse_bytes(&output);
        assert_eq!(parsed.entries().len(), members.len());
        for (expected, observed) in members.iter().zip(parsed.entries()) {
            assert_eq!(observed.name(), expected.name);
            assert_eq!(observed.bytes(), expected.bytes);
        }
        assert_eq!(write_stored_profile_zip(&members, limits()), Ok(output));
    }
}

#[test]
fn emitted_archive_can_verify_complete_typed_inventory_and_sha256() {
    let members = [
        member("assets/image.webp", b"\x00\xff\x01\x80"),
        member("manifest.json", b"{}"),
        member("sections/sample.json", br#"{"strokes":[1,2,3]}"#),
    ];
    let byte_archive = write_stored_profile_zip(&members, limits())
        .expect("deterministic ZIP framing");
    let parsed = parse_bytes(&byte_archive);
    let typed = ProfileManifest {
        container_version: PROFILE_CONTAINER_VERSION.into(),
        entries: vec![
            ProfileManifestEntry {
                byte_length: 4,
                digest: sha256_profile_bytes(members[0].bytes),
                media_type: "image/webp".into(),
                path: "assets/image.webp".into(),
            },
            ProfileManifestEntry {
                byte_length: u64::try_from(members[2].bytes.len())
                    .expect("fixed fixture fits u64"),
                digest: sha256_profile_bytes(members[2].bytes),
                media_type: "application/json".into(),
                path: "sections/sample.json".into(),
            },
        ],
        optional_features: vec![],
        profile_identity: "writer-reader-sha-fixture".into(),
        required_features: vec![],
    };
    let declared = validate_profile_manifest_view(&typed, &[])
        .expect("typed declaration is valid independently");
    let names = parsed.observed_paths();
    let inventory = declared
        .admit_entry_inventory(&names)
        .expect("exact sorted archive member inventory");
    let verified = verify_admitted_profile_zip_payloads(&parsed, inventory)
        .expect("all emitted members match typed digests");
    assert_eq!(verified.verified_entry_count(), 2);
    assert_eq!(
        verified.verified_entry_bytes("assets/image.webp"),
        Some(members[0].bytes)
    );
    assert_eq!(
        verified.verified_entry_bytes("sections/sample.json"),
        Some(members[2].bytes)
    );
    assert_eq!(verified.verified_entry_bytes("manifest.json"), None);
    assert_eq!(verified.verified_entry_bytes("sections/SAMPLE.json"), None);
}

#[test]
fn framing_validity_does_not_imply_manifest_json_syntax_admission() {
    let source = [member("manifest.json", b"{broken syntax...")];
    let bytes = write_stored_profile_zip(&source, limits())
        .expect("opaque root allowed at ZIP framing level");
    let parsed = parse_bytes(&bytes);
    assert_eq!(parsed.manifest_utf8(20), Ok("{broken syntax..."));
    let json = parsed.manifest_json_object(ProfileJsonSyntaxLimits {
        maximum_bytes: 20,
        maximum_nesting: 10,
        maximum_object_members: 10,
        maximum_values: 10,
    });
    assert!(json.is_err(), "ZIP framing is not typed JSON admission");
}

#[test]
fn exact_framing_size_plan_matches_every_written_output() {
    for count in [1usize, 2, 3, 4, 7, 23, 64, 127, 255] {
        let names = (0..count.saturating_sub(1))
            .map(|i| format!("assets/{i:05}.bin"))
            .collect::<Vec<_>>();
        let bytes = (0..count.saturating_sub(1))
            .map(|i| vec![u8::try_from(i % 255).expect("in range"); i % 99])
            .collect::<Vec<_>>();
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
        let expected_len = planned_stored_profile_zip_bytes(&members, bound)
            .expect("all fixed records fit admitted budgets");
        let actual = write_stored_profile_zip(&members, bound)
            .expect("preflighted writer admits same members");
        assert_eq!(actual.len(), expected_len, "count={count}");
        assert_eq!(parse_bytes(&actual).entries().len(), count);
        let exact_bound = ProfileStoredZipWriteLimits {
            maximum_archive_bytes: expected_len,
            ..bound
        };
        assert_eq!(
            planned_stored_profile_zip_bytes(&members, exact_bound),
            Ok(expected_len)
        );
        assert_eq!(write_stored_profile_zip(&members, exact_bound), Ok(actual));
    }
}

#[test]
fn dry_run_and_writer_share_fail_closed_preflight_error_categories() {
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
    let cases: [(&[ProfileStoredZipMember<'_>], ProfileStoredZipWriteLimits);
        10] = [
        (
            &good,
            ProfileStoredZipWriteLimits {
                maximum_archive_bytes: 0,
                ..limits()
            },
        ),
        (
            &good,
            ProfileStoredZipWriteLimits {
                maximum_entries: 0,
                ..limits()
            },
        ),
        (
            &good,
            ProfileStoredZipWriteLimits {
                maximum_entry_bytes: 1,
                ..limits()
            },
        ),
        (
            &good,
            ProfileStoredZipWriteLimits {
                maximum_total_bytes: 1,
                ..limits()
            },
        ),
        (&duplicates, limits()),
        (&invalid_path, limits()),
        (&reversed, limits()),
        (&missing, limits()),
        (&empty, limits()),
        (
            &good,
            ProfileStoredZipWriteLimits {
                maximum_archive_bytes: 1,
                ..limits()
            },
        ),
    ];
    for (input, bound) in cases {
        let planned = planned_stored_profile_zip_bytes(input, bound);
        let written = write_stored_profile_zip(input, bound);
        assert!(planned.is_err());
        assert_eq!(planned, written.map(|bytes| bytes.len()));
    }
}

#[test]
fn deterministic_generated_path_corpus_matches_shared_path_admission() {
    let pieces = [
        "alpha",
        "B",
        "ñ",
        "ñ",
        "😀",
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
        let first = pieces[index % pieces.len()];
        let second = pieces[(index / 17) % pieces.len()];
        let prefix = if index % 2 == 0 {
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
        if expected_admission {
            let encoded = actual.expect("shared path validator admitted name");
            let parsed = parse_bytes(&encoded);
            assert!(
                parsed
                    .entries()
                    .iter()
                    .any(|entry| entry.name() == name
                        && entry.bytes() == b"data")
            );
        } else {
            assert_eq!(
                actual,
                Err(ProfileStoredZipWriteError::InvalidName),
                "invalid candidate {name:?}"
            );
        }
        checked = checked.saturating_add(1);
    }
    assert_eq!(checked, 1_024);
}

#[test]
fn seeded_one_byte_zip_mutations_cannot_change_admitted_payload_identity() {
    let source = [
        member("assets/zeros.webp", &[0, 0, 0, 0, 0]),
        member("manifest.json", b"{\"unknown\":true}"),
        member("sections/strokes.json", b"[0,1,2,3,4,5]"),
    ];
    let original = write_stored_profile_zip(&source, limits())
        .expect("original canonical ZIP framing");
    let original_reader = parse_bytes(&original);
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
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let position = usize::try_from(seed % original.len() as u64)
            .expect("bounded seed position fits usize");
        let mut changed = original.clone();
        let bit = u8::try_from((seed >> 24) % 8).expect("bit index");
        let before = *changed.get(position).expect("bounded mutated offset");
        if let Some(slot) = changed.get_mut(position) {
            *slot = before ^ (1u8 << bit);
        }
        if let Ok(parsed) = read_stored_profile_zip(&changed, bounds) {
            for (before, after) in
                original_reader.entries().iter().zip(parsed.entries())
            {
                assert_eq!(after.name(), before.name());
                assert_eq!(after.bytes(), before.bytes());
            }
            assert_eq!(parsed.entries().len(), original_reader.entries().len());
        }
        checked = checked.saturating_add(1);
    }
    assert_eq!(checked, 4_096);
}
