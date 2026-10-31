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
//   - Golden canonical JSON and cross-adapter profile manifest round trips.
// - Must-Not:
//   - Open selected files, authorize Export, or accept session profiles.
// - Allows:
//   - Inputs: Typed manifests and in-memory real stored ZIP bytes.
//   - Outputs: Deterministic byte comparisons and typed refusal evidence.
//   - Side effects: Test-owned memory only.
// - Split-When:
//   - Concrete section serializers require independent semantic fixtures.
// - Merge-When:
//   - Complete portable-profile Import/Export tests replace these adapters.
// - Summary:
//   - Proves stable bytes, escaping and decode/import-verification agreement.
// - Description:
//   - No output receives implicit filesystem intent from manifest data.
// - Usage:
//   - Run `cargo test -p atrament_profile_manifest_json_writer`.
// - Defaults:
//   - Reject over-budget or malformed values without partial output.
//

//! Canonical profile manifest encoding from admitted typed metadata.

use std::path::PathBuf;
use std::str::from_utf8;

use atrament_explicit_file_path_intent::{
    ExplicitFilePathSource, ExplicitFilePurpose, ExplicitOverwriteIntent,
    classify_explicit_file_path_intent,
};
use atrament_handwriting_profile_container::{
    PROFILE_CONTAINER_VERSION, ProfileManifest, ProfileManifestEntry,
    ProfileFeatureList, ProfileManifestError, Sha256Digest,
};
use atrament_profile_entry_sha256::sha256_profile_bytes;
use atrament_profile_import_preflight::ProfileImportBudget;
use atrament_profile_import_verification::verify_attached_profile_import;
use atrament_profile_json_syntax::{
    ProfileJsonSyntaxLimits, validate_profile_json_object,
};
use atrament_profile_manifest_json_codec::{
    ProfileManifestJsonLimits, decode_attached_profile_manifest_json,
    decode_profile_manifest_json,
};
use atrament_profile_manifest_json_writer::{
    ProfileManifestJsonWriteError as Error, ProfileManifestJsonWriteLimits,
    planned_canonical_profile_manifest_json_bytes,
    write_canonical_profile_manifest_json,
    write_canonical_profile_manifest_json_into,
};
use atrament_profile_stored_zip_writer::{
    ProfileStoredZipMember, ProfileStoredZipWriteLimits,
    write_stored_profile_zip,
};
use atrament_profile_zip_reader::{
    ProfileZipReadLimits, read_stored_profile_zip,
};

// Test-only guards preserve an explicit failed expectation on bad input.
macro_rules! require_ok {
    ($expression:expr) => {{
        let outcome = $expression;
        assert!(
            outcome.is_ok(),
            "expected valid manifest: {:?}",
            outcome.as_ref().err(),
        );
        let Ok(value) = outcome else { return; };
        value
    }};
}

macro_rules! require_some {
    ($expression:expr) => {{
        let selected = $expression;
        assert!(selected.is_some(), "expected fixture entry or byte range");
        let Some(value) = selected else { return; };
        value
    }};
}

const fn limits() -> ProfileManifestJsonWriteLimits {
    ProfileManifestJsonWriteLimits {
        maximum_bytes: 8192,
        maximum_entries: 8,
        maximum_features: 8,
        maximum_string_bytes: 256,
    }
}

fn fixture() -> ProfileManifest {
    ProfileManifest {
        container_version: String::from(PROFILE_CONTAINER_VERSION),
        entries: vec![],
        optional_features: vec!["ma\u{f1}ana".into(), "future".into()],
        profile_identity: "Writer \u{1f58b}".into(),
        required_features: vec!["strokes".into(), "layout".into()],
    }
}

fn entry(path: &str, payload: &[u8]) -> ProfileManifestEntry {
    ProfileManifestEntry {
        byte_length: u64::try_from(payload.len()).unwrap_or(u64::MAX),
        digest: sha256_profile_bytes(payload),
        media_type: "application/json".into(),
        path: path.into(),
    }
}

fn encode(manifest: &ProfileManifest) -> Vec<u8> {
    let result = write_canonical_profile_manifest_json(
        manifest, &["strokes", "layout"], limits(),
    );
    assert!(
        result.is_ok(),
        "canonical fixture must encode: {:?}", result.as_ref().err(),
    );
    result.unwrap_or_default()
}

