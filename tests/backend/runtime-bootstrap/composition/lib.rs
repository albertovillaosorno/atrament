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
//   - Browser launch recovery and real read-only profile CLI regressions.
// - Must-Not:
//   - Launch a browser, bind a listener or expose a session secret.
// - Allows:
//   - Inputs: Typed launch errors, test-owned ZIP files and CLI arguments.
//   - Outputs: Assertions over recovery and verified profile inspection.
//   - Side effects: Test-owned temporary files and read-only subprocesses.
// - Split-When:
//   - Profile CLI grows separate argument or section-decoding policies.
// - Merge-When:
//   - Runtime-bootstrap composition moves behind a library boundary.
// - Summary:
//   - Verifies no browser/session startup or opaque assets on profile CLI.
// - Description:
//   - Pins recovery and real ZIP/section rejection before stdout publication.
// - Usage:
//   - Run cargo test -p atrament_runtime_bootstrap.
// - Defaults:
//   - Any unverified profile byte refuses before printable inspection.
//

//! Runtime-bootstrap CLI and browser recovery regression fixtures.

#![expect(
    clippy::expect_used,
    clippy::missing_assert_message,
    clippy::missing_trait_methods,
    clippy::format_collect,
    reason = "deliberate fail-fast, tiny test-owned fixtures"
)]

#[expect(
    dead_code,
    clippy::redundant_pub_crate,
    reason = "binary source imported as test fixture, not deployed module"
)]
#[path = "../../../../src/backend/runtime-bootstrap/composition/main.rs"]
mod bootstrap;

use std::cell::Cell;
use std::env;
use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::{self, Write as _};
use std::iter;
use std::path::PathBuf;
use std::process::{self, Command};
use std::sync::atomic::{AtomicU64, Ordering};

use atrament_browser_launch::LaunchError;
use atrament_profile_entry_sha256::sha256_profile_bytes;
use atrament_profile_stored_zip_writer::{
    ProfileStoredZipMember, ProfileStoredZipWriteLimits,
    write_stored_profile_zip,
};

static TEST_FILE_ID: AtomicU64 = AtomicU64::new(0);

struct OwnedProfileFile(PathBuf);

impl OwnedProfileFile {
    fn new(bytes: &[u8]) -> Self {
        let path = env::temp_dir().join(format!(
            "atrament-cli-inspect-{}-{}.atrament",
            process::id(),
            TEST_FILE_ID.fetch_add(1, Ordering::Relaxed),
        ));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .expect("create exact test-owned profile path");
        file.write_all(bytes).expect("write owned profile fixture");
        Self(path)
    }
}

impl Drop for OwnedProfileFile {
    fn drop(&mut self) {
        fs::remove_file(&self.0).expect("remove owned test profile only");
    }
}

fn assert_recovery_error_is_secret_free(launch_error: &LaunchError) {
    let error = bootstrap::browser_launch_failure(launch_error);
    let message = error.to_string();
    assert!(message.contains("restart Atrament"));
    assert!(message.contains("credential is intentionally not published"));
    assert!(!message.contains("http://"));
    assert!(!message.contains("https://"));
    assert!(!message.contains("#session="));
    assert!(!message.contains("Open "));
}

#[test]
fn browser_launch_failure_requires_restart_without_publishing_access() {
    assert_recovery_error_is_secret_free(
        &LaunchError::GraphicalSessionUnavailable,
    );
}

#[test]
fn browser_launch_timeout_requires_restart_without_publishing_access() {
    assert_recovery_error_is_secret_free(&LaunchError::TimedOut);
}

