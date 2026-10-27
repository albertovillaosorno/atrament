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

use atrament_explicit_file_path_intent::{
    ExplicitFilePathSource, ExplicitFilePurpose, ExplicitOverwriteIntent,
    classify_explicit_file_path_intent,
};
use atrament_handwriting_profile_container::{
    PROFILE_CONTAINER_VERSION, ProfileManifest, ProfileManifestEntry,
    ProfileManifestError,
    Sha256Digest,
};
use atrament_profile_entry_sha256::sha256_profile_bytes;
use atrament_profile_import_preflight::ProfileImportBudget;
use atrament_profile_import_verification::verify_attached_profile_import;
use atrament_profile_json_syntax::ProfileJsonSyntaxLimits;
use atrament_profile_manifest_json_codec::{
    ProfileManifestJsonLimits, decode_attached_profile_manifest_json,
    decode_profile_manifest_json,
};
use atrament_profile_manifest_json_writer::{
    ProfileManifestJsonWriteError as Error, ProfileManifestJsonWriteLimits,
    write_canonical_profile_manifest_json,
};
use atrament_profile_stored_zip_writer::{
    ProfileStoredZipMember, ProfileStoredZipWriteLimits,
    write_stored_profile_zip,
};
use atrament_profile_zip_reader::{
    ProfileZipReadLimits, read_stored_profile_zip,
};

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
        container_version: PROFILE_CONTAINER_VERSION.to_string(),
        entries: vec![],
        optional_features: vec!["mañana".into(), "future".into()],
        profile_identity: "Writer 🖋".into(),
        required_features: vec!["strokes".into(), "layout".into()],
    }
}

fn entry(path: &str, payload: &[u8]) -> ProfileManifestEntry {
    ProfileManifestEntry {
        byte_length: u64::try_from(payload.len()).unwrap(),
        digest: sha256_profile_bytes(payload),
        media_type: "application/json".into(),
        path: path.into(),
    }
}

fn encode(manifest: &ProfileManifest) -> Vec<u8> {
    write_canonical_profile_manifest_json(
        manifest, &["strokes", "layout"], limits(),
    )
        .expect("valid fixture writes as deterministic JSON")
}

fn decode(bytes: &[u8]) -> ProfileManifest {
    let root = atrament_profile_json_syntax::validate_profile_json_object(
        bytes,
        ProfileJsonSyntaxLimits {
            maximum_bytes: 8192,
            maximum_nesting: 16,
            maximum_object_members: 32,
            maximum_values: 128,
        },
    )
    .expect("writer bytes are accepted by independent JSON grammar");
    decode_profile_manifest_json(
        root,
        ProfileManifestJsonLimits {
            maximum_entries: 8,
            maximum_features: 8,
            maximum_string_bytes: 256,
        },
    )
    .expect("writer schema is accepted by independent typed decoder")
}

#[test]
fn golden_root_only_json_has_fixed_key_and_feature_order() {
    let mut manifest = fixture();
    manifest.profile_identity = "writer-1".into();
    manifest.optional_features.clear();
    let actual = encode(&manifest);
    assert_eq!(
        std::str::from_utf8(&actual).unwrap(),
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
    assert_eq!(decoded.entries[0].path, "assets/b.bin");
    assert_eq!(decoded.entries[1].path, "sections/z.json");
    assert_eq!(decoded.optional_features, ["future", "mañana"]);
    assert_eq!(decoded.required_features, ["layout", "strokes"]);
    assert_eq!(encode(&decoded), bytes);
}

#[test]
fn strings_escape_controls_but_retain_unicode_and_hex_digest_bytes() {
    let mut manifest = fixture();
    manifest.profile_identity = concat!(
        "ñ 🖋 \" \\ \0 \u{001f}",
        "\n\r\t\u{0008}\u{000c}",
    ).into();
    manifest.entries.push(ProfileManifestEntry {
        byte_length: u64::MAX,
        digest: Sha256Digest::from_bytes([0xab; 32]),
        media_type: "application/json".into(),
        path: "assets/ñ.json".into(),
    });
    let bytes = encode(&manifest);
    let text = std::str::from_utf8(&bytes).unwrap();
    assert!(text.contains(r#"ñ 🖋 \" \\ \u0000 \u001f\n\r\t\b\f"#));
    assert!(text.contains("18446744073709551615"));
    assert!(text.contains(&"ab".repeat(32)));
    let decoded = decode(&bytes);
    assert_eq!(decoded.profile_identity, manifest.profile_identity);
    assert_eq!(decoded.entries[0].byte_length, u64::MAX);
    assert_eq!(decoded.entries[0].digest, manifest.entries[0].digest);
    assert_eq!(encode(&decoded), bytes);
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
        Ok(actual.clone()),
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
    manifest.entries[0].path = "assets/../bad".into();
    assert!(matches!(
        write_canonical_profile_manifest_json(
            &manifest, &["strokes", "layout"], bounds,
        ),
        Err(Error::Manifest(
            ProfileManifestError::InvalidEntryPath { .. }
        )),
    ));
    manifest.entries[0].path = "sections/strokes.json".into();
    manifest.entries.push(manifest.entries[0].clone());
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
    let zip = write_stored_profile_zip(
        &members,
        ProfileStoredZipWriteLimits {
            maximum_archive_bytes: 16_384,
            maximum_entries: 2,
            maximum_entry_bytes: 8_192,
            maximum_total_bytes: 16_384,
        },
    )
    .expect("canonical ZIP framing");
    let archive = read_stored_profile_zip(
        &zip,
        ProfileZipReadLimits {
            maximum_archive_bytes: zip.len(),
            maximum_entries: 2,
            maximum_entry_bytes: 8_192,
            maximum_total_bytes: 16_384,
        },
    )
    .expect("checked CRC-framed ZIP");
    let root = archive
        .admitted_root_json(ProfileJsonSyntaxLimits {
            maximum_bytes: 8192,
            maximum_nesting: 16,
            maximum_object_members: 32,
            maximum_values: 128,
        })
        .expect("attached syntax-admitted ZIP root");
    let source = decode_attached_profile_manifest_json(
        &root,
        ProfileManifestJsonLimits {
            maximum_entries: 8,
            maximum_features: 8,
            maximum_string_bytes: 256,
        },
    )
    .expect("source-backed typed manifest");
    let path = classify_explicit_file_path_intent(
        ExplicitFilePurpose::ImportProfile,
        ExplicitFilePathSource::CallerSelection,
        PathBuf::from("test.atrament"),
        ExplicitOverwriteIntent::PreserveExisting,
    )
    .expect("synthetic path intent, not file authority");
    let mut names = [""; 2];
    let verified = verify_attached_profile_import(
        &path,
        &source,
        &["strokes", "layout"],
        ProfileImportBudget {
            maximum_declared_bytes: 64,
            maximum_entries: 1,
            maximum_entry_bytes: 64,
        },
        &mut names,
    )
    .expect("typed source-bound complete payload verification");
    assert_eq!(
        verified.verified_entry_bytes("sections/strokes.json"),
        Some(&payload[..])
    );
    assert_eq!(verified.verified_entry_count(), 1);
    assert_eq!(verified.root_sha256(), sha256_profile_bytes(&root_bytes));
    assert_eq!(verified.archive_sha256(), sha256_profile_bytes(&zip));
    assert_eq!(encode(verified.manifest()), root_bytes);
}
