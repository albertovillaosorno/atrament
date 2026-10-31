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
    write_stored_profile_zip, write_stored_profile_zip_into,
};
use atrament_profile_zip_reader::{
    ProfileZipPathCopyError, ProfileZipReadError, ProfileZipReadLimits,
    read_stored_profile_zip, verify_admitted_profile_zip_payloads,
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
    assert!(archive.matches_fixed_writer_metadata());
    assert!(archive.matches_stored_writer_framing());
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
    assert!(parsed.matches_fixed_writer_metadata());
    assert!(parsed.matches_stored_writer_framing());
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
fn utf8_multibyte_names_obey_zip16_byte_limit_and_preserve_storage() {
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
        let repetitions = body_budget.checked_div(expected_width).unwrap_or(0);
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
        let write_outcome = write_stored_profile_zip(&entries, accepted_limit);
        assert!(
            plan_outcome.is_ok(),
            "byte-exact name must fit: {plan_outcome:?}"
        );
        assert!(
            write_outcome.is_ok(),
            "byte-exact name must fit: {write_outcome:?}"
        );
        let (Ok(planned), Ok(encoded)) = (plan_outcome, write_outcome) else {
            return;
        };
        assert_eq!(encoded.len(), planned);
        let read_outcome = read_stored_profile_zip(&encoded, read_limit);
        assert!(
            read_outcome.is_ok(),
            "strict reader must admit name: {read_outcome:?}"
        );
        let Ok(parsed) = read_outcome else {
            return;
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
        assert_eq!(parsed.source_bytes(), output);
        assert_eq!(parsed.source_sha256(), sha256_profile_bytes(&output));
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
    let mut output = vec![0xe3u8; byte_archive.len() + 7];
    assert_eq!(
        write_stored_profile_zip_into(&members, limits(), &mut output,),
        Ok(byte_archive.len())
    );
    assert_eq!(&output[..byte_archive.len()], byte_archive.as_slice());
    assert_eq!(&output[byte_archive.len()..], &[0xe3; 7]);
    let parsed = parse_bytes(&output[..byte_archive.len()]);
    assert!(atrament_handwriting_profile_container::
        validate_profile_archive_encoding_view(
            parsed.encoding_evidence(),
        ).is_ok());
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
    let mut names = ["unmodified"; 3];
    assert_eq!(parsed.copy_observed_paths(&mut names), Ok(3));
    let inventory = declared
        .admit_sorted_entry_inventory(&names)
        .expect("exact source-borrowed sorted ZIP inventory");
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
        let mut output = [0x5cu8; 1_024];
        let bounded =
            write_stored_profile_zip_into(&candidates, limits(), &mut output);
        if expected_admission {
            let encoded = actual.expect("shared path validator admitted name");
            assert_eq!(bounded, Ok(encoded.len()), "candidate {name:?}");
            assert_eq!(&output[..encoded.len()], encoded.as_slice());
            assert!(output[encoded.len()..].iter().all(|byte| *byte == 0x5c));
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

#[test]
fn independent_python_multimember_zipfile_golden_sha256_is_stable() {
    // An external Python zipfile reader verified the names, content, CRC,
    // UTF-8 flag, stored method and absence of extras/comments. Its hashlib
    // digest freezes this exact multi-member encoding independently.
    let image: &[u8] = &[0, 0xff, 1, 0x80, 0x7f, 0];
    let source = [
        member("assets/empty.webp", b""),
        member("assets/ñ.webp", image),
        member("manifest.json", b"{\"test\":true}"),
        member("sections/😀.json", b"[12,34,56]"),
    ];
    let archived = write_stored_profile_zip(&source, limits())
        .expect("fixed deterministic four-member ZIP");
    assert_eq!(archived.len(), 479);
    let digest = sha256_profile_bytes(&archived).bytes();
    let encoded = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    assert_eq!(
        encoded,
        "1451e12dd97e666c89c7f752c92b3530520b1bbdc07df3cc244959b52d986553"
    );
    let parsed = parse_bytes(&archived);
    assert_eq!(parsed.source_bytes(), archived);
    assert!(std::ptr::eq(
        parsed.source_bytes().as_ptr(),
        archived.as_ptr()
    ));
    assert_eq!(parsed.source_sha256().bytes(), digest);
    let root = parsed
        .admitted_root_json(ProfileJsonSyntaxLimits {
            maximum_bytes: 100,
            maximum_nesting: 3,
            maximum_object_members: 4,
            maximum_values: 4,
        })
        .expect("source-borrowed root grammar is valid");
    assert_eq!(
        root.source_sha256(),
        sha256_profile_bytes(b"{\"test\":true}")
    );
    assert_ne!(root.source_sha256(), parsed.source_sha256());
    assert!(std::ptr::eq(root.archive(), &parsed));
    for (record, original) in parsed.entries().iter().zip(source) {
        assert_eq!(record.name(), original.name);
        assert_eq!(record.bytes(), original.bytes);
    }
}

#[test]
fn large_payload_local_and_central_crc_match_independent_zlib_oracle() {
    // zlib.crc32(bytes(index % 256 for index in range(1_048_576)))
    // is 0x04d0_e435; Python's implementation is independent of the writer.
    const PYTHON_ZLIB_CRC32: u32 = 0x04d0_e435;
    let bytes = (0usize..1_048_576)
        .map(|index| u8::try_from(index % 256).expect("bounded pattern"))
        .collect::<Vec<_>>();
    let entry = [member("manifest.json", &bytes)];
    let bound = ProfileStoredZipWriteLimits {
        maximum_archive_bytes: 1_049_000,
        maximum_entries: 1,
        maximum_entry_bytes: 1_048_576,
        maximum_total_bytes: 1_048_576,
    };
    let planned = planned_stored_profile_zip_bytes(&entry, bound)
        .expect("large stored payload fits finite ZIP32 budget");
    let written = write_stored_profile_zip(&entry, bound)
        .expect("exact preflighted ZIP output");
    assert_eq!(written.len(), planned);
    assert_eq!(planned, bytes.len() + 124);
    let local_crc = u32::from_le_bytes(
        written
            .get(14..18)
            .expect("complete local CRC header")
            .try_into()
            .expect("exact 4-byte CRC field"),
    );
    let eocd = written.len() - 22;
    let central_start = u32::from_le_bytes(
        written
            .get(eocd + 16..eocd + 20)
            .expect("complete end record central offset")
            .try_into()
            .expect("exact central offset field"),
    ) as usize;
    let central_crc = u32::from_le_bytes(
        written
            .get(central_start + 16..central_start + 20)
            .expect("complete central CRC header")
            .try_into()
            .expect("exact 4-byte CRC field"),
    );
    assert_eq!(local_crc, PYTHON_ZLIB_CRC32);
    assert_eq!(central_crc, PYTHON_ZLIB_CRC32);
    let parsed = read_stored_profile_zip(
        &written,
        ProfileZipReadLimits {
            maximum_archive_bytes: written.len(),
            maximum_entries: 1,
            maximum_entry_bytes: bytes.len() as u64,
            maximum_total_bytes: bytes.len() as u64,
        },
    )
    .expect("strict parser accepts both matching CRC fields");
    assert_eq!(parsed.entries()[0].bytes(), bytes);
}

#[test]
fn largest_ordinary_zip32_entry_count_round_trips_without_zip64() {
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
        .expect("maximal ordinary count is still representable");
    let bytes = write_stored_profile_zip(&members, limits)
        .expect("ZIP32 allows 65534 entries without ZIP64 sentinel");
    assert_eq!(bytes.len(), planned);
    let mut caller_owned = vec![0x91u8; planned + 1];
    assert_eq!(
        write_stored_profile_zip_into(
            &members,
            limits,
            &mut caller_owned[..planned - 1],
        ),
        Err(ProfileStoredZipWriteError::BufferTooSmall { required: planned })
    );
    assert!(caller_owned.iter().all(|byte| *byte == 0x91));
    assert_eq!(
        write_stored_profile_zip_into(&members, limits, &mut caller_owned,),
        Ok(planned)
    );
    assert_eq!(&caller_owned[..planned], bytes.as_slice());
    assert_eq!(caller_owned[planned], 0x91);
    let eocd = bytes.len() - 22;
    for offset in [8usize, 10] {
        assert_eq!(
            &bytes[eocd + offset..eocd + offset + 2],
            &(u16::MAX - 1).to_le_bytes()
        );
    }
    let read_limits = ProfileZipReadLimits {
        maximum_archive_bytes: planned,
        maximum_entries: maximum_ordinary_entries,
        maximum_entry_bytes: 2,
        maximum_total_bytes: 2,
    };
    let parsed = read_stored_profile_zip(&bytes, read_limits)
        .expect("strict index accepts maximal ordinary ZIP count");
    assert!(parsed.matches_fixed_writer_metadata());
    assert!(parsed.matches_stored_writer_framing());
    assert_eq!(parsed.entries().len(), maximum_ordinary_entries);
    let mut slots = vec!["unchanged"; maximum_ordinary_entries + 1];
    assert_eq!(
        parsed.copy_observed_paths(&mut slots[..maximum_ordinary_entries - 1]),
        Err(ProfileZipPathCopyError::BufferTooSmall {
            required: maximum_ordinary_entries,
        }),
        "largest supported member count refuses short storage atomically",
    );
    assert!(slots.iter().all(|name| *name == "unchanged"));
    assert_eq!(
        parsed.copy_observed_paths(&mut slots),
        Ok(maximum_ordinary_entries)
    );
    assert_eq!(slots[0], "assets/00000.bin");
    assert_eq!(slots[maximum_ordinary_entries - 1], "manifest.json");
    assert_eq!(slots[maximum_ordinary_entries], "unchanged");
    assert!(std::ptr::eq(
        slots[0].as_ptr(),
        parsed.entries()[0].name().as_ptr()
    ));
    assert!(std::ptr::eq(
        slots[maximum_ordinary_entries - 1].as_ptr(),
        parsed.entries()[maximum_ordinary_entries - 1]
            .name()
            .as_ptr()
    ));
    assert_eq!(parsed.entries()[0].name(), "assets/00000.bin");
    assert_eq!(
        parsed.entries()[maximum_ordinary_entries - 2].name(),
        "assets/65532.bin"
    );
    assert_eq!(
        parsed.entries()[maximum_ordinary_entries - 1].name(),
        "manifest.json"
    );
    assert_eq!(parsed.entries()[0].bytes(), b"");
    assert_eq!(
        parsed.entries()[maximum_ordinary_entries - 1].bytes(),
        b"{}"
    );
    assert_eq!(parsed.source_sha256(), sha256_profile_bytes(&bytes));
    // Python's independent struct/zlib implementation produced these exact
    // ZIP32 bytes; its standard zipfile reader opened all 65,534 entries.
    assert_eq!(planned, 7_077_690);
    let digest = parsed.source_sha256().bytes();
    let golden = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    assert_eq!(
        golden,
        "c564b867a623511169ae843eac4acf3f08322782fdf7b6b6bb7351b90a52a793"
    );
    assert_eq!(
        read_stored_profile_zip(
            &bytes,
            ProfileZipReadLimits {
                maximum_entries: maximum_ordinary_entries - 1,
                ..read_limits
            }
        ),
        Err(ProfileZipReadError::ResourceLimit),
        "too many members must reject before index reservation",
    );
    assert_eq!(
        read_stored_profile_zip(
            &bytes,
            ProfileZipReadLimits {
                maximum_archive_bytes: planned - 1,
                ..read_limits
            }
        ),
        Err(ProfileZipReadError::ArchiveTooLarge),
        "one byte less than exact archive budget must reject",
    );
    assert_eq!(
        read_stored_profile_zip(
            &bytes,
            ProfileZipReadLimits {
                maximum_total_bytes: 1,
                ..read_limits
            }
        ),
        Err(ProfileZipReadError::ResourceLimit),
        "many empty entries cannot bypass nonempty root payload limit",
    );
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
        write_stored_profile_zip(
            &members,
            ProfileStoredZipWriteLimits {
                maximum_entries: maximum_ordinary_entries - 1,
                ..limits
            }
        ),
        Err(ProfileStoredZipWriteError::ResourceLimit),
    );
}

#[test]
fn legacy_stored_headers_round_trip_to_writer_framing_without_payload_edits() {
    let members = [
        member("assets/source.webp", b"\x00\x81\xff"),
        member("manifest.json", br#"{"kept":true}"#),
    ];
    let canonical = write_stored_profile_zip(&members, limits())
        .expect("fixed-metadata ZIP output");
    let central = u32::from_le_bytes(
        canonical[canonical.len() - 6..canonical.len() - 2]
            .try_into()
            .expect("EOCD central offset"),
    ) as usize;
    let mut legacy = canonical.clone();
    // Both fields describe the same first member, so the strict reader may
    // admit this legacy DOS timestamp without treating it as writer output.
    legacy[10..12].copy_from_slice(&[0x15, 0x00]);
    legacy[central + 12..central + 14].copy_from_slice(&[0x15, 0x00]);
    let checked = parse_bytes(&legacy);
    assert!(!checked.matches_fixed_writer_metadata());
    assert!(!checked.matches_stored_writer_framing());
    assert_eq!(checked.entries().len(), members.len());
    let recovered = checked
        .entries()
        .iter()
        .map(|entry| member(entry.name(), entry.bytes()))
        .collect::<Vec<_>>();
    let canonical_reframe = write_stored_profile_zip(&recovered, limits())
        .expect("same caller-selected bytes yield fixed ZIP metadata");
    assert_eq!(canonical_reframe, canonical);
    assert_ne!(canonical_reframe, legacy);
    assert_eq!(checked.source_sha256(), sha256_profile_bytes(&legacy));
    assert_ne!(checked.source_sha256(), sha256_profile_bytes(&canonical));
    assert_eq!(checked.entries()[1].bytes(), members[1].bytes);
    assert!(parse_bytes(&canonical_reframe).matches_stored_writer_framing());
}

#[test]
fn every_accepted_legacy_header_variant_reframes_to_exact_writer_bytes() {
    let selected = [
        member("assets/mark.webp", b"\0\x81\xff"),
        member("manifest.json", b"{}"),
        member("sections/ink.json", br#"[1,2,3]"#),
    ];
    let canonical = write_stored_profile_zip(&selected, limits())
        .expect("three-member exact stored ZIP output");
    let canonical_archive = parse_bytes(&canonical);
    assert!(canonical_archive.matches_stored_writer_framing());
    let central = u32::from_le_bytes(
        canonical[canonical.len() - 6..canonical.len() - 2]
            .try_into()
            .expect("EOCD central offset"),
    ) as usize;
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
                changed[local_offset + field..local_offset + field + 2]
                    .copy_from_slice(&value);
            }
            changed[central_offset + central_field
                ..central_offset + central_field + 2]
                .copy_from_slice(&value);
            let archive = parse_bytes(&changed);
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
                .expect("admitted original payloads can reframe");
            assert_eq!(output, canonical, "case={case} member={index}");
            assert!(parse_bytes(&output).matches_stored_writer_framing());
            checked += 1;
        }
        local_offset += 30 + entry.name.len() + entry.bytes.len();
        central_offset += 46 + entry.name.len();
    }
    assert_eq!(checked, 15);
    assert_eq!(local_offset, central);
}

#[test]
fn generated_stored_writer_framing_equivalence_is_byte_exact() {
    const PATHS: [&str; 7] = [
        "assets/000.bin",
        "assets/ñ.webp",
        "assets/🖌️.webp",
        "manifest.json",
        "sections/000.json",
        "sections/ß.json",
        "sections/😀.json",
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
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            if index != 3 && seed & 3 == 0 {
                continue;
            }
            let size = if index == 3 {
                // The byte writer never parses root JSON: some roots are
                // intentionally invalid while their ZIP framing stays exact.
                [0usize, 2, 7, 31][case % 4]
            } else {
                ((seed >> 19) % 161) as usize
            };
            let mut payload = Vec::with_capacity(size);
            for _ in 0..size {
                seed = seed
                    .wrapping_mul(2862933555777941757)
                    .wrapping_add(3037000493);
                payload.push((seed >> 48) as u8);
            }
            owned.push((path, payload));
        }
        for (path, payload) in &owned {
            selected.push(member(path, payload));
        }
        assert!(selected.iter().any(|item| item.name == "manifest.json"));
        let planned = planned_stored_profile_zip_bytes(&selected, generous)
            .expect("finite inputs fit deterministic ZIP32 plan");
        let canonical = write_stored_profile_zip(&selected, generous)
            .expect("all generated members have safe sorted paths");
        assert_eq!(canonical.len(), planned, "case={case}");
        let mut caller_storage = vec![0x6du8; planned + 3];
        assert_eq!(
            write_stored_profile_zip_into(
                &selected,
                generous,
                &mut caller_storage,
            ),
            Ok(planned),
            "case={case}"
        );
        assert_eq!(&caller_storage[..planned], canonical.as_slice());
        assert_eq!(&caller_storage[planned..], &[0x6d; 3]);
        let archive = read_stored_profile_zip(
            &canonical,
            ProfileZipReadLimits {
                maximum_archive_bytes: planned,
                maximum_entries: selected.len(),
                maximum_entry_bytes: generous.maximum_entry_bytes,
                maximum_total_bytes: generous.maximum_total_bytes,
            },
        )
        .expect("independent strict reader must accept writer output");
        assert!(archive.matches_stored_writer_framing(), "case={case}");
        assert!(archive.matches_fixed_writer_metadata(), "case={case}");
        assert_eq!(archive.source_bytes(), canonical, "case={case}");
        let recovered = archive
            .entries()
            .iter()
            .map(|entry| member(entry.name(), entry.bytes()))
            .collect::<Vec<_>>();
        assert_eq!(recovered, selected, "case={case}");
        let encoded_again = write_stored_profile_zip(&recovered, generous)
            .expect("proven checked inputs re-encode without byte drift");
        assert_eq!(encoded_again, canonical, "case={case}");
        assert_eq!(archive.source_sha256(), sha256_profile_bytes(&canonical));
        // One matching DOS date edit in the first local and central headers
        // retains CRC-verified payloads but can never claim writer framing.
        let directory = u32::from_le_bytes(
            canonical[canonical.len() - 6..canonical.len() - 2]
                .try_into()
                .expect("central directory byte offset"),
        ) as usize;
        let mut legacy = canonical.clone();
        legacy[12..14].copy_from_slice(&[3, 0]);
        legacy[directory + 14..directory + 16].copy_from_slice(&[3, 0]);
        let observed = read_stored_profile_zip(
            &legacy,
            ProfileZipReadLimits {
                maximum_archive_bytes: legacy.len(),
                maximum_entries: selected.len(),
                maximum_entry_bytes: generous.maximum_entry_bytes,
                maximum_total_bytes: generous.maximum_total_bytes,
            },
        )
        .expect("metadata changed without affecting stored entry proof");
        assert!(!observed.matches_stored_writer_framing(), "case={case}");
        assert_eq!(observed.entries(), archive.entries(), "case={case}");
        assert_ne!(observed.source_sha256(), archive.source_sha256());
        cases += 1;
    }
    assert_eq!(cases, 1_024);
}

#[test]
fn caller_owned_zip_output_is_atomic_for_too_small_storage() {
    let members = [
        member("assets/ñ.webp", b"\x00\xff\x80"),
        member("manifest.json", br#"{"spacing":true}"#),
        member("sections/😀.json", b"[1,2,3]"),
    ];
    let expected = write_stored_profile_zip(&members, limits())
        .expect("owned reference emits one exact ZIP frame");
    let required = planned_stored_profile_zip_bytes(&members, limits())
        .expect("exact ZIP32 byte planning");
    assert_eq!(expected.len(), required);
    let mut storage = vec![0xabu8; required + 17];
    for available in [0usize, 1, 22, required - 1] {
        assert_eq!(
            write_stored_profile_zip_into(
                &members,
                limits(),
                &mut storage[..available],
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
            &mut storage[..required],
        ),
        Ok(required)
    );
    assert_eq!(&storage[..required], expected.as_slice());
    assert!(storage[required..].iter().all(|byte| *byte == 0xab));
    storage.fill(0xcd);
    assert_eq!(
        write_stored_profile_zip_into(&members, limits(), &mut storage,),
        Ok(required)
    );
    assert_eq!(&storage[..required], expected.as_slice());
    assert_eq!(&storage[required..], &[0xcd; 17]);
    let archive = parse_bytes(&storage[..required]);
    assert!(archive.matches_stored_writer_framing());
    assert_eq!(archive.source_sha256(), sha256_profile_bytes(&expected));
}

#[test]
fn caller_owned_zip_output_checks_input_before_destination_capacity() {
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
}

#[test]
fn input_member_debug_never_prints_private_archive_paths_or_payload() {
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
}