fn digest_hex(bytes: &[u8]) -> String {
    sha256_profile_bytes(bytes)
        .bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn profile_zip(
    section: &[u8],
    asset: &[u8],
    declared_section: &[u8],
) -> Vec<u8> {
    profile_zip_with_media_type(
        section, asset, declared_section, "application/json",
    )
}

fn profile_zip_with_media_type(
    section: &[u8],
    asset: &[u8],
    declared_section: &[u8],
    media_type: &str,
) -> Vec<u8> {
    let root = format!(
        concat!(
            r#"{{"container_version":"atrament.profile/1","#,
            r#""profile_identity":"writer","required_features":[],"#,
            r#""optional_features":[],"entries":["#,
            r#"{{"path":"sections/writer.json","#,
            r#""media_type":"{}","#,
            r#""byte_length":{},"sha256":"{}"}},"#,
            r#"{{"path":"assets/raw.bin","#,
            r#""media_type":"application/octet-stream","#,
            r#""byte_length":{},"sha256":"{}"}}]}}"#,
        ),
        media_type,
        declared_section.len(),
        digest_hex(declared_section),
        asset.len(),
        digest_hex(asset),
    );
    write_stored_profile_zip(
        &[
            ProfileStoredZipMember {
                name: "assets/raw.bin",
                bytes: asset,
            },
            ProfileStoredZipMember {
                name: "manifest.json",
                bytes: root.as_bytes(),
            },
            ProfileStoredZipMember {
                name: "sections/writer.json",
                bytes: section,
            },
        ],
        ProfileStoredZipWriteLimits {
            maximum_archive_bytes: 4096,
            maximum_entries: 3,
            maximum_entry_bytes: 2048,
            maximum_total_bytes: 3000,
        },
    )
    .expect("valid CRC stored ZIP fixture")
}

#[test]
fn cli_args_reject_unknown_and_incomplete_commands_without_startup() {
    assert!(bootstrap::inspect_cli_args(Vec::<OsString>::new())
        .expect("normal startup dispatch")
        .is_none());
    for arguments in [
        vec!["profile"],
        vec!["inspect", "missing"],
        vec!["profile", "inspect"],
        vec!["profile", "inspect", "a", "extra"],
    ] {
        assert_eq!(
            bootstrap::inspect_cli_args(
                arguments.into_iter().map(OsString::from)
            )
                .expect_err("invalid commands cannot launch")
                .kind(),
            io::ErrorKind::InvalidInput,
        );
    }
}

#[test]
fn cli_rejects_infinite_argument_stream_after_four_values() {
    let pulls = Cell::new(0usize);
    let source = iter::from_fn(|| {
        pulls.set(pulls.get() + 1usize);
        Some(OsString::from("unexpected"))
    });
    assert_eq!(
        bootstrap::inspect_cli_args(source)
            .expect_err("unknown CLI command must fail boundedly")
            .kind(),
        io::ErrorKind::InvalidInput,
    );
    assert_eq!(pulls.get(), 4usize);
}

#[cfg(target_os = "linux")]
#[test]
fn cli_never_substitutes_a_missing_path_with_another_profile() {
    let bytes = profile_zip(
        br#"{"valid":true}"#, b"opaque", br#"{"valid":true}"#,
    );
    let owned = OwnedProfileFile::new(&bytes);
    let missing = owned.0.with_extension("missing");
    assert_eq!(
        bootstrap::inspect_profile(missing.clone())
            .expect_err("missing source must refuse")
            .kind(),
        io::ErrorKind::InvalidData,
    );
    let executed = Command::new(env!("CARGO_BIN_EXE_atrament"))
        .args(["profile", "inspect"])
        .arg(&missing)
        .output()
        .expect("run missing-source profile CLI");
    assert!(!executed.status.success());
    assert!(executed.stdout.is_empty());
}

#[cfg(target_os = "linux")]
#[test]
fn cli_rejects_symlinked_selection_before_any_output() {
    use std::os::unix::fs::symlink;
    let section = br#"{"valid":true}"#;
    let owned = OwnedProfileFile::new(&profile_zip(
        section, b"opaque", section,
    ));
    let link = owned.0.with_extension("linked");
    symlink(&owned.0, &link).expect("create test-owned symlink");
    let executed = Command::new(env!("CARGO_BIN_EXE_atrament"))
        .args(["profile", "inspect"])
        .arg(&link)
        .output()
        .expect("run symlink-refusal profile CLI");
    fs::remove_file(&link).expect("remove test-owned symlink");
    assert!(!executed.status.success());
    assert!(executed.stdout.is_empty());
}

#[cfg(target_os = "linux")]
#[test]
fn cli_inspects_verified_section_without_extracting_opaque_assets() {
    let section = br#"{"strokes":[1,2,3]}"#;
    let asset = b"opaque-asset-secret-123";
    let owned = OwnedProfileFile::new(&profile_zip(section, asset, section));
    let readable = bootstrap::inspect_profile(owned.0.clone())
        .expect("securely inspect exact local CLI-selected file");
    assert!(readable.contains("manifest.json (verified)"));
    assert!(readable.contains(
        "sections/writer.json (syntax-checked; untyped)"
    ));
    assert!(readable.contains("{\"strokes\":[1,2,3]}"));
    assert!(readable.contains("assets/raw.bin"), "manifest retains asset");
    assert!(!readable.contains("opaque-asset-secret-123"));
    assert!(!readable.contains("#session="));
    let executed = Command::new(env!("CARGO_BIN_EXE_atrament"))
        .args(["profile", "inspect"])
        .arg(&owned.0)
        .output()
        .expect("run real CLI without browser");
    assert!(executed.status.success());
    assert_eq!(executed.stdout, readable.as_bytes());
    assert!(executed.stderr.is_empty());
}

#[cfg(target_os = "linux")]
#[test]
fn cli_section_order_is_stable_across_manifest_declaration_orders() {
    let first = br#"{"first":true}"#;
    let last = br#"{"last":true}"#;
    let entry = |name: &str, bytes: &[u8]| {
        format!(
            concat!(
                r#"{{"path":"sections/{}.json","#,
                r#""media_type":"application/json","byte_length":{},"#,
                r#""sha256":"{}"}}"#,
            ),
            name, bytes.len(), digest_hex(bytes),
        )
    };
    let first_entry = entry("a", first);
    let last_entry = entry("z", last);
    let mut outputs = Vec::new();
    for declarations in [
        format!("{last_entry},{first_entry}"),
        format!("{first_entry},{last_entry}"),
    ] {
        let root = format!(
            concat!(
                r#"{{"container_version":"atrament.profile/1","#,
                r#""profile_identity":"writer","required_features":[],"#,
                r#""optional_features":[],"entries":[{}]}}"#,
            ),
            declarations,
        );
        let bytes = write_stored_profile_zip(
            &[
                ProfileStoredZipMember {
                    name: "manifest.json",
                    bytes: root.as_bytes(),
                },
                ProfileStoredZipMember {
                    name: "sections/a.json",
                    bytes: first,
                },
                ProfileStoredZipMember {
                    name: "sections/z.json",
                    bytes: last,
                },
            ],
            ProfileStoredZipWriteLimits {
                maximum_archive_bytes: 4096,
                maximum_entries: 3,
                maximum_entry_bytes: 2048,
                maximum_total_bytes: 3000,
            },
        )
        .expect("valid reordered declarations with sorted ZIP members");
        let source = OwnedProfileFile::new(&bytes);
        let inspected = bootstrap::inspect_profile(source.0.clone())
            .expect("all entries verify before stable display");
        let executed = Command::new(env!("CARGO_BIN_EXE_atrament"))
            .args(["profile", "inspect"])
            .arg(&source.0)
            .output()
            .expect("read-only CLI");
        assert!(executed.status.success());
        assert_eq!(executed.stdout, inspected.as_bytes());
        outputs.push(inspected);
    }
    assert_eq!(outputs.len(), 2);
    assert_eq!(outputs.first(), outputs.last());
    let readable = outputs.first().expect("one admitted fixture");
    assert!(
        readable.find("sections/a.json").expect("first section")
            < readable.find("sections/z.json").expect("last section"),
        "readable sections must use canonical path order",
    );
}

#[cfg(target_os = "linux")]
#[test]
fn cli_preserves_exact_non_utf8_selected_profile_path() {
    use std::os::unix::ffi::OsStringExt as _;

    let section = br#"{"strokes":[1,2]}"#;
    let mut owned = OwnedProfileFile::new(&profile_zip(
        section, b"\0\xffbinary", section,
    ));
    let mut selected_bytes = owned.0.as_os_str().as_encoded_bytes().to_vec();
    selected_bytes.push(0xff);
    let selected = PathBuf::from(OsString::from_vec(selected_bytes));
    fs::rename(&owned.0, &selected)
        .expect("rename only test-owned profile to opaque path");
    owned.0 = selected;

    let inspected = bootstrap::inspect_profile(owned.0.clone())
        .expect("exact opaque source path remains valid Import selection");
    assert!(inspected.contains("sections/writer.json"));
    assert!(!inspected.contains("binary"));
    let executed = Command::new(env!("CARGO_BIN_EXE_atrament"))
        .args(["profile", "inspect"])
        .arg(&owned.0)
        .output()
        .expect("OS argv accepts caller-selected opaque path bytes");
    assert!(executed.status.success());
    assert_eq!(executed.stdout, inspected.as_bytes());
    assert!(executed.stderr.is_empty());
}

#[cfg(target_os = "linux")]
#[test]
fn cli_refuses_later_invalid_section_without_partial_stdout() {
    let first = br#"{"valid":true}"#;
    let later = b"[invalid-section]";
    let opaque = b"\0\xffprivate sample";
    let manifest = format!(
        concat!(
            r#"{{"container_version":"atrament.profile/1","#,
            r#""profile_identity":"writer","required_features":[],"#,
            r#""optional_features":[],"entries":["#,
            r#"{{"path":"sections/a.json","media_type":"application/json","#,
            r#""byte_length":{},"sha256":"{}"}},"#,
            r#"{{"path":"sections/z.json","media_type":"application/json","#,
            r#""byte_length":{},"sha256":"{}"}},"#,
            r#"{{"path":"assets/raw.bin","#,
            r#""media_type":"application/octet-stream","#,
            r#""byte_length":{},"sha256":"{}"}}]}}"#,
        ),
        first.len(), digest_hex(first),
        later.len(), digest_hex(later),
        opaque.len(), digest_hex(opaque),
    );
    let archive = write_stored_profile_zip(
        &[
            ProfileStoredZipMember {
                name: "assets/raw.bin",
                bytes: opaque,
            },
            ProfileStoredZipMember {
                name: "manifest.json",
                bytes: manifest.as_bytes(),
            },
            ProfileStoredZipMember {
                name: "sections/a.json",
                bytes: first,
            },
            ProfileStoredZipMember {
                name: "sections/z.json",
                bytes: later,
            },
        ],
        ProfileStoredZipWriteLimits {
            maximum_archive_bytes: 4096,
            maximum_entries: 4,
            maximum_entry_bytes: 2048,
            maximum_total_bytes: 3000,
        },
    )
    .expect("authentic fully SHA-verified two-section ZIP");
    let owned = OwnedProfileFile::new(&archive);
    let error = bootstrap::inspect_profile(owned.0.clone())
        .expect_err("later invalid section refuses before any inspection");
    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert!(error.to_string().contains("SectionJson { index: 1"));
    let executed = Command::new(env!("CARGO_BIN_EXE_atrament"))
        .args(["profile", "inspect"])
        .arg(&owned.0)
        .output()
        .expect("run actually malformed second-section profile CLI");
    assert!(!executed.status.success());
    assert!(executed.stdout.is_empty());
    assert!(!executed.stderr.is_empty());
}

#[cfg(target_os = "linux")]
#[test]
fn cli_refuses_crc_valid_sha_forgery_without_printing_manifest() {
    let authentic = br#"{"genuine":true}"#;
    let forgery = br#"{"genuine":null}"#;
    assert_eq!(authentic.len(), forgery.len());
    let owned = OwnedProfileFile::new(&profile_zip(
        forgery, b"binary", authentic,
    ));
    let error = bootstrap::inspect_profile(owned.0.clone())
        .expect_err("CRC-correct forged data cannot inspect");
    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    let executed = Command::new(env!("CARGO_BIN_EXE_atrament"))
        .args(["profile", "inspect"])
        .arg(&owned.0)
        .output()
        .expect("run forged profile CLI");
    assert!(!executed.status.success());
    assert!(executed.stdout.is_empty());
    assert!(!executed.stderr.is_empty());
}

#[cfg(target_os = "linux")]
#[test]
fn cli_refuses_sha_valid_json_with_non_json_type_without_stdout() {
    let json = br#"{"strokes":[]}"#;
    for media_type in ["image/png", "text/plain"] {
        let archive = profile_zip_with_media_type(
            json, b"\xff\x00opaque", json, media_type,
        );
        let owned = OwnedProfileFile::new(&archive);
        let error = bootstrap::inspect_profile(owned.0.clone())
            .expect_err("JSON section with misleading type cannot inspect");
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(error.to_string().contains("SectionMediaType { index: 0"));
        let executed = Command::new(env!("CARGO_BIN_EXE_atrament"))
            .args(["profile", "inspect"])
            .arg(&owned.0)
            .output()
            .expect("run securely selected profile inspector");
        assert!(!executed.status.success());
        assert!(executed.stdout.is_empty());
        assert!(!executed.stderr.is_empty());
    }
}

#[cfg(target_os = "linux")]
#[test]
fn cli_refuses_sha_valid_but_invalid_section_json_atomically() {
    let section = b"{invalid-json}";
    let owned = OwnedProfileFile::new(&profile_zip(
        section, b"binary", section,
    ));
    let error = bootstrap::inspect_profile(owned.0.clone())
        .expect_err("section JSON syntax must be checked");
    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    let executed = Command::new(env!("CARGO_BIN_EXE_atrament"))
        .args(["profile", "inspect"])
        .arg(&owned.0)
        .output()
        .expect("run malformed section CLI");
    assert!(!executed.status.success());
    assert!(executed.stdout.is_empty());
}
