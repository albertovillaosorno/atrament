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
//   - Real open-file profile Import and hostile byte/provenance regressions.
// - Must-Not:
//   - Change a session or treat a test path as production I/O authority.
// - Allows:
//   - Inputs: Session-owned OS temporary files and deliberate ZIP mutations.
//   - Outputs: Read-only transport and complete-manifest verification evidence.
//   - Side effects: Create and remove only owned temporary test fixtures.
// - Split-When:
//   - Trusted host path opening or profile acceptance needs separate tests.
// - Merge-When:
//   - Complete profile Import end-to-end tests include these exact assertions.
// - Summary:
//   - Bad bytes cannot reach inspection; valid open files are read exactly.
// - Description:
//   - Open-file inspection never opens paths; selected entrypoint does.
// - Usage:
//   - Run Cargo test for the profile open-file-import adapter.
// - Defaults:
//   - Resource, content and file-source failures refuse without callback.
//

//! Real file descriptors exercise bounded, source-attached Import verification.

#![expect(
    clippy::expect_used,
    clippy::format_collect,
    clippy::missing_trait_methods,
    clippy::redundant_closure_for_method_calls,
    reason = "test-owned fail-fast and HRTB-borrowed inspection fixtures"
)]

use std::cell::Cell;
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Seek as _, SeekFrom, Write as _};
use std::path::PathBuf;
use std::process;
use std::sync::atomic::{AtomicU64, Ordering};

use atrament_explicit_file_path_intent::{
    ExplicitFilePathSource, ExplicitFilePurpose, ExplicitOverwriteIntent,
    classify_explicit_file_path_intent,
};
use atrament_handwriting_profile_container::{
    ProfileEntryVerificationError, ProfileFeatureList, ProfileManifestError,
};
use atrament_profile_entry_sha256::sha256_profile_bytes;
use atrament_profile_file_reader::{
    ProfileFileReadError, read_bounded_profile_file,
};
use atrament_profile_import_preflight::{
    ProfileImportBudget, ProfileImportBudgetError, ProfileImportPreflightError,
};
use atrament_profile_import_verification::ProfileImportVerificationError;
use atrament_profile_json_syntax::{
    ProfileJsonSyntaxError, ProfileJsonSyntaxLimits,
};
use atrament_profile_manifest_json_codec::{
    ProfileManifestJsonError, ProfileManifestJsonLimits,
};
use atrament_profile_open_file_import::{
    ProfileOpenFileImportError as Error, ProfileOpenFileImportLimits,
    inspect_open_profile_import, inspect_selected_profile_import,
    open_selected_profile_import_probe,
};
use atrament_profile_selected_file_open::ProfileSelectedFileOpenError;
use atrament_profile_stored_zip_writer::{
    ProfileStoredZipMember, ProfileStoredZipWriteLimits,
    write_stored_profile_zip,
};
use atrament_profile_zip_reader::ProfileZipPayloadVerificationError;
use atrament_profile_zip_reader::{
    ProfileZipManifestSyntaxError, ProfileZipReadError, ProfileZipReadLimits,
};

const MEMBER: &str = "sections/writer.json";
static FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[cfg(target_os = "linux")]
struct OwnedProbeDirectory {
    path: PathBuf,
}

struct OwnedTestFile {
    path: PathBuf,
}

#[cfg(target_os = "linux")]
impl OwnedProbeDirectory {
    fn new() -> Self {
        let path = env::temp_dir().join(format!(
            "atrament-import-probe-{}-{}",
            process::id(),
            FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed),
        ));
        // A reused path must refuse; this fixture will remove its tree.
        #[expect(clippy::create_dir, reason = "test-owned unique directory")]
        fs::create_dir(&path).expect("create owned Import probe directory");
        Self { path }
    }
}

#[cfg(target_os = "linux")]
impl Drop for OwnedProbeDirectory {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.path)
            .expect("remove only owned Import probe directory");
    }
}

impl OwnedTestFile {
    fn new(bytes: &[u8]) -> Self {
        let path = env::temp_dir().join(format!(
            "atrament-open-profile-import-{}-{}.atrament",
            process::id(),
            FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed),
        ));
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)
            .expect("unique OS temporary fixture");
        file.write_all(bytes).expect("fixture write");
        Self { path }
    }

    fn open(&self) -> File {
        File::open(&self.path).expect("open owned fixture")
    }
}

impl Drop for OwnedTestFile {
    fn drop(&mut self) {
        fs::remove_file(&self.path).expect("remove owned fixture");
    }
}

const fn limits() -> ProfileOpenFileImportLimits {
    ProfileOpenFileImportLimits {
        maximum_file_bytes: 16_384,
        zip: ProfileZipReadLimits {
            maximum_archive_bytes: 16_384,
            maximum_entries: 3,
            maximum_entry_bytes: 8_192,
            maximum_total_bytes: 12_000,
        },
        root_json: ProfileJsonSyntaxLimits {
            maximum_bytes: 4_096,
            maximum_nesting: 8,
            maximum_object_members: 16,
            maximum_values: 50,
        },
        section_json: ProfileJsonSyntaxLimits {
            maximum_bytes: 1_024,
            maximum_nesting: 8,
            maximum_object_members: 16,
            maximum_values: 50,
        },
        manifest: ProfileManifestJsonLimits {
            maximum_entries: 2,
            maximum_features: 4,
            maximum_string_bytes: 256,
        },
        import: ProfileImportBudget {
            maximum_declared_bytes: 1_024,
            maximum_entries: 2,
            maximum_entry_bytes: 1_024,
        },
    }
}

fn selected(
    purpose: ExplicitFilePurpose,
) -> atrament_explicit_file_path_intent::ExplicitFilePathIntent {
    // The adapter must not open this path; the host supplied an open File.
    classify_explicit_file_path_intent(
        purpose,
        ExplicitFilePathSource::CallerSelection,
        PathBuf::from("/no-such-selected-path-never-opened.atrament"),
        ExplicitOverwriteIntent::PreserveExisting,
    )
    .expect("classified path is not file authority")
}

#[cfg(target_os = "linux")]
fn selected_probe_intent(
    path: PathBuf,
) -> atrament_explicit_file_path_intent::ExplicitFilePathIntent {
    classify_explicit_file_path_intent(
        ExplicitFilePurpose::ImportProfile,
        ExplicitFilePathSource::CallerSelection,
        path,
        ExplicitOverwriteIntent::PreserveExisting,
    )
    .expect("typed Import selection")
}

fn root(payload: &[u8], required: &str) -> String {
    let digest = sha256_profile_bytes(payload);
    let hash: String = digest
        .bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    format!(
        concat!(
            r#"{{"container_version":"atrament.profile/1","#,
            r#""profile_identity":"writer-1","required_features":{},"#,
            r#""optional_features":[],"entries":[{{"path":"{}","#,
            r#""media_type":"application/json","byte_length":{},"#,
            r#""sha256":"{}"}}]}}"#,
        ),
        required,
        MEMBER,
        payload.len(),
        hash,
    )
}

fn zip(root_json: &[u8], payload: &[u8]) -> Vec<u8> {
    write_stored_profile_zip(
        &[
            ProfileStoredZipMember {
                name: "manifest.json",
                bytes: root_json,
            },
            ProfileStoredZipMember {
                name: MEMBER,
                bytes: payload,
            },
        ],
        ProfileStoredZipWriteLimits {
            maximum_archive_bytes: 16_384,
            maximum_entries: 3,
            maximum_entry_bytes: 8_192,
            maximum_total_bytes: 12_000,
        },
    )
    .expect("well-formed stored ZIP frame")
}