fn decode(bytes: &[u8]) -> ProfileManifest {
    let parsed_root = validate_profile_json_object(
        bytes,
        ProfileJsonSyntaxLimits {
            maximum_bytes: 8192,
            maximum_nesting: 16,
            maximum_object_members: 32,
            maximum_values: 128,
        },
    );
    assert!(parsed_root.is_ok(), "writer output must pass JSON syntax");
    let Ok(root) = parsed_root else { return fixture(); };
    let decoded = decode_profile_manifest_json(
        root,
        ProfileManifestJsonLimits {
            maximum_entries: 8,
            maximum_features: 8,
            maximum_string_bytes: 256,
        },
    );
    assert!(decoded.is_ok(), "writer output must pass typed decoding");
    decoded.unwrap_or_else(|_| fixture())
}

#[test]
fn golden_root_only_json_has_fixed_key_and_feature_order() {
    let mut manifest = fixture();
    manifest.profile_identity = "writer-1".into();
    manifest.optional_features.clear();
    let actual = encode(&manifest);
    assert_eq!(
        from_utf8(&actual).unwrap_or(""),
        concat!(
            r#"{"container_version":"atrament.profile/1","entries":[],"#,
            r#""optional_features":[],"profile_identity":"writer-1","#,
            r#""required_features":["layout","strokes"]}"#,
        ),
    );
    assert_eq!(decode(&actual).required_features, ["layout", "strokes"],);
}

#[test]
fn input_order_cannot_change_canonical_byte_identity() {
    let mut first = fixture();
    first.entries = vec![
        entry("sections/z.json", b"last"),
        entry("assets/b.bin", b"first"),
    ];
    let mut second = first.clone();
    second.entries.reverse();
    second.optional_features.reverse();
    second.required_features.reverse();
    let bytes = encode(&first);
    assert_eq!(bytes, encode(&second));
    let decoded = decode(&bytes);
    assert_eq!(require_some!(decoded.entries.first()).path, "assets/b.bin");
    assert_eq!(require_some!(decoded.entries.get(1)).path, "sections/z.json");
    assert_eq!(decoded.optional_features, ["future", "ma\u{f1}ana"]);
    assert_eq!(decoded.required_features, ["layout", "strokes"]);
    assert_eq!(encode(&decoded), bytes);
}

#[test]
fn strings_escape_controls_but_retain_unicode_and_hex_digest_bytes() {
    let mut manifest = fixture();
    manifest.profile_identity = concat!(
        "\u{f1} \u{1f58b} \" \\ \0 \u{001f}",
        "\n\r\t\u{0008}\u{000c}",
    ).into();
    manifest.entries.push(ProfileManifestEntry {
        byte_length: u64::MAX,
        digest: Sha256Digest::from_bytes([0xab; 32]),
        media_type: "application/json".into(),
        path: "assets/\u{f1}.json".into(),
    });
    let bytes = encode(&manifest);
    let text = require_ok!(from_utf8(&bytes));
    assert!(text.contains(concat!(
        "\u{f1} \u{1f58b} ",
        r#"\" \\ \u0000 \u001f\n\r\t\b\f"#,
    )));
    assert!(text.contains("18446744073709551615"));
    assert!(text.contains(&"ab".repeat(32)));
    let decoded = decode(&bytes);
    assert_eq!(decoded.profile_identity, manifest.profile_identity);
    assert_eq!(require_some!(decoded.entries.first()).byte_length, u64::MAX);
    assert_eq!(
        require_some!(decoded.entries.first()).digest,
        require_some!(manifest.entries.first()).digest,
    );
    assert_eq!(encode(&decoded), bytes);
}

