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
use atrament_profile_zip_reader::{
    ProfileZipReadLimits, read_stored_profile_zip,
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

/// Own only the exact test-selected destination, not its parent directory.
struct OwnedProfileDestination(PathBuf);

impl Drop for OwnedProfileDestination {
    fn drop(&mut self) {
        match fs::remove_file(&self.0) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => {
                panic!("cannot remove test-owned destination: {error:?}")
            }
        }
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
    profile_zip_with_optional_features(
        section, asset, declared_section, media_type, "[]",
    )
}

fn profile_zip_with_optional_features(
    section: &[u8],
    asset: &[u8],
    declared_section: &[u8],
    media_type: &str,
    optional_features_json: &str,
) -> Vec<u8> {
    let root = format!(
        concat!(
            r#"{{"container_version":"atrament.profile/1","#,
            r#""profile_identity":"writer","required_features":[],"#,
            r#""optional_features":{},"entries":["#,
            r#"{{"path":"sections/writer.json","#,
            r#""media_type":"{}","#,
            r#""byte_length":{},"sha256":"{}"}},"#,
            r#"{{"path":"assets/raw.bin","#,
            r#""media_type":"application/octet-stream","#,
            r#""byte_length":{},"sha256":"{}"}}]}}"#,
        ),
        optional_features_json,
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
        vec!["profile", "repack"],
        vec!["profile", "repack", "a"],
        vec!["profile", "repack", "a", "b", "extra"],
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
fn cli_repack_canonically_preserves_verified_sections_assets_and_features() {
    let section = br#"{"private":"verified-section-9570"}"#;
    let asset = b"\0\xffopaque-repack-asset-3262";
    let original = profile_zip_with_optional_features(
        section, asset, section, "application/json", r#"["future.writer"]"#,
    );
    let source = OwnedProfileFile::new(&original);
    let destination = OwnedProfileDestination(
        source.0.with_extension("canonical.atrament"),
    );
    let output = Command::new(env!("CARGO_BIN_EXE_atrament"))
        .args(["profile", "repack"])
        .arg(&source.0)
        .arg(&destination.0)
        .output()
        .expect("repack selected source into exact new destination");
    assert!(output.status.success());
    assert_eq!(
        output.stdout,
        b"profile repacked (verified; no session import)\n",
    );
    assert!(output.stderr.is_empty());
    assert_eq!(fs::read(&source.0).expect("source remains"), original);
    let repacked = fs::read(&destination.0).expect("complete repacked ZIP");
    assert_ne!(repacked, original, "noncanonical manifest is normalized");
    let archive = read_stored_profile_zip(
        &repacked,
        ProfileZipReadLimits {
            maximum_archive_bytes: 4096,
            maximum_entries: 3,
            maximum_entry_bytes: 2048,
            maximum_total_bytes: 3000,
        },
    ).expect("repacked archive passes independent CRC framing");
    assert!(archive.matches_stored_writer_framing());
    let entries = archive.entries();
    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0].name(), "assets/raw.bin");
    assert_eq!(entries[0].bytes(), asset);
    assert_eq!(entries[1].name(), "manifest.json");
    assert_eq!(entries[2].name(), "sections/writer.json");
    assert_eq!(entries[2].bytes(), section);
    let root = archive.manifest_utf8(4096).expect("canonical manifest");
    assert!(root.contains(r#""optional_features":["future.writer"]"#));
    assert!(root.starts_with(
        r#"{"container_version":"atrament.profile/1","entries":["#,
    ));
    let inspected = bootstrap::inspect_profile(destination.0.clone())
        .expect("canonical selected output reimports with SHA verification");
    assert!(inspected.contains("future.writer"));
    assert!(inspected.contains("verified-section-9570"));
    assert!(!inspected.contains("opaque-repack-asset-3262"));
}

#[cfg(target_os = "linux")]
#[test]
fn cli_repack_preserves_exact_non_utf8_destination_selection() {
    use std::os::unix::ffi::{OsStrExt as _, OsStringExt as _};

    let section = br#"{"verified":true}"#;
    let source = OwnedProfileFile::new(&profile_zip(
        section, b"non-utf8-asset", section,
    ));
    let mut name = source.0.file_name()
        .expect("owned source filename").as_bytes().to_vec();
    name.extend_from_slice(b"-destination-");
    name.push(0xff);
    name.extend_from_slice(b".atrament");
    let destination = OwnedProfileDestination(
        source.0.with_file_name(OsString::from_vec(name)),
    );
    let executed = Command::new(env!("CARGO_BIN_EXE_atrament"))
        .args(["profile", "repack"])
        .arg(&source.0)
        .arg(&destination.0)
        .output()
        .expect("OS argv can preserve non-UTF8 destination bytes");
    assert!(executed.status.success());
    assert!(executed.stderr.is_empty());
    assert!(destination.0.is_file());
    let observed = bootstrap::inspect_profile(destination.0.clone())
        .expect("new exact output passes independent selected Import");
    assert!(observed.contains("verified"));
}

#[cfg(target_os = "linux")]
#[test]
fn cli_repack_refuses_existing_target_without_replacing_private_bytes() {
    let section = br#"{"private":"profile-secret-1954"}"#;
    let original = profile_zip(section, b"asset-secret-5688", section);
    let source = OwnedProfileFile::new(&original);
    let destination = OwnedProfileDestination(
        source.0.with_extension("existing.atrament"),
    );
    fs::write(&destination.0, b"important original output")
        .expect("pre-existing test-owned target");
    let executed = Command::new(env!("CARGO_BIN_EXE_atrament"))
        .args(["profile", "repack"])
        .arg(&source.0)
        .arg(&destination.0)
        .output()
        .expect("run existing-target repack");
    assert!(!executed.status.success());
    assert!(executed.stdout.is_empty());
    assert!(String::from_utf8_lossy(&executed.stderr).contains("TargetExists"));
    assert_eq!(
        fs::read(&destination.0).expect("old target is untouched"),
        b"important original output",
    );
    assert_eq!(fs::read(&source.0).expect("source remains"), original);
    let stderr = String::from_utf8_lossy(&executed.stderr);
    for private in ["profile-secret-1954", "asset-secret-5688"] {
        assert!(!stderr.contains(private));
    }
}

#[cfg(target_os = "linux")]
#[test]
fn cli_repack_refuses_symlink_destination_without_touching_victim() {
    use std::os::unix::fs::symlink;

    let section = br#"{"private":"selection-3791"}"#;
    let source = OwnedProfileFile::new(&profile_zip(
        section, b"private-asset-9045", section,
    ));
    let victim = OwnedProfileDestination(
        source.0.with_extension("victim.atrament"),
    );
    fs::write(&victim.0, b"victim must not change")
        .expect("test-owned non-selected victim");
    let destination = OwnedProfileDestination(
        source.0.with_extension("linked.atrament"),
    );
    symlink(&victim.0, &destination.0)
        .expect("attacker-controlled link at selected output");
    let executed = Command::new(env!("CARGO_BIN_EXE_atrament"))
        .args(["profile", "repack"])
        .arg(&source.0)
        .arg(&destination.0)
        .output()
        .expect("run repack into symlinked target");
    assert!(!executed.status.success());
    assert!(executed.stdout.is_empty());
    assert_eq!(
        fs::read_link(&destination.0).expect("symlink remains"),
        victim.0,
    );
    assert_eq!(
        fs::read(&victim.0).expect("original victim remains"),
        b"victim must not change",
    );
    let stderr = String::from_utf8_lossy(&executed.stderr);
    assert!(!stderr.contains("selection-3791"));
    assert!(!stderr.contains("private-asset-9045"));
}

#[cfg(target_os = "linux")]
#[test]
fn cli_repack_refuses_invalid_later_section_without_partial_target() {
    let early = br#"{"private":"early-5726"}"#;
    let late = b"[not-an-object]";
    let root = format!(
        concat!(
            r#"{{"container_version":"atrament.profile/1", "#,
            r#""profile_identity":"writer","required_features":[],"#,
            r#""optional_features":[],"entries":["#,
            r#"{{"path":"sections/a.json","media_type":"application/json", "#,
            r#""byte_length":{},"sha256":"{}"}},"#,
            r#"{{"path":"sections/z.json","media_type":"application/json", "#,
            r#""byte_length":{},"sha256":"{}"}}]}}"#,
        ),
        early.len(), digest_hex(early), late.len(), digest_hex(late),
    );
    let source = OwnedProfileFile::new(&write_stored_profile_zip(
        &[
            ProfileStoredZipMember {
                name: "manifest.json", bytes: root.as_bytes(),
            },
            ProfileStoredZipMember {
                name: "sections/a.json", bytes: early,
            },
            ProfileStoredZipMember {
                name: "sections/z.json", bytes: late,
            },
        ],
        ProfileStoredZipWriteLimits {
            maximum_archive_bytes: 4096,
            maximum_entries: 3,
            maximum_entry_bytes: 2048,
            maximum_total_bytes: 3000,
        },
    ).expect("authentic SHA-valid but section-invalid source"));
    let destination = OwnedProfileDestination(
        source.0.with_extension("late-invalid.atrament"),
    );
    let executed = Command::new(env!("CARGO_BIN_EXE_atrament"))
        .args(["profile", "repack"])
        .arg(&source.0)
        .arg(&destination.0)
        .output()
        .expect("run late-section-invalid repack");
    assert!(!executed.status.success());
    assert!(executed.stdout.is_empty());
    assert!(!destination.0.exists());
    let stderr = String::from_utf8_lossy(&executed.stderr);
    assert!(stderr.contains("SectionJson { index: 1"));
    assert!(!stderr.contains("early-5726"));
}

#[cfg(target_os = "linux")]
#[test]
fn cli_repack_refuses_sha_forgery_without_creating_any_output() {
    let actual = br#"{"valid":null}"#;
    let claimed = br#"{"valid":true}"#;
    assert_eq!(actual.len(), claimed.len());
    let source = OwnedProfileFile::new(&profile_zip(
        actual, b"opaque-private-7684", claimed,
    ));
    let destination = OwnedProfileDestination(
        source.0.with_extension("forgery-out.atrament"),
    );
    let executed = Command::new(env!("CARGO_BIN_EXE_atrament"))
        .args(["profile", "repack"])
        .arg(&source.0)
        .arg(&destination.0)
        .output()
        .expect("run forged source repack");
    assert!(!executed.status.success());
    assert!(executed.stdout.is_empty());
    assert!(!destination.0.exists());
    let stderr = String::from_utf8_lossy(&executed.stderr);
    assert!(stderr.contains("DigestMismatch"));
    assert!(!stderr.contains("opaque-private-7684"));
}

#[cfg(target_os = "linux")]
#[test]
fn cli_never_substitutes_a_missing_path_with_another_profile() {
    let section = br#"{"valid":"private-section-6421"}"#;
    let bytes = profile_zip(section, b"private-asset-2851", section);
    let owned = OwnedProfileFile::new(&bytes);
    let missing = owned.0.with_extension("private-missing-selection-4852");
    let error = bootstrap::inspect_profile(missing.clone())
        .expect_err("missing source must refuse");
    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert!(error.to_string().contains("NotFound"));
    let executed = Command::new(env!("CARGO_BIN_EXE_atrament"))
        .args(["profile", "inspect"])
        .arg(&missing)
        .output()
        .expect("run missing-source profile CLI");
    assert!(!executed.status.success());
    assert!(executed.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&executed.stderr);
    assert!(stderr.contains("NotFound"));
    for private in [
        "private-missing-selection-4852",
        "private-section-6421",
        "private-asset-2851",
    ] {
        assert!(!error.to_string().contains(private));
        assert!(!stderr.contains(private));
    }
}

#[cfg(target_os = "linux")]
#[test]
fn cli_rejects_symlinked_selection_before_any_output() {
    use std::os::unix::fs::symlink;
    let section = br#"{"valid":"private-link-section-3096"}"#;
    let owned = OwnedProfileFile::new(&profile_zip(
        section, b"private-link-asset-5742", section,
    ));
    let link = owned.0.with_extension("private-selected-link-8331");
    symlink(&owned.0, &link).expect("create test-owned symlink");
    let executed = Command::new(env!("CARGO_BIN_EXE_atrament"))
        .args(["profile", "inspect"])
        .arg(&link)
        .output()
        .expect("run symlink-refusal profile CLI");
    fs::remove_file(&link).expect("remove test-owned symlink");
    assert!(!executed.status.success());
    assert!(executed.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&executed.stderr);
    assert!(stderr.contains("SymlinkForbidden"));
    for private in [
        "private-selected-link-8331",
        "private-link-section-3096",
        "private-link-asset-5742",
    ] {
        assert!(!stderr.contains(private));
    }
    assert_eq!(
        fs::read(&owned.0).expect("selected symlink never changed target"),
        profile_zip(section, b"private-link-asset-5742", section),
    );
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
    let first = br#"{"valid":"private-early-section-8392"}"#;
    let later = b"[private-invalid-late-section-4752]";
    let opaque = b"\0\xffprivate-opaque-asset-2925";
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
    let mut owned = OwnedProfileFile::new(&archive);
    let selected = owned.0.with_extension("private-late-selected-6614");
    fs::rename(&owned.0, &selected)
        .expect("rename the only test-owned profile source");
    owned.0 = selected;
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
    let stderr = String::from_utf8_lossy(&executed.stderr);
    assert!(stderr.contains("SectionJson { index: 1"));
    for private in [
        "private-early-section-8392",
        "private-invalid-late-section-4752",
        "private-opaque-asset-2925",
        "private-late-selected-6614",
    ] {
        assert!(!error.to_string().contains(private));
        assert!(!stderr.contains(private));
    }
}

#[cfg(target_os = "linux")]
#[test]
fn cli_refuses_duplicate_optional_feature_without_partial_stdout() {
    let section = br#"{"private":"section-payload-8621"}"#;
    let asset = b"asset-payload-9345";
    let secret_feature = "feature-private-7103";
    let optional_features = format!(
        r#"["{secret_feature}","{secret_feature}"]"#,
    );
    let archive = profile_zip_with_optional_features(
        section, asset, section, "application/json", &optional_features,
    );
    let mut owned = OwnedProfileFile::new(&archive);
    let private_path = owned
        .0
        .with_extension("private-selection-5218.atrament");
    fs::rename(&owned.0, &private_path)
        .expect("rename only owned profile to private selected path");
    owned.0 = private_path;

    let error = bootstrap::inspect_profile(owned.0.clone())
        .expect_err("repeated capabilities cannot be inspected");
    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert!(error.to_string().contains("DuplicateFeatureIdentifier"));
    let executed = Command::new(env!("CARGO_BIN_EXE_atrament"))
        .args(["profile", "inspect"])
        .arg(&owned.0)
        .output()
        .expect("run duplicate-feature profile CLI");
    assert!(!executed.status.success());
    assert!(executed.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&executed.stderr);
    assert!(stderr.contains("DuplicateFeatureIdentifier"));
    for private in [
        secret_feature,
        "section-payload-8621",
        "asset-payload-9345",
        "private-selection-5218",
    ] {
        assert!(!error.to_string().contains(private));
        assert!(!stderr.contains(private));
    }
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
