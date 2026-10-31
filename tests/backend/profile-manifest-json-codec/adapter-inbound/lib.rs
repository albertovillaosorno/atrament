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
//   - Exact wire-field decoding and resource-budget regression evidence.
// - Must-Not:
//   - Open files, accept a profile, or convert a manifest path to authority.
// - Allows:
//   - Inputs: Independently generated JSON and stored ZIP bytes.
//   - Outputs: Typed schema failures and complete decoded manifest values.
//   - Side effects: Test-only process memory.
// - Split-When:
//   - Typed Import acceptance gets its own integration test.
// - Merge-When:
//   - A complete Import adapter replaces the read-only schema test boundary.
// - Summary:
//   - Proves strict schema decoding from source-sealed ZIP manifest bytes.
// - Description:
//   - Correct metadata must still pass separate inventory and hash admission.
// - Usage:
//   - Run `cargo test -p atrament_profile_manifest_json_codec`.
// - Defaults:
//   - Unknown keys, unsupported values and over-budget arrays reject closed.
//

//! Read-only typed profile manifest codec integration and adversarial fixtures.

use std::path::PathBuf;

use atrament_explicit_file_path_intent::{
    ExplicitFilePathSource, ExplicitFilePurpose, ExplicitOverwriteIntent,
    classify_explicit_file_path_intent,
};
use atrament_handwriting_profile_container::validate_profile_manifest;
use atrament_profile_import_preflight::{
    ProfileImportBudget, preflight_explicit_profile_import_sorted,
};
use atrament_profile_entry_sha256::sha256_profile_bytes;
use atrament_profile_json_syntax::{
    ProfileJsonSyntaxError, ProfileJsonSyntaxLimits,
    validate_profile_json_object,
};
use atrament_profile_manifest_json_codec::{
    ProfileManifestJsonError as Error, ProfileManifestJsonLimits,
    decode_profile_manifest_json,
    decode_attached_profile_manifest_json,
};
use atrament_profile_stored_zip_writer::{
    ProfileStoredZipMember, ProfileStoredZipWriteLimits,
    write_stored_profile_zip,
};
use atrament_profile_zip_reader::{
    ProfileZipReadLimits, read_stored_profile_zip,
    verify_admitted_profile_zip_payloads,
};

const ZERO_SHA: &str = concat!(
    "00000000000000000000000000000000",
    "00000000000000000000000000000000",
);

const fn syntax_limits() -> ProfileJsonSyntaxLimits {
    ProfileJsonSyntaxLimits {
        maximum_bytes: 8_192,
        maximum_nesting: 12,
        maximum_object_members: 20,
        maximum_values: 128,
    }
}

const fn codec_limits() -> ProfileManifestJsonLimits {
    ProfileManifestJsonLimits {
        maximum_entries: 4,
        maximum_features: 4,
        maximum_string_bytes: 128,
    }
}

fn base_json() -> String {
    format!(
        concat!(
            r#"{{"container_version":"atrament.profile/1","#,
            r#""profile_identity":"ñote-1","required_features":["strokes"],"#,
            r#""optional_features":[],"#,
            r#""entries":[{{"path":"sections/strokes.json","media_type":""#,
            r#"application/json","byte_length":2,"sha256":"{}"}}]}}"#,
        ),
        ZERO_SHA
    )
}

fn decode(
    input: &str,
    limits: ProfileManifestJsonLimits,
) -> Result<atrament_handwriting_profile_container::ProfileManifest, Error> {
    let syntax = validate_profile_json_object(input.as_bytes(), syntax_limits())
        .expect("fixtures must have valid JSON grammar");
    decode_profile_manifest_json(syntax, limits)
}

#[test]
fn full_manifest_decodes_every_field_without_losing_optional_features() {
    let input = base_json().replace(
        "\"optional_features\":[]",
        "\"optional_features\":[\"future.feature\",\"mañana\"]",
    );
    let manifest = decode(&input, codec_limits()).expect("all fields decoded");
    assert_eq!(manifest.container_version, "atrament.profile/1");
    assert_eq!(manifest.profile_identity, "ñote-1");
    assert_eq!(manifest.required_features, ["strokes"]);
    assert_eq!(manifest.optional_features, ["future.feature", "mañana"]);
    assert_eq!(manifest.entries.len(), 1);
    assert_eq!(manifest.entries[0].path, "sections/strokes.json");
    assert_eq!(manifest.entries[0].media_type, "application/json");
    assert_eq!(manifest.entries[0].byte_length, 2);
    assert_eq!(manifest.entries[0].digest.bytes(), [0u8; 32]);
    assert_eq!(validate_profile_manifest(&manifest, &["strokes"]), Ok(()));
    assert!(validate_profile_manifest(&manifest, &[]).is_err());
}