#[test]
fn open_regular_file_reads_from_zero_and_exposes_only_verified_payload() {
    let payload = br#"{"strokes":[1,2,3]}"#;
    let root = root(payload, "[]");
    let bytes = zip(root.as_bytes(), payload);
    let fixture = OwnedTestFile::new(&bytes);
    let mut opened = fixture.open();
    let _position = opened.seek(SeekFrom::End(0)).expect("move file cursor");
    let path = selected(ExplicitFilePurpose::ImportProfile);
    let evidence = inspect_open_profile_import(
        &path,
        |max| read_bounded_profile_file(opened, max),
        &[],
        limits(),
        |verified| {
            assert_eq!(verified.path().path(), path.path());
            assert_eq!(verified.manifest().entries.len(), 1);
            assert_eq!(verified.verified_entry_count(), 1);
            assert_eq!(verified.verified_entry_bytes("manifest.json"), None);
            assert_eq!(verified.verified_entry_bytes("sections/missing"), None);
            (
                verified
                    .verified_entry_bytes(MEMBER)
                    .expect("SHA verified")
                    .to_vec(),
                verified.archive_sha256(),
                verified.root_sha256(),
            )
        },
    )
    .expect("complete profile bytes verified from open file");
    assert_eq!(evidence.0, payload);
    assert_eq!(evidence.1, sha256_profile_bytes(&bytes));
    assert_eq!(evidence.2, sha256_profile_bytes(root.as_bytes()));
}

#[cfg(target_os = "linux")]
#[test]
fn selected_linux_path_is_bound_to_opened_descriptor_and_full_sha() {
    let payload = br#"{"strokes":[1,2,3]}"#;
    let root = root(payload, "[]");
    let bytes = zip(root.as_bytes(), payload);
    let fixture = OwnedTestFile::new(&bytes);
    let chosen = classify_explicit_file_path_intent(
        ExplicitFilePurpose::ImportProfile,
        ExplicitFilePathSource::CallerSelection,
        fixture.path.clone(),
        ExplicitOverwriteIntent::PreserveExisting,
    )
    .expect("host-selected typed path");
    let inspected = inspect_selected_profile_import(
        &chosen, &[], limits(), |verified| {
        (
            verified.archive_sha256(),
            verified.verified_entry_bytes(MEMBER).map(<[u8]>::to_vec),
        )
    })
    .expect("source-sealed selected file verified");
    assert_eq!(inspected.0, sha256_profile_bytes(&bytes));
    assert_eq!(inspected.1.as_deref(), Some(payload.as_slice()));
}

#[cfg(target_os = "linux")]
#[test]
fn selected_probe_rechecks_same_byte_replacement_before_inspection() {
    use std::os::unix::fs::MetadataExt as _;

    let owned = OwnedProbeDirectory::new();
    let selected_path = owned.path.join("selected.atrament");
    let parked_path = owned.path.join("parked.atrament");
    let payload = br#"{"strokes":[1,2,3]}"#;
    let bytes = zip(root(payload, "[]").as_bytes(), payload);
    fs::write(&selected_path, &bytes).expect("write selected profile");
    let intent = selected_probe_intent(selected_path.clone());
    let probe =
        open_selected_profile_import_probe(&intent)
            .expect("initial securely selected source");
    fs::rename(&selected_path, &parked_path)
        .expect("park originally selected inode");
    fs::write(&selected_path, &bytes)
        .expect("replace selection with byte-identical ZIP");
    assert_ne!(
        fs::metadata(&selected_path).expect("replacement").ino(),
        fs::metadata(&parked_path).expect("original").ino(),
    );
    let inspected = Cell::new(false);
    assert_eq!(
        probe.inspect(&[], limits(), |_verified| inspected.set(true)),
        Err(Error::SelectedSourceChanged),
    );
    assert!(!inspected.get());
    assert_eq!(fs::read(&selected_path).expect("replacement"), bytes);
    assert_eq!(fs::read(&parked_path).expect("original"), bytes);
}

#[cfg(target_os = "linux")]
#[test]
fn selected_probe_refuses_late_symlink_and_missing_source() {
    use std::os::unix::fs::symlink;

    let owned = OwnedProbeDirectory::new();
    let selected_path = owned.path.join("selected.atrament");
    let parked_path = owned.path.join("parked.atrament");
    let payload = br#"{"strokes":[]}"#;
    let bytes = zip(root(payload, "[]").as_bytes(), payload);
    fs::write(&selected_path, &bytes).expect("write selected profile");
    let intent = selected_probe_intent(selected_path.clone());
    let missing_probe =
        open_selected_profile_import_probe(&intent)
            .expect("initial securely selected source");
    fs::rename(&selected_path, &parked_path)
        .expect("remove selected name without changing opened inode");
    let inspected = Cell::new(false);
    assert_eq!(
        missing_probe.inspect(&[], limits(), |_verified| inspected.set(true)),
        Err(Error::Open(ProfileSelectedFileOpenError::FileIo(
            io::ErrorKind::NotFound,
        ))),
    );
    let linked_probe = {
        fs::rename(&parked_path, &selected_path).expect("restore original");
        open_selected_profile_import_probe(&intent)
            .expect("securely reopen original")
    };
    fs::rename(&selected_path, &parked_path).expect("park original");
    symlink(&parked_path, &selected_path).expect("insert selected symlink");
    assert_eq!(
        linked_probe.inspect(&[], limits(), |_verified| inspected.set(true)),
        Err(Error::Open(ProfileSelectedFileOpenError::SymlinkForbidden)),
    );
    assert!(!inspected.get());
    assert_eq!(fs::read(&parked_path).expect("original"), bytes);
}

#[cfg(target_os = "linux")]
#[test]
fn selected_probe_rechecks_parent_rebinding_before_inspection() {
    let owned = OwnedProbeDirectory::new();
    let selected_dir = owned.path.join("selected-parent");
    let parked_dir = owned.path.join("original-parent");
    fs::create_dir_all(&selected_dir).expect("create selected parent");
    let selected_path = selected_dir.join("source.atrament");
    let payload = br#"{"strokes":[]}"#;
    let bytes = zip(root(payload, "[]").as_bytes(), payload);
    fs::write(&selected_path, &bytes).expect("write selected profile");
    let intent = selected_probe_intent(selected_path.clone());
    let probe =
        open_selected_profile_import_probe(&intent)
            .expect("initial securely selected source");
    fs::rename(&selected_dir, &parked_dir)
        .expect("move original parent and selected inode");
    fs::create_dir_all(&selected_dir).expect("replace selected parent");
    fs::write(&selected_path, &bytes)
        .expect("same-byte source under replacement parent");
    let inspected = Cell::new(false);
    assert_eq!(
        probe.inspect(&[], limits(), |_verified| inspected.set(true)),
        Err(Error::SelectedSourceChanged),
    );
    assert!(!inspected.get());
    assert_eq!(fs::read(&selected_path).expect("replacement"), bytes);
    assert_eq!(
        fs::read(parked_dir.join("source.atrament")).expect("original"),
        bytes,
    );
}

