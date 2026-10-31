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
//   - Browser launch recovery and real selected-profile CLI regressions.
// - Must-Not:
//   - Launch a browser, bind a listener or expose a session secret.
// - Allows:
//   - Inputs: Typed launch errors, test-owned ZIP files and CLI arguments.
//   - Outputs: Assertions over recovery and verified profile inspection.
//   - Side effects: Test-owned files and bounded CLI subprocesses only.
// - Split-When:
//   - Profile CLI grows separate argument or section-decoding policies.
// - Merge-When:
//   - Runtime-bootstrap composition moves behind a library boundary.
// - Summary:
//   - Verifies no browser/session startup or opaque assets on profile CLI.
// - Description:
//   - Pins ZIP/section refusal and exact opt-in output commit behavior.
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
use std::fmt::Write as _;
use std::fs::{self, OpenOptions};
use std::io::{self, Write as _};
use std::iter;
use std::path::PathBuf;
use std::process::{self, Command};
use std::sync::atomic::{AtomicU64, Ordering};

use atrament_browser_launch::LaunchError;
use atrament_explicit_file_path_intent::ExplicitOverwriteIntent;
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
            Err(error) => assert_eq!(
                error.kind(), io::ErrorKind::NotFound,
                "cannot remove test-owned destination",
            ),
        }
    }
}

/// Own only the exact temporary directory created by this test.
#[cfg(target_os = "linux")]
struct OwnedProfileDirectory(PathBuf);

#[cfg(target_os = "linux")]
impl OwnedProfileDirectory {
    #[expect(
        clippy::create_dir,
        reason = "fixture must refuse an existing directory, not reuse it"
    )]
    fn new(path: PathBuf) -> Self {
        fs::create_dir(&path).expect("create test-owned directory");
        Self(path)
    }
}