#[test]
fn decoded_utf8_respects_exact_byte_budget_with_large_caller_limits() {
    let source = concat!(
        r#"{"container_version":"v1", "#,
        r#""profile_identity":"\uD83D\uDD8B\u00f1", "#,
        r#""required_features":["\u00f1"], "#,
        r#""optional_features":["é"],"entries":[]}"#,
    );
    let mut limits = codec_limits();
    limits.maximum_entries = usize::MAX;
    limits.maximum_features = usize::MAX;
    limits.maximum_string_bytes = 6;
    let manifest = decode(source, limits).expect("decoded UTF-8 fits exactly");
    assert_eq!(manifest.profile_identity, "🖋ñ");
    assert_eq!(manifest.profile_identity.len(), 6);
    assert_eq!(manifest.required_features, ["ñ"]);
    assert_eq!(manifest.optional_features, ["é"]);

    limits.maximum_string_bytes = 5;
    assert_eq!(
        decode(source, limits),
        Err(Error::StringLimitExceeded("profile_identity")),
    );
}

#[test]
fn bounded_metadata_vectors_admit_all_entries_at_exact_limit() {
    let member = format!(
        concat!(
            r#"{{"path":"sections/a.json","media_type":"application/json", "#,
            r#""byte_length":2,"sha256":"{}"}}"#,
        ),
        ZERO_SHA,
    );
    let entries = vec![member; 32].join(",");
    let features = vec![r#""feature""#; 32].join(",");
    let source = format!(
        concat!(
            r#"{{"container_version":"v1","profile_identity":"writer", "#,
            r#""required_features":[{features}],"optional_features":[],"#,
            r#""entries":[{entries}]}}"#,
        ),
        features = features,
        entries = entries,
    );
    let syntax = validate_profile_json_object(
        source.as_bytes(),
        ProfileJsonSyntaxLimits {
            maximum_bytes: source.len(),
            maximum_nesting: 12,
            maximum_object_members: 20,
            maximum_values: 512,
        },
    ).expect("large manifest syntax fits its explicit limits");
    let limits = ProfileManifestJsonLimits {
        maximum_entries: 32,
        maximum_features: 32,
        maximum_string_bytes: 64,
    };
    let admitted = decode_profile_manifest_json(syntax, limits)
        .expect("bounded entries and features allocate within limits");
    assert_eq!(admitted.entries.len(), 32);
    assert_eq!(admitted.required_features.len(), 32);
    assert!(admitted.entries.iter().all(|entry| entry.byte_length == 2));
    assert_eq!(
        decode_profile_manifest_json(
            syntax,
            ProfileManifestJsonLimits {
                maximum_entries: 31,
                ..limits
            },
        ),
        Err(Error::ArrayLimitExceeded("entries")),
    );
    assert_eq!(
        decode_profile_manifest_json(
            syntax,
            ProfileManifestJsonLimits {
                maximum_features: 31,
                ..limits
            },
        ),
        Err(Error::ArrayLimitExceeded("required_features")),
    );
}

#[test]
fn keys_and_strings_decode_json_escapes_without_coercion() {
    let input = base_json()
        .replace(
            "\"profile_identity\":\"ñote-1\"",
            r#""profile_\u0069dentity":"\u00f1ote-1""#,
        )
        .replace("\"sha256\":", "\"sha\\u003256\":");
    // The escaped key above actually decodes to `sha256`.
    let manifest = decode(&input, codec_limits()).expect("decoded exact keys");
    assert_eq!(manifest.profile_identity, "ñote-1");
    assert_eq!(manifest.entries[0].digest.bytes(), [0; 32]);
    let ambiguous = input.replace(
        r#""profile_\u0069dentity":"\u00f1ote-1""#,
        r#""profile_identity":"new","profile_\u0069dentity":"old""#,
    );
    assert_eq!(
        validate_profile_json_object(ambiguous.as_bytes(), syntax_limits()),
        Err(ProfileJsonSyntaxError::DuplicateObjectKey),
    );
}

#[test]
fn missing_unknown_and_wrong_kind_fields_fail_closed() {
    let valid = base_json();
    let invalid_entries_kind = format!(
        "{}\"entries\":{{}}}}",
        valid.split("\"entries\":").next().unwrap()
    );
    let cases = [
        (
            valid.replace("\"optional_features\":[],", ""),
            Error::MissingField("optional_features"),
        ),
        (
            valid.replace(
                "\"profile_identity\":",
                "\"unknown\":0,\"profile_identity\":",
            ),
            Error::UnknownField("manifest"),
        ),
        (
            valid.replace("\"path\":", "\"extra\":0,\"path\":"),
            Error::UnknownField("entries[]"),
        ),
        (
            valid.replace("\"entries\":[{", "\"entries\":[null,{"),
            Error::WrongType("entries[]"),
        ),
        (invalid_entries_kind, Error::WrongType("entries")),
        (
            valid.replace(
                "\"required_features\":[\"strokes\"]",
                "\"required_features\":[123]",
            ),
            Error::WrongType("required_features"),
        ),
        (
            valid.replace(
                "\"profile_identity\":\"ñote-1\"",
                "\"profile_identity\":false",
            ),
            Error::WrongType("profile_identity"),
        ),
        (
            valid.replace("\"path\":\"sections/strokes.json\",", ""),
            Error::MissingField("path"),
        ),
    ];
    for (mutated, expected) in cases {
        assert_eq!(
            decode(&mutated, codec_limits()),
            Err(expected),
            "bad schema was accepted: {mutated}"
        );
    }
}

#[test]
fn string_and_array_limits_refuse_excess_without_partial_admission() {
    let valid = base_json();
    let mut limits = codec_limits();
    limits.maximum_entries = 0;
    assert_eq!(
        decode(&valid, limits),
        Err(Error::ArrayLimitExceeded("entries"))
    );
    limits = codec_limits();
    limits.maximum_features = 0;
    assert_eq!(
        decode(&valid, limits),
        Err(Error::ArrayLimitExceeded("required_features"))
    );
    limits = codec_limits();
    limits.maximum_string_bytes = "atrament.profile/1".len();
    assert!(decode(&valid, limits).is_err());
    limits.maximum_string_bytes = 128;
    assert!(decode(&valid, limits).is_ok());

    let two = valid.replace("\"strokes\"]", "\"strokes\",\"next\"]");
    limits.maximum_features = 1;
    assert_eq!(
        decode(&two, limits),
        Err(Error::ArrayLimitExceeded("required_features"))
    );
    let entry = valid.split("\"entries\":[")
        .nth(1).expect("fixture entries field")
        .strip_suffix("]}").expect("fixture entries array");
    let second = valid.replace(entry, &format!("{entry},{entry}"));
    limits.maximum_entries = 1;
    limits.maximum_features = 4;
    assert_eq!(
        decode(&second, limits),
        Err(Error::ArrayLimitExceeded("entries"))
    );
    limits.maximum_entries = 2;
    assert_eq!(decode(&second, limits).unwrap().entries.len(), 2);
}

#[test]
fn unsigned_lengths_and_exact_lowercase_hashes_reject_ambiguous_values() {
    let valid = base_json();
    for bad in ["-1", "2.0", "2e0", "18446744073709551616", "true", "\"2\""] {
        let mutated = valid.replace(
            "\"byte_length\":2", &format!("\"byte_length\":{bad}"),
        );
        assert_eq!(
            decode(&mutated, codec_limits()),
            Err(Error::InvalidUnsignedInteger("entries[].byte_length"))
        );
    }
    for bad in ["a", "A", "g", "0"]
        .map(str::to_string)
        .into_iter()
        .chain(["0".repeat(63), "0".repeat(65)])
    {
        let mutated = valid.replace(ZERO_SHA, &bad);
        assert_eq!(decode(&mutated, codec_limits()), Err(Error::InvalidSha256));
    }
    let mutated = valid.replace(
        &format!("\"sha256\":\"{ZERO_SHA}\""), "\"sha256\":null",
    );
    assert_eq!(
        decode(&mutated, codec_limits()),
        Err(Error::WrongType("entries[].sha256"))
    );
    let decimal = valid.replace(
        "\"byte_length\":2", "\"byte_length\":18446744073709551615",
    );
    assert_eq!(
        decode(&decimal, codec_limits()).unwrap().entries[0].byte_length,
        u64::MAX
    );
}

#[test]
fn real_zip_root_decodes_to_typed_manifest_with_independent_payload_digest() {
    let payload = b"hi";
    let sha = sha256_profile_bytes(payload);
    let hex: String = sha
        .bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let root = base_json().replace(ZERO_SHA, &hex);
    let members = [
        ProfileStoredZipMember {
            name: "manifest.json",
            bytes: root.as_bytes(),
        },
        ProfileStoredZipMember {
            name: "sections/strokes.json",
            bytes: payload,
        },
    ];
    let zip = write_stored_profile_zip(
        &members,
        ProfileStoredZipWriteLimits {
            maximum_archive_bytes: 16_384,
            maximum_entries: 2,
            maximum_entry_bytes: 8_192,
            maximum_total_bytes: 8_192,
        },
    )
    .expect("canonical stored framing");
    let archive = read_stored_profile_zip(
        &zip,
        ProfileZipReadLimits {
            maximum_archive_bytes: 16_384,
            maximum_entries: 2,
            maximum_entry_bytes: 8_192,
            maximum_total_bytes: 8_192,
        },
    )
    .expect("checked archive framing");
    let attached = archive
        .admitted_root_json(syntax_limits())
        .expect("source-backed JSON root");
    assert_eq!(attached.archive().source_sha256(), archive.source_sha256());
    let original_sha = attached.source_sha256();
    let attached_decoded = decode_attached_profile_manifest_json(
        &attached, codec_limits(),
    ).expect("type remains associated with source ZIP");
    let debug = format!("{attached_decoded:?}");
    assert!(debug.contains("DecodedProfileZipManifest"));
    for private in ["ñote-1", "sections/strokes.json", "strokes"] {
        assert!(!debug.contains(private));
    }
    assert!(!debug.contains(&format!("{zip:?}")));
    assert_eq!(attached_decoded.root_sha256(), original_sha);
    assert_eq!(attached_decoded.archive().source_sha256(),
        archive.source_sha256());
    assert!(std::ptr::eq(attached_decoded.archive(), &archive));
    let manifest = attached_decoded.manifest();
    assert_eq!(manifest.entries[0].digest, sha);
    assert_eq!(manifest.entries[0].byte_length, payload.len() as u64);
    assert_eq!(validate_profile_manifest(manifest, &["strokes"]), Ok(()));
    let selected = classify_explicit_file_path_intent(
        ExplicitFilePurpose::ImportProfile,
        ExplicitFilePathSource::CallerSelection,
        PathBuf::from("selected/profile.atrament"),
        ExplicitOverwriteIntent::PreserveExisting,
    ).expect("synthetic caller selection for read-only preflight");
    let mut paths = [""; 2];
    let count = archive.copy_observed_paths(&mut paths)
        .expect("names sourced from this ZIP");
    let preflight = preflight_explicit_profile_import_sorted(
        &selected, manifest, &["strokes"], &paths[..count],
        archive.encoding_evidence(), ProfileImportBudget {
            maximum_declared_bytes: 2,
            maximum_entries: 1,
            maximum_entry_bytes: 2,
        },
    ).expect("typed root agrees with actual archive inventory");
    let verified = verify_admitted_profile_zip_payloads(
        &archive, preflight.inventory(),
    ).expect("declared digest agrees with exact ZIP member bytes");
    assert_eq!(verified.verified_entry_bytes("sections/strokes.json"),
        Some(payload.as_slice()));

    // The stored ZIP writer computes a new CRC for substituted bytes.
    // Root JSON is unchanged and remains schema-valid, but its SHA-256
    // declaration must reject a same-length payload alteration.
    let tampered_members = [
        ProfileStoredZipMember {
            name: "manifest.json",
            bytes: root.as_bytes(),
        },
        ProfileStoredZipMember { name: "sections/strokes.json", bytes: b"ho" },
    ];
    let changed_zip = write_stored_profile_zip(&tampered_members,
        ProfileStoredZipWriteLimits {
            maximum_archive_bytes: 16_384,
            maximum_entries: 2,
            maximum_entry_bytes: 8_192,
            maximum_total_bytes: 8_192,
        },
    ).expect("CRC-correct altered ZIP");
    let changed_archive = read_stored_profile_zip(&changed_zip,
        ProfileZipReadLimits {
            maximum_archive_bytes: 16_384,
            maximum_entries: 2,
            maximum_entry_bytes: 8_192,
            maximum_total_bytes: 8_192,
        },
    ).expect("same framing still passes");
    let changed_root = changed_archive.admitted_root_json(syntax_limits())
        .expect("the same root stays schema-admissible");
    let changed_decoded = decode_attached_profile_manifest_json(
        &changed_root, codec_limits(),
    ).expect("the other ZIP also has a typed manifest");
    assert!(std::ptr::eq(changed_decoded.archive(), &changed_archive));
    assert_eq!(changed_decoded.root_sha256(), original_sha);
    assert_ne!(changed_decoded.archive().source_sha256(),
        attached_decoded.archive().source_sha256());
    assert!(verify_admitted_profile_zip_payloads(
        &changed_archive, preflight.inventory(),
    ).is_err());
    // No selected filesystem source has been opened or session mutated.
}

#[test]
fn deterministic_manifest_mutations_never_escape_sealed_schema_gates() {
    // These inputs model untrusted root bytes before file-source admission.
    // Syntax and typed admission must agree on every repeated observation.
    let base = base_json().into_bytes();
    let mut state = 0x4c52_4f4f_u64;
    let mut syntax_rejected = 0usize;
    let mut schema_rejected = 0usize;
    let mut admitted = 0usize;
    let mut observed_operations = [0usize; 8];
    for case in 0usize..4_096 {
        state = state.wrapping_mul(6_364_136_223_846_793_005u64)
            .wrapping_add(1_442_695_040_888_963_407u64);
        let offset = usize::try_from(state >> 32u32)
            .expect("high LCG half fits usize") % base.len();
        let mutation = case % observed_operations.len();
        if let Some(count) = observed_operations.get_mut(mutation) {
            *count = count.saturating_add(1);
        }
        let mut raw = base.clone();
        match mutation {
            0 => raw[offset] = [b'"', b'\\', b'[', b'{', 0xff]
                .get(case % 5).copied().expect("bounded selection"),
            1 => { let _removed = raw.remove(offset); }
            2 => raw.insert(offset, b'0'),
            3 => raw.insert(offset, b' '),
            4 => {
                let next = raw.get(offset).copied().expect("bounded offset");
                raw.insert(offset, next);
            }
            5 => raw = base_json().replace(
                "\"byte_length\":2", &format!(
                    "\"byte_length\":{}", case % 11,
                ),
            ).into_bytes(),
            6 => raw = base_json().replace(
                ZERO_SHA, &format!("{:064x}", case),
            ).into_bytes(),
            7 => raw = base_json().replace(
                "\"optional_features\":[]",
                "\"optional_features\":[\"unknown\\u0000future\"]",
            ).into_bytes(),
            _ => unreachable!("the operation index is bounded"),
        }
        let first = validate_profile_json_object(&raw, syntax_limits());
        let second = validate_profile_json_object(&raw, syntax_limits());
        assert_eq!(first, second, "case={case}, mutation={mutation}");
        let Ok(root) = first else {
            syntax_rejected = syntax_rejected.saturating_add(1);
            continue;
        };
        let first = decode_profile_manifest_json(root, codec_limits());
        let second = decode_profile_manifest_json(root, codec_limits());
        assert_eq!(first, second, "case={case}, mutation={mutation}");
        match first {
            Ok(manifest) => {
                admitted = admitted.saturating_add(1);
                assert!(
                    manifest.entries.len() <= codec_limits().maximum_entries
                );
                assert!(
                    manifest.required_features.len()
                        <= codec_limits().maximum_features
                );
                assert!(
                    manifest.optional_features.len()
                        <= codec_limits().maximum_features
                );
                // No schema success grants selected-file or section admission.
            }
            Err(_schema) => {
                schema_rejected = schema_rejected.saturating_add(1);
            }
        }
    }
    assert!(observed_operations.iter().all(|count| *count >= 512));
    assert!(syntax_rejected > 500);
    assert!(schema_rejected > 100);
    assert!(admitted > 300);
}