#[cfg(target_os = "linux")]
#[test]
fn selected_probe_refuses_parent_swap_with_preexisting_same_inode_hardlink() {
    use std::os::unix::fs::MetadataExt as _;

    let owned = OwnedProbeDirectory::new();
    let selected_dir = owned.path.join("selected-parent");
    let alternate_dir = owned.path.join("alternate-parent");
    let parked_dir = owned.path.join("parked-parent");
    fs::create_dir_all(&selected_dir).expect("selected directory");
    fs::create_dir_all(&alternate_dir).expect("alternate directory");
    let selected_path = selected_dir.join("source.atrament");
    let alternate_path = alternate_dir.join("source.atrament");
    let payload = br#"{"strokes":[]}"#;
    let bytes = zip(root(payload, "[]").as_bytes(), payload);
    fs::write(&selected_path, &bytes).expect("selected bytes");
    // Establish both links BEFORE the first observation: no later leaf
    // modification, inode or ctime change can reveal the parent swap.
    fs::hard_link(&selected_path, &alternate_path)
        .expect("alternate path to the same source inode");
    let initial = fs::metadata(&selected_path).expect("original metadata");
    let intent = selected_probe_intent(selected_path.clone());
    let probe = open_selected_profile_import_probe(&intent)
        .expect("open initial selection");
    fs::rename(&selected_dir, &parked_dir).expect("park selected directory");
    fs::rename(&alternate_dir, &selected_dir)
        .expect("replace selected directory with prepared hardlink");
    let replacement = fs::metadata(&selected_path).expect("same inode");
    assert_eq!(initial.ino(), replacement.ino());
    assert_eq!(initial.ctime(), replacement.ctime());
    assert_eq!(initial.ctime_nsec(), replacement.ctime_nsec());
    let inspected = Cell::new(false);
    assert_eq!(
        probe.inspect(&[], limits(), |_verified| inspected.set(true)),
        Err(Error::SelectedSourceChanged),
    );
    assert!(!inspected.get());
    assert_eq!(fs::read(&selected_path).expect("selected source"), bytes);
    assert_eq!(
        fs::read(parked_dir.join("source.atrament")).expect("old source"),
        bytes,
    );
}

#[cfg(target_os = "linux")]
#[test]
fn selected_probe_refuses_inplace_timestamp_drift_before_inspection() {
    use std::time::SystemTime;

    let owned = OwnedProbeDirectory::new();
    let path = owned.path.join("selected.atrament");
    let payload = br#"{"strokes":[]}"#;
    let bytes = zip(root(payload, "[]").as_bytes(), payload);
    fs::write(&path, &bytes).expect("selected source");
    let intent = selected_probe_intent(path.clone());
    let probe = open_selected_profile_import_probe(&intent)
        .expect("original selected file");
    let opened = OpenOptions::new().write(true).open(&path)
        .expect("test owned selected file");
    opened.set_modified(SystemTime::UNIX_EPOCH)
        .expect("adjust selected file timestamp without changing bytes");
    let inspected = Cell::new(false);
    assert_eq!(
        probe.inspect(&[], limits(), |_verified| inspected.set(true)),
        Err(Error::SelectedSourceChanged),
    );
    assert!(!inspected.get());
    assert_eq!(fs::read(&path).expect("untouched bytes"), bytes);
}

#[cfg(target_os = "linux")]
#[test]
fn selected_probe_relative_cwd_rebinding_refuses_even_for_same_inode() {
    const CHILD: &str = "ATRAMENT_TEST_IMPORT_PROBE_CWD_CHILD";
    if env::var_os(CHILD).is_none() {
        let result = process::Command::new(
            env::current_exe().expect("running test executable"),
        )
        .arg("--exact")
        .arg(
            "selected_probe_relative_cwd_rebinding_refuses_even_for_same_inode",
        )
        .env(CHILD, "yes")
        .output()
        .expect("isolated import-CWD child");
        assert!(result.status.success(), "child failed: {}",
            String::from_utf8_lossy(&result.stderr));
        return;
    }

    // Only the isolated child changes the process-wide CWD, never the
    // parallel test runner. The alternative path hardlinks the original
    // source to prove CWD selection matters even for identical inodes.
    let owned = OwnedProbeDirectory::new();
    let first = owned.path.join("first");
    let second = owned.path.join("second");
    fs::create_dir_all(&first).expect("first selected directory");
    fs::create_dir_all(&second).expect("second selected directory");
    let payload = br#"{"strokes":[]}"#;
    let bytes = zip(root(payload, "[]").as_bytes(), payload);
    fs::write(first.join("chosen.atrament"), &bytes)
        .expect("original relative source");
    fs::hard_link(
        first.join("chosen.atrament"),
        second.join("chosen.atrament"),
    ).expect("alternate relative name with identical inode");
    let original_cwd = env::current_dir().expect("initial cwd");
    env::set_current_dir(&first).expect("select original relative root");
    let intent = selected_probe_intent(PathBuf::from("chosen.atrament"));
    let probe = open_selected_profile_import_probe(&intent)
        .expect("original relative selection");
    env::set_current_dir(&second).expect("rebind relative root");
    let inspected = Cell::new(false);
    assert_eq!(
        probe.inspect(&[], limits(), |_verified| inspected.set(true)),
        Err(Error::Open(
            ProfileSelectedFileOpenError::WorkingDirectoryChanged,
        )),
    );
    assert!(!inspected.get());
    env::set_current_dir(&first).expect("restore original relative root");
    assert_eq!(
        inspect_selected_profile_import(&intent, &[], limits(),
            |verified| verified.archive_sha256()),
        Ok(sha256_profile_bytes(&bytes)),
    );
    env::set_current_dir(original_cwd).expect("restore child test cwd");
    assert_eq!(fs::read(second.join("chosen.atrament")).expect("alias"), bytes);
}

#[cfg(target_os = "linux")]
#[test]
fn selected_crc_valid_source_with_wrong_sha_never_reaches_inspection() {
    let authentic = b"authentic";
    let forged = b"impostors";
    assert_eq!(authentic.len(), forged.len());
    let declared_root = root(authentic, "[]");
    // This is a new, structurally valid ZIP with the forged bytes' own CRC.
    let selected_zip = zip(declared_root.as_bytes(), forged);
    let source = OwnedTestFile::new(&selected_zip);
    let chosen = classify_explicit_file_path_intent(
        ExplicitFilePurpose::ImportProfile,
        ExplicitFilePathSource::CallerSelection,
        source.path.clone(),
        ExplicitOverwriteIntent::PreserveExisting,
    )
    .expect("typed selected Import path");
    let inspected = Cell::new(false);
    let result =
        inspect_selected_profile_import(
            &chosen, &[], limits(), |_verified| inspected.set(true));
    assert!(matches!(
        result,
        Err(Error::Verification(
            ProfileImportVerificationError::Payload(
                ProfileZipPayloadVerificationError::Entry {
                    index: 0,
                    reason: ProfileEntryVerificationError::DigestMismatch,
                },
            )
        )),
    ));
    assert!(
        !inspected.get(),
        "no payload inspection before full SHA admission"
    );
}

#[test]
fn selected_entrypoint_checks_purpose_before_touching_selected_path() {
    // The selected path does not exist; opening it would produce a distinct
    // OS error. Purpose rejection must take precedence even on Linux.
    for purpose in [
        ExplicitFilePurpose::ImportNotebook,
        ExplicitFilePurpose::ExportProfile,
        ExplicitFilePurpose::ExportNotebook,
    ] {
        let inspected = Cell::new(false);
        let intent = selected(purpose);
        assert_eq!(
            inspect_selected_profile_import(&intent, &[], limits(), |_view| {
                inspected.set(true);
            }),
            Err(Error::WrongPathPurpose),
        );
        assert!(!inspected.get());
    }
}