#[test]
fn every_decimal_width_preserves_plan_owned_and_buffer_bytes() {
    let mut manifest = fixture();
    manifest.entries.push(entry("assets/source.bin", b"ok"));
    let mut lengths = vec![0, u64::from(u32::MAX), u64::MAX - 1, u64::MAX];
    for exponent in 1..=19u32 {
        let boundary = 10u64.pow(exponent);
        lengths.extend([boundary - 1, boundary]);
    }
    for length in lengths {
        require_some!(manifest.entries.first_mut()).byte_length = length;
        let planned = require_ok!(planned_canonical_profile_manifest_json_bytes(
            &manifest, &["strokes", "layout"], limits(),
        ));
        let owned = encode(&manifest);
        assert_eq!(owned.len(), planned);
        let readable = require_ok!(from_utf8(&owned));
        assert!(
            readable.contains(&format!(r#""byte_length":{length}"#)),
            "canonical decimal was not exact: {length}",
        );
        let decoded = decode(&owned);
        assert_eq!(require_some!(decoded.entries.first()).byte_length, length);
        let mut destination = vec![0xa5; planned + 7];
        let written = require_ok!(write_canonical_profile_manifest_json_into(
            &manifest, &["strokes", "layout"], limits(),
            &mut destination,
        ));
        assert_eq!(written, planned);
        assert_eq!(destination.get(..written), Some(owned.as_slice()));
        assert_eq!(destination.get(written..), Some(&[0xa5; 7][..]));
        let mut tighter = limits();
        tighter.maximum_bytes = planned - 1;
        assert_eq!(
            planned_canonical_profile_manifest_json_bytes(
                &manifest, &["strokes", "layout"], tighter,
            ),
            Err(Error::ManifestTooLarge),
        );
    }
}

#[test]
fn output_budget_matches_exact_escape_aware_count() {
    let manifest = fixture();
    let actual = encode(&manifest);
    let mut allowed = limits();
    allowed.maximum_bytes = actual.len();
    assert_eq!(
        write_canonical_profile_manifest_json(
            &manifest, &["layout", "strokes"], allowed,
        ),
        Ok(actual),
    );
    allowed.maximum_bytes -= 1;
    assert_eq!(
        write_canonical_profile_manifest_json(
            &manifest, &["layout", "strokes"], allowed,
        ),
        Err(Error::ManifestTooLarge),
    );
    allowed.maximum_bytes = 0;
    assert_eq!(
        write_canonical_profile_manifest_json(
            &manifest, &["layout", "strokes"], allowed,
        ),
        Err(Error::ManifestTooLarge),
    );
}

#[test]
fn typed_admission_and_field_limits_refuse_before_serialization() {
    let mut manifest = fixture();
    manifest.entries.push(entry("sections/strokes.json", b"ok"));
    let mut bounds = limits();
    bounds.maximum_entries = 0;
    assert_eq!(
        write_canonical_profile_manifest_json(
            &manifest, &["strokes", "layout"], bounds,
        ),
        Err(Error::ArrayLimitExceeded("entries")),
    );
    bounds = limits();
    bounds.maximum_features = 1;
    assert_eq!(
        write_canonical_profile_manifest_json(
            &manifest, &["strokes", "layout"], bounds,
        ),
        Err(Error::ArrayLimitExceeded("required_features")),
    );
    bounds = limits();
    bounds.maximum_string_bytes = 5;
    assert_eq!(
        write_canonical_profile_manifest_json(
            &manifest, &["strokes", "layout"], bounds,
        ),
        Err(Error::StringLimitExceeded("container_version")),
    );
    bounds = limits();
    assert!(matches!(
        write_canonical_profile_manifest_json(&manifest, &[], bounds),
        Err(Error::Manifest(
            ProfileManifestError::UnsupportedRequiredFeature { .. }
        )),
    ));
    require_some!(manifest.entries.first_mut()).path = "assets/../bad".into();
    assert!(matches!(
        write_canonical_profile_manifest_json(
            &manifest, &["strokes", "layout"], bounds,
        ),
        Err(Error::Manifest(
            ProfileManifestError::InvalidEntryPath { .. }
        )),
    ));
    require_some!(manifest.entries.first_mut()).path =
        "sections/strokes.json".into();
    let duplicate = require_some!(manifest.entries.first()).clone();
    manifest.entries.push(duplicate);
    assert!(matches!(
        write_canonical_profile_manifest_json(
            &manifest, &["strokes", "layout"], bounds,
        ),
        Err(Error::Manifest(
            ProfileManifestError::DuplicateEntryPath { .. }
        )),
    ));
}

#[test]
fn contradictory_feature_requirements_cannot_be_canonically_serialized() {
    let mut profile = fixture();
    profile.optional_features.push("strokes".into());
    let expected = Error::Manifest(
        ProfileManifestError::ConflictingFeatureRequirement {
            feature: "strokes".into(),
        },
    );
    assert_eq!(
        planned_canonical_profile_manifest_json_bytes(
            &profile, &["strokes", "layout"], limits(),
        ),
        Err(expected.clone()),
    );
    assert_eq!(
        write_canonical_profile_manifest_json(
            &profile, &["strokes", "layout"], limits(),
        ),
        Err(expected.clone()),
    );
    let mut destination = [0xcau8; 1024];
    assert_eq!(
        write_canonical_profile_manifest_json_into(
            &profile, &["strokes", "layout"], limits(),
            &mut destination,
        ),
        Err(expected),
    );
    assert_eq!(destination, [0xca; 1024]);
}

#[test]
fn duplicated_features_cannot_escape_any_canonical_writer() {
    for list in [ProfileFeatureList::Required, ProfileFeatureList::Optional] {
        let mut profile = fixture();
        let feature = match list {
            ProfileFeatureList::Required => {
                profile.required_features.push("layout".into());
                "layout"
            }
            ProfileFeatureList::Optional => {
                profile.optional_features.push("future".into());
                "future"
            }
        };
        let expected = Error::Manifest(
            ProfileManifestError::DuplicateFeatureIdentifier {
                feature: feature.into(),
                list,
            },
        );
        assert_eq!(
            planned_canonical_profile_manifest_json_bytes(
                &profile, &["strokes", "layout"], limits(),
            ),
            Err(expected.clone()),
        );
        assert_eq!(
            write_canonical_profile_manifest_json(
                &profile, &["strokes", "layout"], limits(),
            ),
            Err(expected.clone()),
        );
        let mut destination = [0xbcu8; 1024];
        assert_eq!(
            write_canonical_profile_manifest_json_into(
                &profile, &["strokes", "layout"], limits(),
                &mut destination,
            ),
            Err(expected),
        );
        assert_eq!(destination, [0xbc; 1024]);
    }
}

#[test]
fn canonical_manifest_in_stored_zip_passes_source_attached_hash_checks() {
    let payload = b"{\"stroke\":true}";
    let mut manifest = fixture();
    manifest
        .entries
        .push(entry("sections/strokes.json", payload));
    let root_bytes = encode(&manifest);
    let members = [
        ProfileStoredZipMember {
            bytes: &root_bytes,
            name: "manifest.json",
        },
        ProfileStoredZipMember {
            bytes: payload,
            name: "sections/strokes.json",
        },
    ];
    let zip = require_ok!(write_stored_profile_zip(
        &members,
        ProfileStoredZipWriteLimits {
            maximum_archive_bytes: 16_384,
            maximum_entries: 2,
            maximum_entry_bytes: 8_192,
            maximum_total_bytes: 16_384,
        },
    ));
    let archive = require_ok!(read_stored_profile_zip(
        &zip,
        ProfileZipReadLimits {
            maximum_archive_bytes: zip.len(),
            maximum_entries: 2,
            maximum_entry_bytes: 8_192,
            maximum_total_bytes: 16_384,
        },
    ));
    let root = require_ok!(archive
        .admitted_root_json(ProfileJsonSyntaxLimits {
            maximum_bytes: 8192,
            maximum_nesting: 16,
            maximum_object_members: 32,
            maximum_values: 128,
        }));
    let source = require_ok!(decode_attached_profile_manifest_json(
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
        PathBuf::from("test.atrament"),
        ExplicitOverwriteIntent::PreserveExisting,
    ));
    let mut names = [""; 2];
    let verified = require_ok!(verify_attached_profile_import(
        &path,
        &source,
        &["strokes", "layout"],
        ProfileImportBudget {
            maximum_declared_bytes: 64,
            maximum_entries: 1,
            maximum_entry_bytes: 64,
        },
        &mut names,
    ));
    assert_eq!(
        verified.verified_entry_bytes("sections/strokes.json"),
        Some(&payload[..])
    );
    assert_eq!(verified.verified_entry_count(), 1);
    assert_eq!(verified.root_sha256(), sha256_profile_bytes(&root_bytes));
    assert_eq!(verified.archive_sha256(), sha256_profile_bytes(&zip));
    assert_eq!(encode(verified.manifest()), root_bytes);
}

#[test]
fn caller_owned_output_is_atomic_on_short_and_invalid_inputs() {
    let manifest = fixture();
    let bytes = encode(&manifest);
    let required = require_ok!(planned_canonical_profile_manifest_json_bytes(
        &manifest, &["layout", "strokes"], limits(),
    ));
    assert_eq!(required, bytes.len());

    for size in [0, 1, required - 1] {
        let mut destination = vec![0x5au8; size];
        assert_eq!(
            write_canonical_profile_manifest_json_into(
                &manifest, &["strokes", "layout"],
                limits(), &mut destination,
            ),
            Err(Error::DestinationTooSmall { required }),
        );
        assert!(destination.iter().all(|&byte| byte == 0x5a));
    }

    for size in [required, required + 1, required + 16] {
        let mut destination = vec![0x5au8; size];
        assert_eq!(
            write_canonical_profile_manifest_json_into(
                &manifest, &["layout", "strokes"],
                limits(), &mut destination,
            ),
            Ok(required),
        );
        assert_eq!(destination.get(..required), Some(bytes.as_slice()));
        assert!(destination.get(required..).is_some_and(|tail| {
            tail.iter().all(|&byte| byte == 0x5a)
        }));
    }

    let mut destination = vec![0x5au8; required + 16];
    let mut too_small = limits();
    too_small.maximum_bytes = required - 1;
    assert_eq!(
        write_canonical_profile_manifest_json_into(
            &manifest, &["layout", "strokes"],
            too_small, &mut destination,
        ),
        Err(Error::ManifestTooLarge),
    );
    assert_eq!(
        planned_canonical_profile_manifest_json_bytes(
            &manifest, &["layout", "strokes"], too_small,
        ),
        Err(Error::ManifestTooLarge),
    );
    assert!(destination.iter().all(|&byte| byte == 0x5a));

    let mut invalid = manifest;
    invalid.required_features.push("unknown".into());
    assert!(matches!(
        write_canonical_profile_manifest_json_into(
            &invalid, &["layout", "strokes"],
            limits(), &mut destination,
        ),
        Err(Error::Manifest(
            ProfileManifestError::UnsupportedRequiredFeature { .. }
        )),
    ));
    assert!(destination.iter().all(|&byte| byte == 0x5a));
}

#[test]
fn caller_owned_and_allocating_outputs_agree_across_512_manifests() {
    let mut state = 0x4930_fab2_7511_280du64;
    for iteration in 0usize..512 {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let mut manifest = fixture();
        manifest.profile_identity = format!(
            "profile-{iteration}-{}-\n\u{0000}\u{1f58b}\u{f1}\\\"",
            state.rem_euclid(10_000),
        );
        if state & 1 != 0 {
            manifest.required_features.reverse();
        }
        if state & 2 != 0 {
            manifest.optional_features.reverse();
        }
        let count = require_ok!(usize::try_from((state >> 4).rem_euclid(5)));
        for index in 0usize..count {
            let path = if (state >> (index + 10)) & 1 == 0 {
                format!("sections/{index:02}-\u{f1}.json")
            } else {
                format!("assets/{index:02}-\u{1f58b}.bin")
            };
            let mut declaration = entry(&path, b"payload");
            declaration.byte_length = u64::MAX
                .saturating_sub(u64::try_from(index).unwrap_or(u64::MAX));
            declaration.digest = Sha256Digest::from_bytes(
                [u8::try_from(index.saturating_mul(17)).unwrap_or(u8::MAX); 32],
            );
            manifest.entries.push(declaration);
        }
        if state & 4 != 0 {
            manifest.entries.reverse();
        }
        let owned = encode(&manifest);
        let required = require_ok!(
            planned_canonical_profile_manifest_json_bytes(
                &manifest, &["layout", "strokes"], limits(),
            )
        );
        assert_eq!(required, owned.len(), "iteration {iteration}");
        let mut destination = vec![0xa5; required + 7];
        assert_eq!(
            write_canonical_profile_manifest_json_into(
                &manifest, &["layout", "strokes"],
                limits(), &mut destination,
            ),
            Ok(required),
        );
        assert_eq!(
            destination.get(..required), Some(owned.as_slice()),
            "iteration {iteration}",
        );
        assert_eq!(destination.get(required..), Some(&[0xa5; 7][..]));
        let verified_bytes = require_some!(destination.get(..required));
        assert_eq!(encode(&decode(verified_bytes)), owned);
    }
}