#[cfg(target_os = "linux")]
impl Drop for OwnedProfileDirectory {
    fn drop(&mut self) {
        fs::remove_dir(&self.0)
            .expect("remove empty test-owned directory");
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
        vec!["profile", "verify"],
        vec!["profile", "check-canonical"],
        vec!["profile", "check-canonical", "a", "extra"],
        vec!["profile", "verify", "a", "extra"],
        vec!["profile", "inspect", "a", "extra"],
        vec!["profile", "repack"],
        vec!["profile", "repack", "a"],
        vec!["profile", "repack", "a", "b", "extra"],
        vec!["profile", "repack", "--replace"],
        vec!["profile", "repack", "--replace", "a"],
        vec!["profile", "repack", "--replace", "a", "b", "extra"],
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
fn repack_cli_refuses_ambiguous_bare_option_as_file_path() {
    for arguments in [
        vec!["profile", "repack", "not-a-file", "--replace"],
        vec!["profile", "repack", "not-a-file", "--replcae"],
        vec!["profile", "repack", "not-a-file", "-x"],
        vec!["profile", "repack", "-unknown", "target"],
        vec!["profile", "repack", "--replace", "--replace", "target"],
        vec!["profile", "repack", "--replace", "-unknown", "target"],
        vec!["profile", "repack", "--replace", "not-a-file", "--replace"],
        vec!["profile", "repack", "--replace", "not-a-file", "-x"],
    ] {
        assert_eq!(
            bootstrap::inspect_cli_args(
                arguments.into_iter().map(OsString::from)
            )
            .expect_err("an ambiguous option token cannot select a file")
            .kind(),
            io::ErrorKind::InvalidInput,
        );
    }
}

#[cfg(target_os = "linux")]
#[test]
fn profile_cli_refuses_bare_option_source_without_private_inspection() {
    let section = br#"{"private":"bare-option-section-8712"}"#;
    let asset = b"bare-option-asset-4592";
    let original = OwnedProfileFile::new(&profile_zip(
        section, asset, section,
    ));
    let folder = OwnedProfileDirectory::new(original.0.with_extension(
        "option-source-folder",
    ));
    let source = OwnedProfileDestination(folder.0.join("--private"));
    fs::write(&source.0, fs::read(&original.0).expect("owned fixture"))
        .expect("copy test-owned bytes to option-like selected name");
    for operation in ["inspect", "verify"] {
        let rejected = Command::new(env!("CARGO_BIN_EXE_atrament"))
            .current_dir(&folder.0)
            .args(["profile", operation, "--private"])
            .output()
            .expect("reject bare source flag before private profile read");
        assert!(!rejected.status.success());
        assert!(rejected.stdout.is_empty());
        let stderr = String::from_utf8_lossy(&rejected.stderr);
        assert!(stderr.contains("usage:"));
        for private in ["bare-option-section-8712", "bare-option-asset-4592"] {
            assert!(!stderr.contains(private));
        }

        let selected = Command::new(env!("CARGO_BIN_EXE_atrament"))
            .current_dir(&folder.0)
            .args(["profile", operation, "./--private"])
            .output()
            .expect("explicit relative path selects the exact profile");
        assert!(selected.status.success());
        assert!(selected.stderr.is_empty());
        if operation == "inspect" {
            let readable = String::from_utf8_lossy(&selected.stdout);
            assert!(readable.contains("bare-option-section-8712"));
            assert!(!readable.contains("bare-option-asset-4592"));
        } else {
            assert_eq!(
                selected.stdout,
                b"profile verified (no session import)\n",
            );
        }
    }
    assert_eq!(
        fs::read(&source.0).expect("no CLI operation alters source"),
        fs::read(&original.0).expect("original source stays unchanged"),
    );
}

#[cfg(target_os = "linux")]
#[test]
fn repack_cli_requires_unambiguous_path_for_option_named_target() {
    let section = br#"{"safe":"literal-option-target"}"#;
    let source = OwnedProfileFile::new(&profile_zip(
        section, b"private-opaque-option-9013", section,
    ));
    let folder = OwnedProfileDirectory::new(source.0.with_extension(
        "option-destination-folder",
    ));
    let target = OwnedProfileDestination(folder.0.join("--replace"));
    for option in ["--replace", "--replcae", "-x"] {
        let rejected = Command::new(env!("CARGO_BIN_EXE_atrament"))
            .current_dir(&folder.0)
            .args(["profile", "repack"])
            .arg(&source.0)
            .arg(option)
            .output()
            .expect("run misplaced or unknown bare option");
        assert!(!rejected.status.success());
        assert!(rejected.stdout.is_empty());
        let stderr = String::from_utf8_lossy(&rejected.stderr);
        assert!(stderr.contains("usage:"));
        assert!(!folder.0.join(option).exists(),
            "bare option must not create a file");
    }
    assert!(!target.0.exists());

    let accepted = Command::new(env!("CARGO_BIN_EXE_atrament"))
        .current_dir(&folder.0)
        .args(["profile", "repack"])
        .arg(&source.0)
        .arg("./--replace")
        .output()
        .expect("run explicitly disambiguated destination");
    assert!(accepted.status.success());
    assert_eq!(
        accepted.stdout,
        b"profile repacked (verified; no session import)\n",
    );
    assert!(accepted.stderr.is_empty());
    let repacked = fs::read(&target.0).expect("literal option target exists");
    let archive = read_stored_profile_zip(
        &repacked,
        ProfileZipReadLimits {
            maximum_archive_bytes: 4096,
            maximum_entries: 3,
            maximum_entry_bytes: 2048,
            maximum_total_bytes: 3000,
        },
    )
    .expect("literal target contains a complete verified ZIP");
    assert_eq!(archive.entries().len(), 3);
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

#[test]
fn repack_cli_rejects_extra_argument_before_any_source_open() {
    let pulls = Cell::new(0usize);
    let source = ["profile", "repack", "not-a-file", "not-a-target"]
        .into_iter().map(OsString::from)
        .chain(iter::from_fn(|| {
            pulls.set(pulls.get() + 1usize);
            Some(OsString::from("unexpected extra argument"))
        }));
    assert_eq!(
        bootstrap::inspect_cli_args(source)
            .expect_err("extra argument is not an authorized repack")
            .kind(),
        io::ErrorKind::InvalidInput,
    );
    assert_eq!(pulls.get(), 1usize, "at most fifth item inspected");
}

#[test]
fn repack_replace_cli_rejects_extra_argument_before_any_source_open() {
    let pulls = Cell::new(0usize);
    let source = [
        "profile", "repack", "--replace", "not-a-file", "not-a-target",
    ]
    .into_iter().map(OsString::from)
    .chain(iter::from_fn(|| {
        pulls.set(pulls.get() + 1usize);
        Some(OsString::from("unexpected extra argument"))
    }));
    assert_eq!(
        bootstrap::inspect_cli_args(source)
            .expect_err("extra argument cannot authorize replacement")
            .kind(),
        io::ErrorKind::InvalidInput,
    );
    assert_eq!(pulls.get(), 1usize, "at most sixth item inspected");
}

#[cfg(target_os = "linux")]
#[test]
fn cli_verify_reports_only_fixed_success_after_full_source_admission() {
    let section = br#"{"private":"section-secret-65018"}"#;
    let asset = b"\xff\0opaque-private-24915";
    let source = OwnedProfileFile::new(&profile_zip_with_optional_features(
        section, asset, section, "application/json",
        r#"["private-optional-feature-72281"]"#,
    ));
    bootstrap::verify_profile(source.0.clone())
        .expect("valid profile can be verified without inspection");
    let executed = Command::new(env!("CARGO_BIN_EXE_atrament"))
        .args(["profile", "verify"])
        .arg(&source.0)
        .output()
        .expect("run private selected-profile verification");
    assert!(executed.status.success());
    assert_eq!(
        executed.stdout,
        b"profile verified (no session import)\n",
    );
    assert!(executed.stderr.is_empty());
    assert_eq!(fs::read(&source.0).expect("no mutation"),
        profile_zip_with_optional_features(
            section, asset, section, "application/json",
            r#"["private-optional-feature-72281"]"#,
        ));
    for private in [
        "section-secret-65018",
        "opaque-private-24915",
        "private-optional-feature-72281",
        "sections/writer.json",
        "assets/raw.bin",
        "writer",
    ] {
        assert!(!String::from_utf8_lossy(&executed.stdout).contains(private));
    }
}

#[cfg(target_os = "linux")]
#[test]
fn cli_verify_refuses_invalid_source_before_any_stdout() {
    let authentic = br#"{"private":"profile-secret-9638"}"#;
    let forged = br#"{"private":"profile-secret-9368"}"#;
    assert_eq!(authentic.len(), forged.len());
    let invalid_json = b"[private-invalid-section-1147]";
    let sources = [
        (profile_zip(forged, b"private-asset-6419", authentic),
            "DigestMismatch"),
        (profile_zip(invalid_json, b"private-asset-6419", invalid_json),
            "SectionJson"),
    ];
    for (source_bytes, diagnostic) in sources {
        let source = OwnedProfileFile::new(&source_bytes);
        let error = bootstrap::verify_profile(source.0.clone())
            .expect_err("invalid profile is not verified");
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(error.to_string().contains(diagnostic));
        let executed = Command::new(env!("CARGO_BIN_EXE_atrament"))
            .args(["profile", "verify"])
            .arg(&source.0)
            .output()
            .expect("run rejected private source verification");
        assert!(!executed.status.success());
        assert!(executed.stdout.is_empty());
        let stderr = String::from_utf8_lossy(&executed.stderr);
        assert!(stderr.contains(diagnostic));
        for private in [
            "profile-secret-9638",
            "profile-secret-9368",
            "private-invalid-section-1147",
            "private-asset-6419",
        ] {
            assert!(!stderr.contains(private));
        }
        assert_eq!(fs::read(&source.0).expect("read-only verification"),
            source_bytes);
    }
}

#[cfg(target_os = "linux")]
#[test]
fn cli_verify_refuses_symlinked_selected_source_without_path_leak() {
    use std::os::unix::fs::symlink;
    let section = br#"{"private":"source-59018"}"#;
    let source = OwnedProfileFile::new(&profile_zip(
        section, b"opaque-60228", section,
    ));
    let link = OwnedProfileDestination(
        source.0.with_extension("private-link-2190"),
    );
    symlink(&source.0, &link.0).expect("create test-owned selection link");
    let executed = Command::new(env!("CARGO_BIN_EXE_atrament"))
        .args(["profile", "verify"])
        .arg(&link.0)
        .output()
        .expect("verify rejects source symlink");
    assert!(!executed.status.success());
    assert!(executed.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&executed.stderr);
    assert!(stderr.contains("Open("));
    assert!(!stderr.contains("private-link-2190"));
    assert!(!stderr.contains("source-59018"));
    assert!(!stderr.contains("opaque-60228"));
    assert!(link.0.is_symlink());
}

#[cfg(target_os = "linux")]
#[test]
fn cli_check_canonical_distinguishes_integrity_from_canonical_bytes() {
    let section = br#"{"private":"canonical-only-49352"}"#;
    let asset = b"private-opaque-canonical-49252\xff";
    let original = profile_zip(section, asset, section);
    let source = OwnedProfileFile::new(&original);
    let initial = fs::metadata(&source.0).expect("original source metadata");

    // ZIP metadata is already deterministic, but manifest keys are not.
    bootstrap::verify_profile(source.0.clone())
        .expect("noncanonical source still has valid CRC and SHA-256");
    let rejected = Command::new(env!("CARGO_BIN_EXE_atrament"))
        .args(["profile", "check-canonical"])
        .arg(&source.0)
        .output()
        .expect("check noncanonical verified source");
    assert!(!rejected.status.success());
    assert!(rejected.stdout.is_empty());
    let error = String::from_utf8_lossy(&rejected.stderr);
    assert!(error.contains(
        "verified but not canonically encoded: manifest JSON",
    ));
    assert!(!error.contains("canonical-only-49352"));
    assert!(!error.contains("private-opaque-canonical-49252"));
    assert_eq!(fs::read(&source.0).expect("unchanged source"), original);
    assert_eq!(
        fs::metadata(&source.0).expect("still original")
            .modified().expect("unchanged modification time"),
        initial.modified().expect("initial modification time"),
    );

    let target = OwnedProfileDestination(
        source.0.with_extension("canonical-check.atrament"),
    );
    bootstrap::repack_profile(
        source.0.clone(), target.0.clone(),
        ExplicitOverwriteIntent::PreserveExisting,
    ).expect("verified noncanonical profile can be repacked");
    let expected = fs::read(&target.0).expect("canonical output exists");
    assert_ne!(expected, original, "root canonicalization changed bytes");
    let canonical = Command::new(env!("CARGO_BIN_EXE_atrament"))
        .args(["profile", "check-canonical"])
        .arg(&target.0)
        .output()
        .expect("check complete canonical profile");
    assert!(canonical.status.success());
    assert_eq!(
        canonical.stdout,
        b"profile canonical (verified; no session import)\n",
    );
    assert!(canonical.stderr.is_empty());
    assert_eq!(fs::read(&target.0).expect("check is read-only"), expected);
}

#[cfg(target_os = "linux")]
#[test]
fn cli_check_canonical_never_publishes_forged_source_bytes() {
    let authentic = br#"{"private":"canonical-integrity-70165"}"#;
    let forged = br#"{"private":"canonical-integrity-70166"}"#;
    assert_eq!(authentic.len(), forged.len());
    let bytes = profile_zip(
        forged, b"opaque-integrity-70165", authentic,
    );
    let source = OwnedProfileFile::new(&bytes);
    let rejected = Command::new(env!("CARGO_BIN_EXE_atrament"))
        .args(["profile", "check-canonical"])
        .arg(&source.0)
        .output()
        .expect("reject digest-forged source before canonicality check");
    assert!(!rejected.status.success());
    assert!(rejected.stdout.is_empty());
    let diagnostic = String::from_utf8_lossy(&rejected.stderr);
    assert!(diagnostic.contains("DigestMismatch"));
    for private in [
        "canonical-integrity-70165",
        "canonical-integrity-70166",
        "opaque-integrity-70165",
    ] {
        assert!(!diagnostic.contains(private));
    }
    assert_eq!(
        fs::read(&source.0).expect("canonicality cannot alter forged source"),
        bytes,
    );
}

#[cfg(target_os = "linux")]
#[test]
fn cli_check_canonical_rejects_legacy_zip_metadata_even_with_canonical_root() {
    let section = br#"{"private":"legacy-writer-metadata-51235"}"#;
    let initial = OwnedProfileFile::new(&profile_zip(
        section, b"private-asset-51235", section,
    ));
    let canonical_path = OwnedProfileDestination(
        initial.0.with_extension("metadata-canonical.atrament"),
    );
    bootstrap::repack_profile(
        initial.0.clone(), canonical_path.0.clone(),
        ExplicitOverwriteIntent::PreserveExisting,
    ).expect("obtain independently verified canonical ZIP");
    let mut legacy = fs::read(&canonical_path.0)
        .expect("canonical writer ZIP");
    let trailer_offset = legacy.len().checked_sub(6)
        .expect("ZIP trailer directory offset is present");
    let directory_bytes = legacy.get(trailer_offset..trailer_offset + 4)
        .expect("ZIP directory offset bytes");
    let directory = usize::try_from(u32::from_le_bytes(
        directory_bytes.try_into().expect("four directory offset bytes"),
    )).expect("ZIP directory offset fits usize");
    legacy.get_mut(4..6).expect("first local version slot")
        .copy_from_slice(&10u16.to_le_bytes());
    legacy.get_mut(directory + 6..directory + 8)
        .expect("first central version slot")
        .copy_from_slice(&10u16.to_le_bytes());
    let source = OwnedProfileFile::new(&legacy);
    bootstrap::verify_profile(source.0.clone())
        .expect("legacy metadata does not invalidate the verified archive");
    let result = Command::new(env!("CARGO_BIN_EXE_atrament"))
        .args(["profile", "check-canonical"])
        .arg(&source.0)
        .output()
        .expect("check legacy header framing");
    assert!(!result.status.success());
    assert!(result.stdout.is_empty());
    let error = String::from_utf8_lossy(&result.stderr);
    assert!(error.contains(
        "verified but not canonically encoded: ZIP framing",
    ));
    assert!(!error.contains("legacy-writer-metadata-51235"));
    assert!(!error.contains("private-asset-51235"));
    assert_eq!(fs::read(&source.0).expect("source remains unchanged"), legacy);
}

#[cfg(target_os = "linux")]
#[test]
fn cli_repack_handles_root_only_profile_without_phantom_members() {
    let root = concat!(
        r#"{"required_features":[],"profile_identity":"empty-writer", "#,
        r#""optional_features":[],"entries":[],"#,
        r#""container_version":"atrament.profile/1"}"#,
    );
    let original = write_stored_profile_zip(
        &[ProfileStoredZipMember {
            name: "manifest.json", bytes: root.as_bytes(),
        }],
        ProfileStoredZipWriteLimits {
            maximum_archive_bytes: 4096,
            maximum_entries: 1,
            maximum_entry_bytes: 2048,
            maximum_total_bytes: 3000,
        },
    ).expect("root-only input is valid ZIP");
    let source = OwnedProfileFile::new(&original);
    let verified = Command::new(env!("CARGO_BIN_EXE_atrament"))
        .args(["profile", "verify"])
        .arg(&source.0)
        .output()
        .expect("root-only profile verification");
    assert!(verified.status.success());
    assert_eq!(verified.stdout, b"profile verified (no session import)\n");
    assert!(verified.stderr.is_empty());
    let destination = OwnedProfileDestination(
        source.0.with_extension("empty-repack.atrament"),
    );
    let executed = Command::new(env!("CARGO_BIN_EXE_atrament"))
        .args(["profile", "repack"])
        .arg(&source.0)
        .arg(&destination.0)
        .output()
        .expect("root-only selected source repack");
    assert!(executed.status.success());
    assert!(executed.stderr.is_empty());
    let bytes = fs::read(&destination.0).expect("root-only canonical ZIP");
    let archive = read_stored_profile_zip(
        &bytes,
        ProfileZipReadLimits {
            maximum_archive_bytes: 4096,
            maximum_entries: 1,
            maximum_entry_bytes: 2048,
            maximum_total_bytes: 3000,
        },
    ).expect("repacked root-only ZIP independently parses");
    assert_eq!(archive.entries().len(), 1);
    assert_eq!(
        archive.entries().first().expect("root manifest member").name(),
        "manifest.json",
    );
    assert!(archive.matches_stored_writer_framing());
    let manifest = archive.manifest_utf8(4096).expect("canonical root JSON");
    assert!(manifest.starts_with(
        r#"{"container_version":"atrament.profile/1","entries":[]"#,
    ));
    assert!(manifest.contains(r#""profile_identity":"empty-writer""#));
    let inspection = bootstrap::inspect_profile(destination.0.clone())
        .expect("root-only export independently reimports");
    assert!(inspection.contains("empty-writer"));
    assert!(!inspection.contains("sections/"));
    bootstrap::check_canonical_profile(destination.0.clone())
        .expect("repacked empty profile is exactly canonical");
}

#[cfg(target_os = "linux")]
#[test]
fn cli_repack_rejects_selected_source_symlink_before_target_creation() {
    use std::os::unix::fs::symlink;
    let section = br#"{"private":"must-not-leak-8326"}"#;
    let original = profile_zip(section, b"opaque-source-9511", section);
    let source = OwnedProfileFile::new(&original);
    let link = OwnedProfileDestination(
        source.0.with_extension("source-link.atrament"),
    );
    symlink(&source.0, &link.0).expect("untrusted link selected by caller");
    let destination = OwnedProfileDestination(
        source.0.with_extension("from-link.atrament"),
    );
    let executed = Command::new(env!("CARGO_BIN_EXE_atrament"))
        .args(["profile", "repack"])
        .arg(&link.0)
        .arg(&destination.0)
        .output()
        .expect("attempt to repack source through disallowed symlink");
    assert!(!executed.status.success());
    assert!(executed.stdout.is_empty());
    assert!(!destination.0.exists());
    assert_eq!(fs::read(&source.0).expect("original untouched"), original);
    let stderr = String::from_utf8_lossy(&executed.stderr);
    assert!(stderr.contains("Open("));
    assert!(!stderr.contains("must-not-leak-8326"));
    assert!(!stderr.contains("opaque-source-9511"));
}

#[cfg(target_os = "linux")]
#[test]
fn cli_repack_refuses_same_source_and_target_without_in_place_rewrite() {
    let section = br#"{"private":"preserve-source-1179"}"#;
    let original = profile_zip(section, b"preserve-asset-3001", section);
    let source = OwnedProfileFile::new(&original);
    let executed = Command::new(env!("CARGO_BIN_EXE_atrament"))
        .args(["profile", "repack"])
        .arg(&source.0)
        .arg(&source.0)
        .output()
        .expect("self-repack without overwrite authorization");
    assert!(!executed.status.success());
    assert!(executed.stdout.is_empty());
    assert!(String::from_utf8_lossy(&executed.stderr).contains("TargetExists"));
    assert_eq!(fs::read(&source.0).expect("same file untouched"), original);
}

#[cfg(target_os = "linux")]
#[test]
fn cli_repack_hardlinked_target_requires_replace_without_mutating_source() {
    use std::os::unix::fs::MetadataExt as _;

    let section = br#"{"verified":"hardlinked-source-6108"}"#;
    let original = profile_zip(section, b"private-hardlink-asset", section);
    let source = OwnedProfileFile::new(&original);
    let destination = OwnedProfileDestination(
        source.0.with_extension("hardlink-target.atrament"),
    );
    fs::hard_link(&source.0, &destination.0)
        .expect("test selected target aliases the original inode");
    let source_inode = fs::metadata(&source.0).expect("source inode").ino();
    assert_eq!(
        fs::metadata(&destination.0).expect("linked inode").ino(),
        source_inode,
    );
    let refused = Command::new(env!("CARGO_BIN_EXE_atrament"))
        .args(["profile", "repack"])
        .arg(&source.0)
        .arg(&destination.0)
        .output()
        .expect("try to repack into hardlinked existing target");
    assert!(!refused.status.success());
    assert!(refused.stdout.is_empty());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("TargetExists"));
    assert_eq!(fs::read(&source.0).expect("source preserved"), original);
    assert_eq!(fs::read(&destination.0).expect("alias preserved"), original);
    assert_eq!(
        fs::metadata(&destination.0).expect("same linked inode").ino(),
        source_inode,
    );

    let replaced = Command::new(env!("CARGO_BIN_EXE_atrament"))
        .args(["profile", "repack", "--replace"])
        .arg(&source.0)
        .arg(&destination.0)
        .output()
        .expect("explicitly replace one hardlink destination");
    assert!(replaced.status.success());
    assert!(replaced.stderr.is_empty());
    assert_eq!(
        replaced.stdout,
        b"profile repacked (verified; no session import)\n",
    );
    let canonical = fs::read(&destination.0).expect("new canonical ZIP");
    assert_ne!(canonical, original, "noncanonical original is untouched");
    assert_eq!(fs::read(&source.0).expect("linked source intact"), original);
    assert_eq!(
        fs::metadata(&source.0).expect("source inode").ino(),
        source_inode,
    );
    assert_ne!(
        fs::metadata(&destination.0).expect("new selected inode").ino(),
        source_inode,
    );
    let inspection = bootstrap::inspect_profile(destination.0.clone())
        .expect("replaced alias independently verifies");
    assert!(inspection.contains("hardlinked-source-6108"));
    assert!(!inspection.contains("private-hardlink-asset"));
}

#[cfg(target_os = "linux")]
#[test]
fn cli_repack_canonically_preserves_verified_sections_assets_and_features() {
    use std::os::unix::fs::PermissionsExt as _;

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
    let asset_entry = entries.first().expect("asset member");
    let manifest_entry = entries.get(1).expect("root manifest member");
    let section_entry = entries.get(2).expect("writer section member");
    assert_eq!(asset_entry.name(), "assets/raw.bin");
    assert_eq!(asset_entry.bytes(), asset);
    assert_eq!(manifest_entry.name(), "manifest.json");
    assert_eq!(section_entry.name(), "sections/writer.json");
    assert_eq!(section_entry.bytes(), section);
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

    // Canonical bytes must be stable when a verified output is repacked again.
    let repeat = OwnedProfileDestination(
        source.0.with_extension("canonical-repeat.atrament"),
    );
    let repeated = Command::new(env!("CARGO_BIN_EXE_atrament"))
        .args(["profile", "repack"])
        .arg(&destination.0)
        .arg(&repeat.0)
        .output()
        .expect("repack already-canonical output");
    assert!(repeated.status.success());
    assert!(repeated.stderr.is_empty());
    assert_eq!(
        fs::read(&repeat.0).expect("canonical repeated output"),
        repacked,
    );
    assert_eq!(
        fs::metadata(&destination.0)
            .expect("persisted output metadata")
            .permissions().mode() & 0o777,
        0o600,
        "new output should be private to the owning process user",
    );
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
fn cli_repack_replace_preserves_raw_non_utf8_target_in_both_states() {
    use std::os::unix::ffi::{OsStrExt as _, OsStringExt as _};

    let section = br#"{"verified":"raw-output-selection-9631"}"#;
    let source = OwnedProfileFile::new(&profile_zip(
        section, b"raw-destination-asset-3015", section,
    ));
    let mut name = source.0.file_name()
        .expect("test-owned source name").as_bytes().to_vec();
    name.extend_from_slice(b"-replace-");
    name.push(0xff);
    name.extend_from_slice(b".atrament");
    let destination = OwnedProfileDestination(
        source.0.with_file_name(OsString::from_vec(name)),
    );
    assert!(!destination.0.exists());
    let mut first = None;
    for replace_existing in [false, true] {
        if replace_existing {
            fs::write(&destination.0, b"old target bytes must be replaced")
                .expect("explicit replacement target exists");
        }
        let output = Command::new(env!("CARGO_BIN_EXE_atrament"))
            .args(["profile", "repack", "--replace"])
            .arg(&source.0)
            .arg(&destination.0)
            .output()
            .expect("selected non-UTF8 destination is passed as OS argv");
        assert!(output.status.success());
        assert_eq!(
            output.stdout,
            b"profile repacked (verified; no session import)\n",
        );
        assert!(output.stderr.is_empty());
        let canonical = fs::read(&destination.0)
            .expect("complete canonical output selected by raw path");
        if let Some(previous) = &first {
            assert_eq!(&canonical, previous, "stable canonical output bytes");
        } else {
            first = Some(canonical);
        }
        let verified = bootstrap::inspect_profile(destination.0.clone())
            .expect("raw-byte target can be selected for secure import");
        assert!(verified.contains("raw-output-selection-9631"));
        assert!(!verified.contains("raw-destination-asset-3015"));
    }
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
fn cli_repack_requires_explicit_replace_to_update_existing_target() {
    use std::os::unix::fs::PermissionsExt as _;

    let section = br#"{"verified":"new-section-content-1633"}"#;
    let asset = b"new-opaque-asset-content-1105";
    let original = profile_zip(section, asset, section);
    let source = OwnedProfileFile::new(&original);
    let destination = OwnedProfileDestination(
        source.0.with_extension("explicit-replace.atrament"),
    );
    fs::write(&destination.0, b"older profile artifact")
        .expect("existing selected destination");
    let executed = Command::new(env!("CARGO_BIN_EXE_atrament"))
        .args(["profile", "repack", "--replace"])
        .arg(&source.0)
        .arg(&destination.0)
        .output()
        .expect("explicitly replace selected output");
    assert!(executed.status.success());
    assert_eq!(
        executed.stdout,
        b"profile repacked (verified; no session import)\n",
    );
    assert!(executed.stderr.is_empty());
    let rewritten = fs::read(&destination.0).expect("complete replacement");
    assert_ne!(rewritten, b"older profile artifact");
    assert_eq!(fs::read(&source.0).expect("source unchanged"), original);
    let archived = read_stored_profile_zip(
        &rewritten,
        ProfileZipReadLimits {
            maximum_archive_bytes: 4096,
            maximum_entries: 3,
            maximum_entry_bytes: 2048,
            maximum_total_bytes: 3000,
        },
    ).expect("complete canonical output");
    assert!(archived.matches_stored_writer_framing());
    let inspected = bootstrap::inspect_profile(destination.0.clone())
        .expect("replaced file re-verifies independently");
    assert!(inspected.contains("new-section-content-1633"));
    assert!(!inspected.contains("new-opaque-asset-content-1105"));
    assert_eq!(
        fs::metadata(&destination.0).expect("new metadata")
            .permissions().mode() & 0o777,
        0o600,
    );
}

#[cfg(target_os = "linux")]
#[test]
fn cli_repack_replace_can_canonicalize_the_selected_source_in_place() {
    let section = br#"{"verified":"in-place-7591"}"#;
    let original = profile_zip(section, b"private-asset-1107", section);
    let source = OwnedProfileFile::new(&original);
    let executed = Command::new(env!("CARGO_BIN_EXE_atrament"))
        .args(["profile", "repack", "--replace"])
        .arg(&source.0)
        .arg(&source.0)
        .output()
        .expect("explicit in-place repack");
    assert!(executed.status.success());
    assert!(executed.stderr.is_empty());
    assert_eq!(
        executed.stdout,
        b"profile repacked (verified; no session import)\n",
    );
    let canonical = fs::read(&source.0).expect("canonical source replacement");
    assert_ne!(canonical, original, "canonical ZIP bytes replace source");
    let inspected = bootstrap::inspect_profile(source.0.clone())
        .expect("self-repacked profile remains independently verifiable");
    assert!(inspected.contains("in-place-7591"));
    assert!(!inspected.contains("private-asset-1107"));
}

#[cfg(target_os = "linux")]
#[test]
fn cli_repack_replace_refuses_invalid_source_without_touching_target() {
    let actual = br#"{"private":"wrong-content-3275"}"#;
    let claimed = br#"{"private":"other-content-2841"}"#;
    assert_eq!(actual.len(), claimed.len());
    let source = OwnedProfileFile::new(&profile_zip(
        actual, b"private-asset-5938", claimed,
    ));
    let destination = OwnedProfileDestination(
        source.0.with_extension("invalid-replace.atrament"),
    );
    let previous = b"existing output must stay byte-exact";
    fs::write(&destination.0, previous).expect("selected existing output");
    let executed = Command::new(env!("CARGO_BIN_EXE_atrament"))
        .args(["profile", "repack", "--replace"])
        .arg(&source.0)
        .arg(&destination.0)
        .output()
        .expect("attempt invalid replace source");
    assert!(!executed.status.success());
    assert!(executed.stdout.is_empty());
    assert_eq!(fs::read(&destination.0).expect("target survived"), previous);
    let stderr = String::from_utf8_lossy(&executed.stderr);
    assert!(stderr.contains("DigestMismatch"));
    for private in [
        "wrong-content-3275", "other-content-2841", "private-asset-5938",
        "invalid-replace.atrament",
    ] {
        assert!(!stderr.contains(private));
    }
}

#[cfg(target_os = "linux")]
#[test]
fn cli_repack_replace_refuses_directory_target_without_modification() {
    let section = br#"{"secret":"directory-refusal-5915"}"#;
    let source = OwnedProfileFile::new(&profile_zip(
        section, b"opaque-directory-9343", section,
    ));
    let target = OwnedProfileDirectory::new(
        source.0.with_extension("existing-directory"),
    );
    let output = Command::new(env!("CARGO_BIN_EXE_atrament"))
        .args(["profile", "repack", "--replace"])
        .arg(&source.0)
        .arg(&target.0)
        .output()
        .expect("attempt directory replacement");
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("InvalidExistingTarget"));
    assert!(!stderr.contains("directory-refusal-5915"));
    assert!(!stderr.contains("opaque-directory-9343"));
    assert!(target.0.is_dir());
    assert_eq!(
        fs::read_dir(&target.0).expect("unchanged directory").count(),
        0,
    );
}

#[cfg(target_os = "linux")]
#[test]
fn cli_repack_replace_refuses_symlinked_parent_before_writing() {
    use std::os::unix::fs::symlink;

    let section = br#"{"secret":"ancestor-refusal-7586"}"#;
    let source = OwnedProfileFile::new(&profile_zip(
        section, b"opaque-ancestor-2327", section,
    ));
    let actual_directory = OwnedProfileDirectory::new(
        source.0.with_extension("real-parent"),
    );
    let alias = OwnedProfileDestination(
        source.0.with_extension("linked-parent"),
    );
    symlink(&actual_directory.0, &alias.0)
        .expect("link to test-owned selected parent");
    let target = alias.0.join("new-profile.atrament");
    let output = Command::new(env!("CARGO_BIN_EXE_atrament"))
        .args(["profile", "repack", "--replace"])
        .arg(&source.0)
        .arg(&target)
        .output()
        .expect("attempt forbidden symlink ancestor");
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("profile repack target refused"));
    assert!(!stderr.contains("ancestor-refusal-7586"));
    assert!(!stderr.contains("opaque-ancestor-2327"));
    assert!(!target.exists());
    assert!(actual_directory.0.is_dir());
    assert_eq!(
        fs::read_dir(&actual_directory.0)
            .expect("linked directory remains untouched").count(),
        0,
    );
    assert_eq!(
        fs::read_link(&alias.0).expect("ancestor remains a symlink"),
        actual_directory.0,
    );
}

#[cfg(target_os = "linux")]
#[test]
fn cli_repack_replace_refuses_symlink_target_without_following_victim() {
    use std::os::unix::fs::symlink;

    let section = br#"{"private":"replacement-secret-3769"}"#;
    let source = OwnedProfileFile::new(&profile_zip(
        section, b"private-asset-3379", section,
    ));
    let victim = OwnedProfileDestination(
        source.0.with_extension("replace-victim.atrament"),
    );
    fs::write(&victim.0, b"victim original")
        .expect("test-owned unrelated victim");
    let destination = OwnedProfileDestination(
        source.0.with_extension("replace-link.atrament"),
    );
    symlink(&victim.0, &destination.0).expect("test-owned link");
    let executed = Command::new(env!("CARGO_BIN_EXE_atrament"))
        .args(["profile", "repack", "--replace"])
        .arg(&source.0)
        .arg(&destination.0)
        .output()
        .expect("replace request must refuse linked target");
    assert!(!executed.status.success());
    assert!(executed.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&executed.stderr);
    assert!(stderr.contains("InvalidExistingTarget"));
    assert!(!stderr.contains("replacement-secret-3769"));
    assert!(!stderr.contains("private-asset-3379"));
    assert_eq!(
        fs::read(&victim.0).expect("victim unchanged"),
        b"victim original",
    );
    assert_eq!(
        fs::read_link(&destination.0).expect("link unchanged"),
        victim.0,
    );
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
fn cli_inspection_preserves_exact_large_section_after_output_planning() {
    // Near-2 MiB Unicode sections expand to either ~3.8 MiB of BMP
    // escapes or ~5.8 MiB of surrogate-pair escapes. Every output
    // reservation must count the complete display before any append.
    for section in [
        format!(r#"{{"large":"{}"}}"#, "v".repeat(130_000)),
        format!(r#"{{"large":"{}"}}"#, "\u{202e}".repeat(640_000)),
        format!(r#"{{"large":"{}"}}"#, "\u{e0100}".repeat(480_000)),
    ] {
        let digest = digest_hex(section.as_bytes());
        let source_root = format!(
            concat!(
                r#"{{"profile_identity":"writer","required_features":[],"#,
                r#""optional_features":[],"entries":[{{"#,
                r#""path":"sections/long.json","#,
                r#""sha256":"{}","byte_length":{},"#,
                r#""media_type":"application/json"}}],"#,
                r#""container_version":"atrament.profile/1"}}"#,
            ),
            digest, section.len(),
        );
        let canonical_root = format!(
            concat!(
                r#"{{"container_version":"atrament.profile/1","#,
                r#""entries":[{{"byte_length":{},"#,
                r#""media_type":"application/json","#,
                r#""path":"sections/long.json","sha256":"{}"}}],"#,
                r#""optional_features":[],"profile_identity":"writer","#,
                r#""required_features":[]}}"#,
            ),
            section.len(), digest,
        );
        let archive = write_stored_profile_zip(
            &[
                ProfileStoredZipMember {
                    name: "manifest.json",
                    bytes: source_root.as_bytes(),
                },
                ProfileStoredZipMember {
                    name: "sections/long.json",
                    bytes: section.as_bytes(),
                },
            ],
            ProfileStoredZipWriteLimits {
                maximum_archive_bytes: 4 * 1024 * 1024,
                maximum_entries: 2,
                maximum_entry_bytes: 2 * 1024 * 1024,
                maximum_total_bytes: 4 * 1024 * 1024,
            },
        )
        .expect("long JSON section stays inside bounded profile size");
        let owned = OwnedProfileFile::new(&archive);
        let printable_section = section
            .replace('\u{202e}', r"\u202e")
            .replace('\u{e0100}', r"\udb40\udd00");
        let expected = format!(
            "manifest.json (verified):\n{canonical_root}\nsections/long.json \
             (syntax-checked; untyped):\n{printable_section}\n",
        );
        if section.contains('\u{202e}') {
            assert!(
                expected.len() > 3_800_000 && expected.len() > archive.len(),
                "escaped terminal display must expand beyond source bytes",
            );
        }
        if section.contains('\u{e0100}') {
            assert!(
                expected.len() > 5_700_000 && expected.len() > archive.len(),
                "supplementary terminal escapes triple raw UTF-8 length",
            );
        }
        let readable = bootstrap::inspect_profile(owned.0.clone())
            .expect("all reserved output bytes remain intact");
        assert_eq!(readable, expected);
        let executed = Command::new(env!("CARGO_BIN_EXE_atrament"))
            .args(["profile", "inspect"])
            .arg(&owned.0)
            .output()
            .expect("inspect one large verified section in a real CLI");
        assert!(executed.status.success());
        assert!(executed.stderr.is_empty());
        assert_eq!(executed.stdout, expected.as_bytes());
        assert_eq!(fs::read(&owned.0).expect("source not modified"), archive);
        assert!(!readable.contains('\u{202e}'), "no raw terminal controls");
        assert!(!readable.contains('\u{e0100}'), "no raw invisible tags");
    }
}

#[cfg(target_os = "linux")]
#[test]
fn cli_inspection_escapes_bidi_controls_in_verified_json_and_names() {
    let section_path = format!("sections/rtl{}name.json", '\u{200f}');
    let bidi_controls = [
        '\u{061c}', '\u{200e}', '\u{200f}', '\u{202a}',
        '\u{202b}', '\u{202c}', '\u{202d}', '\u{202e}',
        '\u{2066}', '\u{2067}', '\u{2068}', '\u{2069}',
    ];
    let all_bidi = bidi_controls.into_iter().collect::<String>();
    let section = format!(
        r#"{{"normal":"café","spoof":"left{}right{}end","all":"{}"}}"#,
        '\u{202e}', '\u{2069}', all_bidi,
    );
    let root = format!(
        concat!(
            r#"{{"container_version":"atrament.profile/1","#,
            r#""profile_identity":"writer{}verified","#,
            r#""required_features":[],"optional_features":[],"entries":["#,
            r#"{{"path":"{}","media_type":"application/json","#,
            r#""byte_length":{},"sha256":"{}"}}]}}"#,
        ),
        '\u{2066}',
        section_path,
        section.len(),
        digest_hex(section.as_bytes()),
    );
    let archive = write_stored_profile_zip(
        &[
            ProfileStoredZipMember {
                name: "manifest.json", bytes: root.as_bytes(),
            },
            ProfileStoredZipMember {
                name: &section_path, bytes: section.as_bytes(),
            },
        ],
        ProfileStoredZipWriteLimits {
            maximum_archive_bytes: 4096,
            maximum_entries: 2,
            maximum_entry_bytes: 2048,
            maximum_total_bytes: 3000,
        },
    ).expect("valid UTF-8 names and JSON may contain bidi controls");
    let source = OwnedProfileFile::new(&archive);
    bootstrap::verify_profile(source.0.clone())
        .expect("bidi controls are valid verified JSON content");

    let inspected = Command::new(env!("CARGO_BIN_EXE_atrament"))
        .args(["profile", "inspect"])
        .arg(&source.0)
        .output()
        .expect("inspect completely verified source");
    assert!(inspected.status.success());
    assert!(inspected.stderr.is_empty());
    let display = String::from_utf8(inspected.stdout)
        .expect("inspection output is UTF-8");
    assert!(display.contains(r#""profile_identity":"writer\u2066verified""#));
    assert!(display.contains(
        r"sections/rtl\u200fname.json (syntax-checked; untyped):",
    ));
    assert!(display.contains(r#""spoof":"left\u202eright\u2069end""#));
    assert!(display.contains("\"normal\":\"caf\u{e9}\""));
    let complete_bidi_set = concat!(
        r#""all":"\u061c\u200e\u200f\u202a\u202b\u202c"#,
        r#"\u202d\u202e\u2066\u2067\u2068\u2069""#,
    );
    assert!(display.contains(complete_bidi_set));
    for control in bidi_controls {
        assert!(!display.contains(control), "unescaped bidi control");
    }
    assert_eq!(fs::read(&source.0).expect("original survives"), archive);
}

#[cfg(target_os = "linux")]
#[test]
fn cli_inspection_escapes_c1_and_unicode_line_separators() {
    let section_path = format!(
        "sections/split{}{}page.json", '\u{2028}', '\u{2029}',
    );
    let section = format!(
        r#"{{"value":"left{}{}{}right"}}"#,
        '\u{007f}', '\u{0085}', '\u{2029}',
    );
    let root = format!(
        concat!(
            r#"{{"container_version":"atrament.profile/1","#,
            r#""profile_identity":"record{}owner","#,
            r#""required_features":[],"optional_features":[],"entries":["#,
            r#"{{"path":"{}","media_type":"application/json","#,
            r#""byte_length":{},"sha256":"{}"}}]}}"#,
        ),
        '\u{0085}',
        section_path,
        section.len(),
        digest_hex(section.as_bytes()),
    );
    let bytes = write_stored_profile_zip(
        &[
            ProfileStoredZipMember {
                name: "manifest.json", bytes: root.as_bytes(),
            },
            ProfileStoredZipMember {
                name: &section_path, bytes: section.as_bytes(),
            },
        ],
        ProfileStoredZipWriteLimits {
            maximum_archive_bytes: 4096,
            maximum_entries: 2,
            maximum_entry_bytes: 2048,
            maximum_total_bytes: 3000,
        },
    ).expect("source may contain valid raw JSON C1 and Unicode separators");
    let source = OwnedProfileFile::new(&bytes);
    bootstrap::verify_profile(source.0.clone())
        .expect("raw C1 and Unicode line separators remain valid source");
    let process = Command::new(env!("CARGO_BIN_EXE_atrament"))
        .args(["profile", "inspect"])
        .arg(&source.0)
        .output()
        .expect("print verified source with escaped terminal controls");
    assert!(process.status.success());
    assert!(process.stderr.is_empty());
    let display = String::from_utf8(process.stdout)
        .expect("inspection output is valid UTF-8");
    assert!(display.contains(r#""profile_identity":"record\u0085owner""#));
    assert!(display.contains(
        r"sections/split\u2028\u2029page.json (syntax-checked; untyped):",
    ));
    assert!(display.contains(
        r#""value":"left\u007f\u0085\u2029right""#,
    ));
    for separator in [
        '\u{007f}', '\u{0085}', '\u{2028}', '\u{2029}',
    ] {
        assert!(!display.contains(separator), "raw terminal separator");
    }
    assert_eq!(fs::read(&source.0).expect("read-only display"), bytes);
}

#[cfg(target_os = "linux")]
#[test]
fn cli_inspection_escapes_opaque_asset_metadata_without_revealing_bytes() {
    // These are distinct verified ZIP names, but the zero-width character
    // makes them look the same unless the inspection display escapes it.
    let asset_path = format!("assets/{}opaque.bin", '\u{2029}');
    let invisible_path = format!(
        "assets/{}opa{}{}que.bin", '\u{2029}', '\u{200b}',
        '\u{e0100}',
    );
    let opaque_asset = b"\x1b[31mprivate-opaque-91413\xff";
    let other_asset = b"hidden-private-asset-91414";
    let declaration = |path: &str, bytes: &[u8]| {
        format!(
            concat!(
                r#"{{"path":"{}","media_type":"application/octet-stream","#,
                r#""byte_length":{},"sha256":"{}"}}"#,
            ),
            path, bytes.len(), digest_hex(bytes),
        )
    };
    let entries = format!(
        "{},{}",
        declaration(&asset_path, opaque_asset),
        declaration(&invisible_path, other_asset),
    );
    let root = format!(
        concat!(
            r#"{{"container_version":"atrament.profile/1","#,
            r#""profile_identity":"asset{}only","required_features":[],"#,
            r#""optional_features":["opaque{}capability","hidden{}name","#,
            r#""mark{}name","bom{}name","tag{}hidden"],"entries":[{}]}}"#,
        ),
        '\u{034f}', '\u{009b}', '\u{2060}', '\u{00ad}',
        '\u{feff}', '\u{e0061}', entries,
    );
    let archive = write_stored_profile_zip(
        &[
            ProfileStoredZipMember {
                name: &asset_path, bytes: opaque_asset,
            },
            ProfileStoredZipMember {
                name: &invisible_path, bytes: other_asset,
            },
            ProfileStoredZipMember {
                name: "manifest.json", bytes: root.as_bytes(),
            },
        ],
        ProfileStoredZipWriteLimits {
            maximum_archive_bytes: 4096,
            maximum_entries: 3,
            maximum_entry_bytes: 2048,
            maximum_total_bytes: 3000,
        },
    ).expect("two distinct opaque asset names must stay distinct");
    let source = OwnedProfileFile::new(&archive);
    bootstrap::verify_profile(source.0.clone())
        .expect("full payload digest checks do not interpret opaque assets");
    let process = Command::new(env!("CARGO_BIN_EXE_atrament"))
        .args(["profile", "inspect"])
        .arg(&source.0)
        .output()
        .expect("inspect fully verified asset-only profile");
    assert!(process.status.success());
    assert!(process.stderr.is_empty());
    let text = String::from_utf8(process.stdout)
        .expect("terminal-safe UTF-8 display");
    assert!(text.starts_with("manifest.json (verified):\n"));
    assert!(text.contains(r#""path":"assets/\u2029opaque.bin""#));
    assert!(text.contains(
        r#""path":"assets/\u2029opa\u200b\udb40\udd00que.bin""#,
    ));
    for displayed in [
        r#""asset\u034fonly""#,
        r#""opaque\u009bcapability""#,
        r#""hidden\u2060name""#,
        r#""mark\u00adname""#,
        r#""bom\ufeffname""#,
        r#""tag\udb40\udc61hidden""#,
    ] {
        assert!(text.contains(displayed), "formatted metadata is visible");
    }
    assert!(!text.contains(" (syntax-checked; untyped):"));
    for opaque in ["private-opaque-91413", "hidden-private-asset-91414"] {
        assert!(!text.contains(opaque), "opaque bytes cannot leak");
    }
    for control in [
        '\u{2029}', '\u{009b}', '\u{001b}', '\u{034f}',
        '\u{200b}', '\u{2060}', '\u{00ad}', '\u{feff}',
        '\u{e0061}', '\u{e0100}',
    ] {
        assert!(
            !text.contains(control),
            "asset metadata cannot inject invisible terminal characters",
        );
    }
    assert_eq!(fs::read(&source.0).expect("input remains exact"), archive);
}

#[cfg(target_os = "linux")]
#[test]
fn cli_inspection_escapes_visually_blank_unicode_metadata() {
    let section_path = format!("sections/left{}right.json", '\u{115f}');
    let section = format!(
        r#"{{"value":"front{}middle{}back"}}"#,
        '\u{1160}', '\u{2800}',
    );
    let root = format!(
        concat!(
            r#"{{"container_version":"atrament.profile/1","#,
            r#""profile_identity":"pre{}post","required_features":[],"#,
            r#""optional_features":["cap{}ability"],"entries":[{{"#,
            r#""path":"{}","media_type":"application/json","#,
            r#""byte_length":{},"sha256":"{}"}}]}}"#,
        ),
        '\u{3164}', '\u{ffa0}', section_path, section.len(),
        digest_hex(section.as_bytes()),
    );
    let archive = write_stored_profile_zip(
        &[
            ProfileStoredZipMember {
                name: "manifest.json", bytes: root.as_bytes(),
            },
            ProfileStoredZipMember {
                name: &section_path, bytes: section.as_bytes(),
            },
        ],
        ProfileStoredZipWriteLimits {
            maximum_archive_bytes: 4096,
            maximum_entries: 2,
            maximum_entry_bytes: 2048,
            maximum_total_bytes: 3000,
        },
    ).expect("blank Unicode scalars remain valid profile metadata");
    let source = OwnedProfileFile::new(&archive);
    bootstrap::verify_profile(source.0.clone())
        .expect("verified source is unchanged by display policy");
    let process = Command::new(env!("CARGO_BIN_EXE_atrament"))
        .args(["profile", "inspect"])
        .arg(&source.0)
        .output()
        .expect("inspect exact verified profile in a real CLI");
    assert!(process.status.success());
    assert!(process.stderr.is_empty());
    let display = String::from_utf8(process.stdout)
        .expect("terminal display stays valid UTF-8");
    assert!(display.contains(r#""profile_identity":"pre\u3164post""#));
    assert!(display.contains(r#""optional_features":["cap\uffa0ability"]"#));
    assert!(display.contains(
        r"sections/left\u115fright.json (syntax-checked; untyped):",
    ));
    assert!(display.contains(
        r#""value":"front\u1160middle\u2800back""#,
    ));
    for blank in [
        '\u{115f}', '\u{1160}', '\u{2800}', '\u{3164}', '\u{ffa0}',
    ] {
        assert!(!display.contains(blank), "raw visually blank metadata");
    }
    assert_eq!(fs::read(&source.0).expect("source unchanged"), archive);
}

#[cfg(target_os = "linux")]
#[test]
fn cli_inspection_escapes_default_ignorable_unicode_ranges() {
    // Enumerate the full current Default_Ignorable_Code_Point range table,
    // including reserved supplementary scalars, not just range endpoints.
    let ranges: [(u32, u32); 17] = [
        (0x00ad, 0x00ad), (0x034f, 0x034f), (0x061c, 0x061c),
        (0x115f, 0x1160), (0x17b4, 0x17b5), (0x180b, 0x180f),
        (0x200b, 0x200f), (0x202a, 0x202e), (0x2060, 0x206f),
        (0x3164, 0x3164), (0xfe00, 0xfe0f), (0xfeff, 0xfeff),
        (0xffa0, 0xffa0), (0xfff0, 0xfff8),
        (0x1bca0, 0x1bca3), (0x1d173, 0x1d17a),
        (0xe0000, 0xe0fff),
    ];
    let invisible: String = ranges.into_iter()
        .flat_map(|(start, end)| (start..=end).filter_map(char::from_u32))
        .collect();
    assert_eq!(invisible.chars().count(), 4_174);
    let section = format!(r#"{{"special":"{invisible}"}}"#);
    let root = format!(
        concat!(
            r#"{{"container_version":"atrament.profile/1","#,
            r#""profile_identity":"writer","required_features":[],"#,
            r#""optional_features":[],"entries":[{{"#,
            r#""path":"sections/format.json","media_type":"application/json","#,
            r#""byte_length":{},"sha256":"{}"}}]}}"#,
        ),
        section.len(), digest_hex(section.as_bytes()),
    );
    let archive = write_stored_profile_zip(
        &[
            ProfileStoredZipMember {
                name: "manifest.json", bytes: root.as_bytes(),
            },
            ProfileStoredZipMember {
                name: "sections/format.json", bytes: section.as_bytes(),
            },
        ],
        ProfileStoredZipWriteLimits {
            maximum_archive_bytes: 64 * 1024,
            maximum_entries: 2,
            maximum_entry_bytes: 32 * 1024,
            maximum_total_bytes: 64 * 1024,
        },
    ).expect("valid JSON may contain ignorable Unicode scalars");
    let source = OwnedProfileFile::new(&archive);
    bootstrap::verify_profile(source.0.clone())
        .expect("verified bytes can contain default-ignorable scalars");
    let process = Command::new(env!("CARGO_BIN_EXE_atrament"))
        .args(["profile", "inspect"])
        .arg(&source.0)
        .output()
        .expect("inspect default-ignorable profile in a real CLI");
    assert!(process.status.success());
    assert!(process.stderr.is_empty());
    let display = String::from_utf8(process.stdout)
        .expect("terminal display remains UTF-8");
    // Independently encode exact UTF-16 code units to printable JSON text.
    let mut escaped = String::new();
    for scalar in invisible.chars() {
        let mut buffer = [0u16; 2];
        for unit in scalar.encode_utf16(&mut buffer) {
            write!(&mut escaped, "\\u{unit:04x}")
                .expect("string formatting cannot fail");
        }
    }
    assert!(display.contains(&format!(r#""special":"{escaped}""#)));
    assert!(display.is_ascii(), "all profile text is ASCII escaped");
    for scalar in invisible.chars() {
        assert!(!display.contains(scalar), "no invisible scalars remain");
    }
    assert_eq!(fs::read(&source.0).expect("source remains exact"), archive);
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
    let verified = Command::new(env!("CARGO_BIN_EXE_atrament"))
        .args(["profile", "verify"])
        .arg(&owned.0)
        .output()
        .expect("verify exact non-UTF8 selected path");
    assert!(verified.status.success());
    assert_eq!(verified.stdout, b"profile verified (no session import)\n");
    assert!(verified.stderr.is_empty());
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
    let verification = Command::new(env!("CARGO_BIN_EXE_atrament"))
        .args(["profile", "verify"])
        .arg(&owned.0)
        .output()
        .expect("verify must reject invalid later section too");
    assert!(!verification.status.success());
    assert!(verification.stdout.is_empty());
    let verification_stderr = String::from_utf8_lossy(&verification.stderr);
    assert!(verification_stderr.contains("SectionJson { index: 1"));
    for private in [
        "private-early-section-8392",
        "private-invalid-late-section-4752",
        "private-opaque-asset-2925",
        "private-late-selected-6614",
    ] {
        assert!(!error.to_string().contains(private));
        assert!(!stderr.contains(private));
        assert!(!verification_stderr.contains(private));
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