#[cfg(target_os = "linux")]
#[test]
fn selected_entrypoint_refuses_missing_source_despite_other_valid_file() {
    let payload = b"{}";
    let genuine =
        OwnedTestFile::new(&zip(root(payload, "[]").as_bytes(), payload));
    let chosen = selected(ExplicitFilePurpose::ImportProfile);
    let inspected = Cell::new(false);
    // A permissive external callback could use genuine.open() for chosen.
    // The path-bound API has no such callback and must refuse the selection.
    assert_eq!(
        inspect_selected_profile_import(&chosen, &[], limits(), |_verified| {
            inspected.set(true);
        }),
        Err(Error::Open(ProfileSelectedFileOpenError::FileIo(
            io::ErrorKind::NotFound,
        ))),
    );
    assert!(!inspected.get());
    assert_eq!(
        inspect_open_profile_import(
            &chosen,
            |max| read_bounded_profile_file(genuine.open(), max),
            &[],
            limits(),
            |verified| verified.verified_entry_count(),
        ),
        Ok(1),
        "opened-source API leaves descriptor binding to host",
    );
}

#[cfg(target_os = "linux")]
#[test]
fn selected_entrypoint_rejects_symlink_even_to_valid_profile() {
    use std::os::unix::fs::symlink;
    let payload = b"trusted";
    let genuine =
        OwnedTestFile::new(&zip(root(payload, "[]").as_bytes(), payload));
    let alias = genuine.path.with_extension("linked");
    symlink(&genuine.path, &alias).expect("test symlink");
    let chosen = classify_explicit_file_path_intent(
        ExplicitFilePurpose::ImportProfile,
        ExplicitFilePathSource::CallerSelection,
        alias.clone(),
        ExplicitOverwriteIntent::PreserveExisting,
    )
    .expect("classified alias is not authority");
    let inspected = Cell::new(false);
    let result =
        inspect_selected_profile_import(
            &chosen, &[], limits(), |_verified| inspected.set(true));
    fs::remove_file(&alias).expect("remove only test-created symlink");
    assert_eq!(
        result,
        Err(Error::Open(ProfileSelectedFileOpenError::SymlinkForbidden)),
    );
    assert!(!inspected.get());
}

#[cfg(not(target_os = "linux"))]
#[test]
fn selected_entrypoint_fails_closed_without_supported_open_policy() {
    let intent = selected(ExplicitFilePurpose::ImportProfile);
    assert_eq!(
        inspect_selected_profile_import(&intent, &[], limits(), |_verified| ()),
        Err(Error::Open(
            ProfileSelectedFileOpenError::UnsupportedOpenPolicy,
        )),
    );
}

#[test]
fn wrong_purpose_refuses_before_inspection_or_file_read() {
    let fixture = OwnedTestFile::new(b"not a zip");
    let inspected = Cell::new(false);
    let result = inspect_open_profile_import(
        &selected(ExplicitFilePurpose::ExportProfile),
        |max| read_bounded_profile_file(fixture.open(), max),
        &[],
        limits(),
        |_verified| inspected.set(true),
    );
    assert_eq!(result, Err(Error::WrongPathPurpose));
    assert!(!inspected.get());
}

#[test]
fn file_and_zip_byte_limits_refuse_before_inspection() {
    let payload = b"{}";
    let bytes = zip(root(payload, "[]").as_bytes(), payload);
    let fixture = OwnedTestFile::new(&bytes);
    let intent = selected(ExplicitFilePurpose::ImportProfile);
    let inspected = Cell::new(false);
    let mut config = limits();
    config.maximum_file_bytes = bytes.len() - 1;
    assert_eq!(
        inspect_open_profile_import(
            &intent,
            |max| read_bounded_profile_file(fixture.open(), max),
            &[],
            config,
            |_verified| inspected.set(true)
        ),
        Err(Error::Source(ProfileFileReadError::FileTooLarge)),
    );
    config.maximum_file_bytes = bytes.len();
    config.zip.maximum_archive_bytes = bytes.len() - 1;
    assert_eq!(
        inspect_open_profile_import(
            &intent,
            |max| read_bounded_profile_file(fixture.open(), max),
            &[],
            config,
            |_verified| inspected.set(true)
        ),
        Err(Error::Source(ProfileFileReadError::FileTooLarge)),
    );
    config.zip.maximum_archive_bytes = bytes.len();
    assert_eq!(
        inspect_open_profile_import(
            &intent,
            |max| read_bounded_profile_file(fixture.open(), max),
            &[],
            config,
            |verified| verified.verified_entry_count()
        ),
        Ok(1),
    );
    assert!(!inspected.get());
}

#[test]
fn directory_and_write_only_descriptor_refuse() {
    let intent = selected(ExplicitFilePurpose::ImportProfile);
    let directory =
        File::open(env::temp_dir()).expect("OS test directory");
    assert_eq!(
        inspect_open_profile_import(
            &intent,
            |max| read_bounded_profile_file(directory, max),
            &[],
            limits(),
            |_verified| ()
        ),
        Err(Error::Source(ProfileFileReadError::NotRegularFile)),
    );
    let fixture = OwnedTestFile::new(b"abc");
    let write_only = OpenOptions::new()
        .write(true)
        .open(&fixture.path)
        .expect("open owned fixture for writes only");
    assert!(matches!(
        inspect_open_profile_import(
            &intent,
            |max| read_bounded_profile_file(write_only, max),
            &[],
            limits(),
            |_verified| ()
        ),
        Err(Error::Source(ProfileFileReadError::FileIo(_))),
    ));
}

#[test]
fn broken_zip_and_root_are_distinct_without_callback() {
    let intent = selected(ExplicitFilePurpose::ImportProfile);
    let called = Cell::new(0usize);
    let malformed = OwnedTestFile::new(b"this is not a zip");
    assert!(matches!(
        inspect_open_profile_import(
            &intent,
            |max| read_bounded_profile_file(malformed.open(), max),
            &[],
            limits(),
            |_verified| called.set(called.get() + 1usize)
        ),
        Err(Error::Zip(
            ProfileZipReadError::Truncated
                | ProfileZipReadError::InvalidStructure
        )),
    ));
    let malformed_root = zip(b"{invalid-json}", b"data");
    let fixture = OwnedTestFile::new(&malformed_root);
    assert!(matches!(
        inspect_open_profile_import(
            &intent,
            |max| read_bounded_profile_file(fixture.open(), max),
            &[],
            limits(),
            |_verified| called.set(called.get() + 1usize)
        ),
        Err(Error::RootJson(ProfileZipManifestSyntaxError::Json(
            ProfileJsonSyntaxError::Malformed
        ))),
    ));
    let schema_root = zip(br#"{"not-a-manifest":true}"#, b"data");
    let schema_file = OwnedTestFile::new(&schema_root);
    assert!(matches!(
        inspect_open_profile_import(
            &intent,
            |max| read_bounded_profile_file(schema_file.open(), max),
            &[],
            limits(),
            |_verified| called.set(called.get() + 1usize)
        ),
        Err(Error::Manifest(ProfileManifestJsonError::UnknownField(_))),
    ));
    assert_eq!(called.get(), 0usize);
}

#[test]
fn unsupported_features_and_crc_correct_sha_substitution_fail_closed() {
    let intent = selected(ExplicitFilePurpose::ImportProfile);
    let payload = b"authentic";
    let required_root = root(payload, r#"["never-supported"]"#);
    let required_zip =
        OwnedTestFile::new(&zip(required_root.as_bytes(), payload));
    let inspected = Cell::new(false);
    assert!(matches!(
        inspect_open_profile_import(
            &intent,
            |max| read_bounded_profile_file(required_zip.open(), max),
            &[],
            limits(),
            |_verified| { inspected.set(true) }
        ),
        Err(Error::Verification(
            ProfileImportVerificationError::Preflight(_)
        )),
    ));
    let declared_root = root(payload, "[]");
    let replaced =
        OwnedTestFile::new(&zip(declared_root.as_bytes(), b"impostors"));
    assert!(matches!(
        inspect_open_profile_import(
            &intent,
            |max| read_bounded_profile_file(replaced.open(), max),
            &[],
            limits(),
            |_verified| { inspected.set(true) }
        ),
        Err(Error::Verification(
            ProfileImportVerificationError::Payload(
                ProfileZipPayloadVerificationError::Entry {
                    index: 0,
                    reason: ProfileEntryVerificationError::DigestMismatch,
                },
            )
        )),
    ));
    assert!(!inspected.get());
}

#[test]
fn contradictory_feature_requirements_refuse_before_file_inspection() {
    let payload = br#"{"ink":true}"#;
    let valid = root(payload, r#"["stroke-vocabulary"]"#);
    let conflicting = valid.replace(
        "\"optional_features\":[]",
        "\"optional_features\":[\"stroke-vocabulary\"]",
    );
    assert_ne!(valid, conflicting);
    let file = OwnedTestFile::new(&zip(conflicting.as_bytes(), payload));
    let intent = selected(ExplicitFilePurpose::ImportProfile);
    let inspected = Cell::new(false);
    assert_eq!(
        inspect_open_profile_import(
            &intent,
            |max| read_bounded_profile_file(file.open(), max),
            &["stroke-vocabulary"],
            limits(),
            |_verified| { inspected.set(true); },
        ),
        Err(Error::Verification(ProfileImportVerificationError::Preflight(
            ProfileImportPreflightError::Manifest(
                ProfileManifestError::ConflictingFeatureRequirement {
                    feature: "stroke-vocabulary".into(),
                },
            ),
        ))),
    );
    assert!(!inspected.get(), "no verified section can escape on ambiguity");
}

#[test]
fn repeated_optional_features_refuse_before_any_file_inspection() {
    let payload = br#"{"ink":true}"#;
    let valid = root(payload, r#"["stroke-vocabulary"]"#);
    let intent = selected(ExplicitFilePurpose::ImportProfile);
    for optional in [
        r#""optional_features":["future","future"]"#,
        r#""optional_features":["future","\u0066uture"]"#,
    ] {
        let repeated = valid.replace("\"optional_features\":[]", optional);
        let file = OwnedTestFile::new(&zip(repeated.as_bytes(), payload));
        let inspected = Cell::new(false);
        assert_eq!(
            inspect_open_profile_import(
                &intent,
                |max| read_bounded_profile_file(file.open(), max),
                &["stroke-vocabulary"],
                limits(),
                |_verified| inspected.set(true),
            ),
            Err(Error::Verification(
                ProfileImportVerificationError::Preflight(
                    ProfileImportPreflightError::Manifest(
                        ProfileManifestError::DuplicateFeatureIdentifier {
                            feature: "future".into(),
                            list: ProfileFeatureList::Optional,
                        },
                    ),
                ),
            )),
            "optional feature wire spelling {optional}",
        );
        assert!(!inspected.get());
    }
}

#[cfg(target_os = "linux")]
#[test]
fn selected_path_refuses_conflicting_features_without_inspection() {
    let payload = br#"{"stroke":[1,2]}"#;
    let valid = root(payload, r#"["stroke-vocabulary"]"#);
    let conflicting = valid.replace(
        "\"optional_features\":[]",
        "\"optional_features\":[\"stroke-vocabulary\"]",
    );
    let file = OwnedTestFile::new(&zip(conflicting.as_bytes(), payload));
    let chosen = classify_explicit_file_path_intent(
        ExplicitFilePurpose::ImportProfile,
        ExplicitFilePathSource::CallerSelection,
        file.path.clone(),
        ExplicitOverwriteIntent::PreserveExisting,
    )
    .expect("test-owned selected Import path");
    let inspected = Cell::new(false);
    assert_eq!(
        inspect_selected_profile_import(
            &chosen,
            &["stroke-vocabulary"],
            limits(),
            |_verified| inspected.set(true),
        ),
        Err(Error::Verification(ProfileImportVerificationError::Preflight(
            ProfileImportPreflightError::Manifest(
                ProfileManifestError::ConflictingFeatureRequirement {
                    feature: "stroke-vocabulary".into(),
                },
            ),
        ))),
    );
    assert!(!inspected.get(), "no selected profile bytes were exposed");
}

#[cfg(target_os = "linux")]
#[test]
fn selected_path_rejects_repeated_required_features_before_inspection() {
    let section = br#"{"stroke":[1,2]}"#;
    let duplicate_root = root(
        section, r#"["stroke-vocabulary","stroke-\u0076ocabulary"]"#,
    );
    let archive = zip(duplicate_root.as_bytes(), section);
    let file = OwnedTestFile::new(&archive);
    let chosen = classify_explicit_file_path_intent(
        ExplicitFilePurpose::ImportProfile,
        ExplicitFilePathSource::CallerSelection,
        file.path.clone(),
        ExplicitOverwriteIntent::PreserveExisting,
    )
    .expect("classify selected profile file");
    let inspected = Cell::new(false);
    assert_eq!(
        inspect_selected_profile_import(
            &chosen,
            &["stroke-vocabulary"],
            limits(),
            |_verified| inspected.set(true),
        ),
        Err(Error::Verification(ProfileImportVerificationError::Preflight(
            ProfileImportPreflightError::Manifest(
                ProfileManifestError::DuplicateFeatureIdentifier {
                    feature: "stroke-vocabulary".into(),
                    list: ProfileFeatureList::Required,
                },
            ),
        ))),
    );
    assert!(!inspected.get());
    assert_eq!(
        fs::read(&file.path).expect("test-owned selected bytes"),
        archive,
    );
}

#[test]
fn empty_regular_file_reaches_zip_refusal_not_synthetic_eof() {
    let fixture = OwnedTestFile::new(b"");
    let intent = selected(ExplicitFilePurpose::ImportProfile);
    let mut config = limits();
    config.maximum_file_bytes = 0;
    assert!(matches!(
        inspect_open_profile_import(
            &intent,
            |max| read_bounded_profile_file(fixture.open(), max),
            &[],
            config,
            |_verified| ()
        ),
        Err(Error::Zip(ProfileZipReadError::Truncated)),
    ));
}

#[cfg(target_os = "linux")]
#[test]
fn pseudo_regular_zero_length_file_does_not_fake_eof() {
    // Procfs exposes some readable files as regular with st_size=0.
    let source = File::open("/proc/version").expect("Linux procfs fixture");
    assert_eq!(source.metadata().expect("stat").len(), 0);
    let intent = selected(ExplicitFilePurpose::ImportProfile);
    assert_eq!(
        inspect_open_profile_import(
            &intent,
            |max| read_bounded_profile_file(source, max),
            &[],
            limits(),
            |_verified| ()
        ),
        Err(Error::Source(ProfileFileReadError::FileLengthChanged)),
    );
}

#[test]
fn tighter_import_and_schema_entry_caps_refuse_zip_before_root_decode() {
    let payload = b"{}";
    // Invalid JSON still has valid stored ZIP framing and CRC. The member
    // count must refuse without attempting root JSON or digest inspection.
    let fixture = OwnedTestFile::new(&zip(b"{broken-json}", payload));
    let intent = selected(ExplicitFilePurpose::ImportProfile);
    let inspected = Cell::new(false);
    for (import_max, schema_max, zip_max) in [(0, 2, 3), (2, 0, 3), (2, 2, 1)] {
        let mut config = limits();
        config.import.maximum_entries = import_max;
        config.manifest.maximum_entries = schema_max;
        config.zip.maximum_entries = zip_max;
        assert_eq!(
            inspect_open_profile_import(
                &intent,
                |max| read_bounded_profile_file(fixture.open(), max),
                &[],
                config,
                |_verified| inspected.set(true),
            ),
            Err(Error::Zip(ProfileZipReadError::ResourceLimit)),
        );
    }
    assert!(!inspected.get());
    let mut config = limits();
    config.import.maximum_entries = usize::MAX;
    config.manifest.maximum_entries = usize::MAX;
    assert!(matches!(
        inspect_open_profile_import(
            &intent,
            |max| read_bounded_profile_file(fixture.open(), max),
            &[],
            config,
            |_verified| (),
        ),
        Err(Error::RootJson(ProfileZipManifestSyntaxError::Json(
            ProfileJsonSyntaxError::Malformed,
        ))),
    ));
}

#[test]
fn manifest_only_archive_passes_with_zero_non_root_entry_budget() {
    let root = concat!(
        r#"{"container_version":"atrament.profile/1","#,
        r#""profile_identity":"empty","required_features":[],"#,
        r#""optional_features":[],"entries":[]}"#,
    );
    let bytes = write_stored_profile_zip(
        &[ProfileStoredZipMember {
            name: "manifest.json",
            bytes: root.as_bytes(),
        }],
        ProfileStoredZipWriteLimits {
            maximum_archive_bytes: 16_384,
            maximum_entries: 3,
            maximum_entry_bytes: 8_192,
            maximum_total_bytes: 12_000,
        },
    )
    .expect("manifest-only stored archive");
    let fixture = OwnedTestFile::new(&bytes);
    let intent = selected(ExplicitFilePurpose::ImportProfile);
    let mut config = limits();
    config.import.maximum_entries = 0;
    config.import.maximum_declared_bytes = 0;
    config.import.maximum_entry_bytes = 0;
    config.manifest.maximum_entries = 0;
    config.section_json.maximum_bytes = 0;
    config.section_json.maximum_values = 0;
    assert_eq!(
        inspect_open_profile_import(
            &intent,
            |max| read_bounded_profile_file(fixture.open(), max),
            &[],
            config,
            |verified| {
                assert_eq!(
                    verified.verified_entry_bytes("manifest.json"),
                    None
                );
                assert!(verified.manifest().entries.is_empty());
                (verified.verified_entry_count(), verified.archive_sha256())
            },
        ),
        Ok((0, sha256_profile_bytes(&bytes))),
    );
    config.zip.maximum_entries = 0;
    assert_eq!(
        inspect_open_profile_import(
            &intent,
            |max| read_bounded_profile_file(fixture.open(), max),
            &[],
            config,
            |_verified| (),
        ),
        Err(Error::Zip(ProfileZipReadError::ResourceLimit)),
    );
}

#[test]
fn sha_valid_section_syntax_failure_refuses_before_generic_inspection() {
    let section = b"{not-json}";
    let asset = b"\0\xffopaque non-JSON asset";
    let manifest = two_entry_manifest(section, asset);
    let original = OwnedTestFile::new(&two_entry_zip(
        &manifest, section, asset,
    ));
    let chosen = classify_explicit_file_path_intent(
        ExplicitFilePurpose::ImportProfile,
        ExplicitFilePathSource::CallerSelection,
        original.path.clone(),
        ExplicitOverwriteIntent::PreserveExisting,
    )
    .expect("real caller-selected profile");
    let observed = Cell::new(false);
    assert_eq!(
        inspect_selected_profile_import(
            &chosen,
            &[],
            limits(),
            |_verified| observed.set(true),
        ),
        Err(Error::SectionJson {
            index: 0,
            reason: ProfileJsonSyntaxError::Malformed,
        }),
    );
    assert!(!observed.get());
}

#[test]
fn sha_valid_non_json_section_media_type_refuses_before_inspection() {
    let section = br#"{"strokes":[]}"#;
    let admitted_root = root(section, "[]");
    let intent = selected(ExplicitFilePurpose::ImportProfile);
    let called = Cell::new(false);
    for mime in [
        "text/plain", "image/png", "application/json; charset=utf-8",
    ] {
        let root = admitted_root.replacen("application/json", mime, 1);
        let source = OwnedTestFile::new(&zip(root.as_bytes(), section));
        assert_eq!(
            inspect_open_profile_import(
                &intent,
                |max| read_bounded_profile_file(source.open(), max),
                &[],
                limits(),
                |_verified| called.set(true),
            ),
            Err(Error::SectionMediaType { index: 0 }),
            "SHA-valid, syntactically valid sections cannot misdeclare MIME",
        );
        assert!(!called.get());
    }
    let valid_source = OwnedTestFile::new(&zip(
        admitted_root.as_bytes(), section,
    ));
    assert_eq!(
        inspect_open_profile_import(
            &intent,
            |max| read_bounded_profile_file(valid_source.open(), max),
            &[],
            limits(),
            |verified| {
                verified.verified_entry_bytes(MEMBER).map(<[u8]>::to_vec)
            },
        ),
        Ok(Some(section.to_vec())),
    );
}

#[test]
fn section_json_duplicate_keys_and_independent_budget_refuse_inspection() {
    let intent = selected(ExplicitFilePurpose::ImportProfile);
    let duplicate = br#"{"a":1,"\u0061":2}"#;
    let duplicate_file = OwnedTestFile::new(&zip(
        root(duplicate, "[]").as_bytes(), duplicate,
    ));
    let called = Cell::new(false);
    assert_eq!(
        inspect_open_profile_import(
            &intent,
            |max| read_bounded_profile_file(duplicate_file.open(), max),
            &[],
            limits(),
            |_verified| called.set(true),
        ),
        Err(Error::SectionJson {
            index: 0,
            reason: ProfileJsonSyntaxError::DuplicateObjectKey,
        }),
    );
    assert!(!called.get());

    let valid = br#"{"a":1}"#;
    let valid_file = OwnedTestFile::new(&zip(
        root(valid, "[]").as_bytes(), valid,
    ));
    let mut bounded = limits();
    bounded.section_json.maximum_bytes = valid.len() - 1;
    assert_eq!(
        inspect_open_profile_import(
            &intent,
            |max| read_bounded_profile_file(valid_file.open(), max),
            &[],
            bounded,
            |_verified| called.set(true),
        ),
        Err(Error::SectionJson {
            index: 0,
            reason: ProfileJsonSyntaxError::MaximumBytesExceeded,
        }),
    );
    assert!(!called.get());
}

#[test]
fn section_nesting_keys_values_and_utf8_require_independent_admission() {
    let default_section_limits = limits().section_json;
    let cases: [(&[u8], ProfileJsonSyntaxLimits, ProfileJsonSyntaxError); 4] = [
        (
            br#"{"outer":{"inner":{}}}"#,
            ProfileJsonSyntaxLimits {
                maximum_nesting: 2,
                ..default_section_limits
            },
            ProfileJsonSyntaxError::MaximumNestingExceeded,
        ),
        (
            br#"{"a":1,"b":2}"#,
            ProfileJsonSyntaxLimits {
                maximum_object_members: 1,
                ..default_section_limits
            },
            ProfileJsonSyntaxError::MaximumObjectMembersExceeded,
        ),
        (
            br#"{"a":1,"b":2}"#,
            ProfileJsonSyntaxLimits {
                maximum_values: 2,
                ..default_section_limits
            },
            ProfileJsonSyntaxError::MaximumValuesExceeded,
        ),
        (
            b"{\"a\":\"\xff\"}",
            default_section_limits,
            ProfileJsonSyntaxError::InvalidUtf8,
        ),
    ];
    let intent = selected(ExplicitFilePurpose::ImportProfile);
    let called = Cell::new(false);
    for (section, section_json, reason) in cases {
        let fixture = OwnedTestFile::new(&zip(
            root(section, "[]").as_bytes(), section,
        ));
        let mut config = limits();
        config.section_json = section_json;
        assert_eq!(
            inspect_open_profile_import(
                &intent,
                |max| read_bounded_profile_file(fixture.open(), max),
                &[],
                config,
                |_verified| called.set(true),
            ),
            Err(Error::SectionJson { index: 0, reason }),
        );
        assert!(!called.get());
    }
}

#[cfg(target_os = "linux")]
#[test]
fn valid_section_preserves_non_utf8_opaque_asset_without_decoding() {
    let section = br#"{"written":true}"#;
    let opaque = b"\xff\0binary\x80";
    let manifest = two_entry_manifest(section, opaque);
    let bytes = two_entry_zip(&manifest, section, opaque);
    let source = OwnedTestFile::new(&bytes);
    let selected = classify_explicit_file_path_intent(
        ExplicitFilePurpose::ImportProfile,
        ExplicitFilePathSource::CallerSelection,
        source.path.clone(),
        ExplicitOverwriteIntent::PreserveExisting,
    )
    .expect("source is caller-selected");
    let inspected = inspect_selected_profile_import(
        &selected,
        &[],
        limits(),
        |verified| {
            (
                verified.verified_entry_bytes(MEMBER).map(<[u8]>::to_vec),
                verified.verified_entry_bytes("assets/icon.png")
                    .map(<[u8]>::to_vec),
            )
        },
    );
    assert_eq!(
        inspected,
        Ok((Some(section.to_vec()), Some(opaque.to_vec()))),
    );
}

#[test]
fn later_invalid_section_cannot_expose_prior_verified_section() {
    let first = br#"{"valid":true}"#;
    let last = b"[1,2,3]";
    let first_manifest = root(first, "[]");
    let prefix = first_manifest
        .strip_suffix("]}")
        .expect("one-entry manifest suffix");
    let digest: String = sha256_profile_bytes(last)
        .bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let manifest = format!(
        concat!(
            r#"{},{{"path":"sections/z.json","media_type":"application/json","#,
            r#""byte_length":{},"sha256":"{}"}}]}}"#,
        ),
        prefix,
        last.len(),
        digest,
    );
    let archive = write_stored_profile_zip(
        &[
            ProfileStoredZipMember {
                name: "manifest.json",
                bytes: manifest.as_bytes(),
            },
            ProfileStoredZipMember {
                name: MEMBER,
                bytes: first,
            },
            ProfileStoredZipMember {
                name: "sections/z.json",
                bytes: last,
            },
        ],
        ProfileStoredZipWriteLimits {
            maximum_archive_bytes: 16_384,
            maximum_entries: 3,
            maximum_entry_bytes: 8_192,
            maximum_total_bytes: 12_000,
        },
    )
    .expect("valid ZIP and complete manifest hashes");
    let file = OwnedTestFile::new(&archive);
    let called = Cell::new(false);
    assert_eq!(
        inspect_open_profile_import(
            &selected(ExplicitFilePurpose::ImportProfile),
            |max| read_bounded_profile_file(file.open(), max),
            &[],
            limits(),
            |_verified| called.set(true),
        ),
        Err(Error::SectionJson {
            index: 1,
            reason: ProfileJsonSyntaxError::RootNotObject,
        }),
    );
    assert!(!called.get());

    // A second section's misleading type must fail just as atomically as
    // its bad syntax, even when all section bytes and digests are unchanged.
    let mislabeled = manifest.replace(
        r#""path":"sections/z.json","media_type":"application/json""#,
        r#""path":"sections/z.json","media_type":"text/plain""#,
    );
    assert_ne!(mislabeled, manifest);
    let mismatched_archive = write_stored_profile_zip(
        &[
            ProfileStoredZipMember {
                name: "manifest.json",
                bytes: mislabeled.as_bytes(),
            },
            ProfileStoredZipMember { name: MEMBER, bytes: first },
            ProfileStoredZipMember { name: "sections/z.json", bytes: last },
        ],
        ProfileStoredZipWriteLimits {
            maximum_archive_bytes: 16_384,
            maximum_entries: 3,
            maximum_entry_bytes: 8_192,
            maximum_total_bytes: 12_000,
        },
    )
    .expect("fully SHA-valid ZIP with late misdeclared type");
    let mismatched_file = OwnedTestFile::new(&mismatched_archive);
    assert_eq!(
        inspect_open_profile_import(
            &selected(ExplicitFilePurpose::ImportProfile),
            |max| read_bounded_profile_file(mismatched_file.open(), max),
            &[],
            limits(),
            |_verified| called.set(true),
        ),
        Err(Error::SectionMediaType { index: 1 }),
    );
    assert!(!called.get());
}

fn two_entry_manifest(section: &[u8], asset: &[u8]) -> String {
    let one = root(section, "[]");
    let prefix = one
        .strip_suffix("]}")
        .expect("one-entry fixture has a JSON entries suffix");
    let hash: String = sha256_profile_bytes(asset)
        .bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    format!(
        concat!(
            r#"{},{{"path":"assets/icon.png","media_type":"image/png","#,
            r#""byte_length":{},"sha256":"{}"}}]}}"#,
        ),
        prefix,
        asset.len(),
        hash,
    )
}

fn two_entry_zip(root_json: &str, section: &[u8], asset: &[u8]) -> Vec<u8> {
    // Canonical ZIP name ordering differs from declaration order.
    write_stored_profile_zip(
        &[
            ProfileStoredZipMember {
                name: "assets/icon.png",
                bytes: asset,
            },
            ProfileStoredZipMember {
                name: "manifest.json",
                bytes: root_json.as_bytes(),
            },
            ProfileStoredZipMember {
                name: MEMBER,
                bytes: section,
            },
        ],
        ProfileStoredZipWriteLimits {
            maximum_archive_bytes: 16_384,
            maximum_entries: 3,
            maximum_entry_bytes: 8_192,
            maximum_total_bytes: 12_000,
        },
    )
    .expect("canonical three-member stored ZIP")
}

#[test]
fn two_payload_open_file_admits_only_after_all_digest_checks() {
    let section = br#"{"first":true}"#;
    let asset = b"second";
    let root_json = two_entry_manifest(section, asset);
    let original = two_entry_zip(&root_json, section, asset);
    let original_file = OwnedTestFile::new(&original);
    let intent = selected(ExplicitFilePurpose::ImportProfile);
    let success = inspect_open_profile_import(
        &intent,
        |max| read_bounded_profile_file(original_file.open(), max),
        &[],
        limits(),
        |verified| {
            assert_eq!(verified.verified_entry_count(), 2);
            (
                verified
                    .verified_entry_bytes(MEMBER)
                    .expect("verified section")
                    .to_vec(),
                verified
                    .verified_entry_bytes("assets/icon.png")
                    .expect("verified asset")
                    .to_vec(),
                verified.archive_sha256(),
            )
        },
    )
    .expect("all original members pass SHA-256");
    assert_eq!(success.0, section);
    assert_eq!(success.1, asset);
    assert_eq!(success.2, sha256_profile_bytes(&original));

    let inspected = Cell::new(false);
    // Both payload replacements are equal length, and the writer updates
    // CRC-32 for each forged ZIP. Only declared SHA-256 can reject them.
    for (replacement_section, replacement_asset, index) in [
        (br#"{"first":null}"#.as_slice(), asset.as_slice(), 0),
        (section.as_slice(), b"seconD".as_slice(), 1),
    ] {
        let altered =
            two_entry_zip(&root_json, replacement_section, replacement_asset);
        let file = OwnedTestFile::new(&altered);
        assert_eq!(
            inspect_open_profile_import(
                &intent,
                |max| read_bounded_profile_file(file.open(), max),
                &[],
                limits(),
                |_verified| inspected.set(true),
            ),
            Err(Error::Verification(
                ProfileImportVerificationError::Payload(
                    ProfileZipPayloadVerificationError::Entry {
                        index,
                        reason: ProfileEntryVerificationError::DigestMismatch,
                    },
                ),
            )),
        );
    }
    assert!(!inspected.get());
}

#[cfg(target_os = "linux")]
#[test]
fn all_crc_correct_payload_bit_flips_refuse_selected_file_inspection() {
    let section = br#"{"strokes":[1,2,3]}"#;
    let asset = b"\0\xffopaque-private-bytes";
    let root_json = two_entry_manifest(section, asset);
    let original = two_entry_zip(&root_json, section, asset);
    let intact = OwnedTestFile::new(&original);
    let chosen = classify_explicit_file_path_intent(
        ExplicitFilePurpose::ImportProfile,
        ExplicitFilePathSource::CallerSelection,
        intact.path.clone(),
        ExplicitOverwriteIntent::PreserveExisting,
    ).expect("test-owned selected input");
    let baseline = inspect_selected_profile_import(
        &chosen, &[], limits(), |verified| {
            assert_eq!(verified.verified_entry_count(), 2);
            assert_eq!(
                verified.verified_entry_bytes(MEMBER),
                Some(section.as_slice()),
            );
            assert_eq!(
                verified.verified_entry_bytes("assets/icon.png"),
                Some(asset.as_slice()),
            );
        },
    );
    assert_eq!(baseline, Ok(()), "unmodified source is fully admitted");

    let inspection_called = Cell::new(false);
    let mut attempted = 0usize;
    // Each forged ZIP has valid CRC32 and exact local/central framing;
    // only the unchanged manifest SHA-256 commitments can reject it.
    for (index, payload) in [
        (0usize, section.as_slice()),
        (1, asset.as_slice()),
    ] {
        for position in 0..payload.len() {
            for bit in 0..8u8 {
                let mut forged_payload = payload.to_vec();
                let selected_byte = forged_payload.get_mut(position)
                    .expect("position is bounded by payload length");
                *selected_byte ^= 1u8 << bit;
                let forged = if index == 0 {
                    two_entry_zip(&root_json, &forged_payload, asset)
                } else {
                    two_entry_zip(&root_json, section, &forged_payload)
                };
                let fixture = OwnedTestFile::new(&forged);
                let path = classify_explicit_file_path_intent(
                    ExplicitFilePurpose::ImportProfile,
                    ExplicitFilePathSource::CallerSelection,
                    fixture.path.clone(),
                    ExplicitOverwriteIntent::PreserveExisting,
                ).expect("test-owned mutated selection");
                let expected_payload_error =
                    ProfileZipPayloadVerificationError::Entry {
                        index,
                        reason: ProfileEntryVerificationError::DigestMismatch,
                    };
                let mismatch = Error::Verification(
                    ProfileImportVerificationError::Payload(
                        expected_payload_error,
                    ),
                );
                assert_eq!(
                    inspect_selected_profile_import(
                        &path, &[], limits(), |_verified| {
                            inspection_called.set(true);
                        },
                    ),
                    Err(mismatch),
                    "member {index} byte {position} bit {bit}",
                );
                assert!(
                    !inspection_called.get(),
                    "forged bytes reached inspection",
                );
                attempted += 1;
            }
        }
    }
    assert_eq!(attempted, (section.len() + asset.len()) * 8);
}

#[test]
fn valid_file_framing_cannot_override_exact_manifest_inventory() {
    let payload = b"entry";
    let empty_root = concat!(
        r#"{"container_version":"atrament.profile/1","#,
        r#""profile_identity":"empty","required_features":[],"#,
        r#""optional_features":[],"entries":[]}"#,
    );
    let with_undeclared =
        OwnedTestFile::new(&zip(empty_root.as_bytes(), payload));
    let with_missing = OwnedTestFile::new(
        &write_stored_profile_zip(
            &[ProfileStoredZipMember {
                name: "manifest.json",
                bytes: root(payload, "[]").as_bytes(),
            }],
            ProfileStoredZipWriteLimits {
                maximum_archive_bytes: 16_384,
                maximum_entries: 3,
                maximum_entry_bytes: 8_192,
                maximum_total_bytes: 12_000,
            },
        )
        .expect("valid ZIP with a missing declared member"),
    );
    let intent = selected(ExplicitFilePurpose::ImportProfile);
    let inspected = Cell::new(false);
    for fixture in [&with_undeclared, &with_missing] {
        assert!(matches!(
            inspect_open_profile_import(
                &intent,
                |max| read_bounded_profile_file(fixture.open(), max),
                &[],
                limits(),
                |_verified| inspected.set(true),
            ),
            Err(Error::Verification(
                ProfileImportVerificationError::Preflight(
                    ProfileImportPreflightError::EntryInventory(_),
                )
            )),
        ));
    }
    assert!(!inspected.get());
}

#[test]
fn declared_member_bytes_obey_import_budget_despite_valid_file_integrity() {
    let payload = b"six123";
    let fixture =
        OwnedTestFile::new(&zip(root(payload, "[]").as_bytes(), payload));
    let intent = selected(ExplicitFilePurpose::ImportProfile);
    let mut config = limits();
    config.import.maximum_entry_bytes = 5;
    let inspected = Cell::new(false);
    assert_eq!(
        inspect_open_profile_import(
            &intent,
            |max| read_bounded_profile_file(fixture.open(), max),
            &[],
            config,
            |_verified| inspected.set(true),
        ),
        Err(Error::Verification(
            ProfileImportVerificationError::Preflight(
                ProfileImportPreflightError::Budget(
                    ProfileImportBudgetError::EntryBytesExceeded {
                        declared: 6,
                        index: 0,
                        limit: 5,
                    },
                ),
            )
        )),
    );
    assert!(!inspected.get());
}
