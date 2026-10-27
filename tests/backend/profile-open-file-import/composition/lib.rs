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
//   - Proves caller path strings never cause this adapter to open files.
// - Usage:
//   - Run Cargo test for the profile open-file-import adapter.
// - Defaults:
//   - Resource, content and file-source failures refuse without callback.
//

//! Real file descriptors exercise bounded, source-attached Import verification.

use std::cell::Cell;
use std::fs::{File, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use atrament_explicit_file_path_intent::{
    ExplicitFilePathSource, ExplicitFilePurpose, ExplicitOverwriteIntent,
    classify_explicit_file_path_intent,
};
use atrament_profile_entry_sha256::sha256_profile_bytes;
use atrament_profile_file_reader::{
    ProfileFileReadError, read_bounded_profile_file,
};
use atrament_profile_import_preflight::ProfileImportBudget;
use atrament_profile_import_verification::ProfileImportVerificationError;
use atrament_profile_json_syntax::{
    ProfileJsonSyntaxError, ProfileJsonSyntaxLimits,
};
use atrament_profile_manifest_json_codec::{
    ProfileManifestJsonError, ProfileManifestJsonLimits,
};
use atrament_profile_open_file_import::{
    ProfileOpenFileImportError as Error, ProfileOpenFileImportLimits,
    inspect_open_profile_import,
};
use atrament_profile_stored_zip_writer::{
    ProfileStoredZipMember, ProfileStoredZipWriteLimits,
    write_stored_profile_zip,
};
use atrament_profile_zip_reader::{
    ProfileZipManifestSyntaxError, ProfileZipReadError, ProfileZipReadLimits,
};

const MEMBER: &str = "sections/writer.json";
static FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct OwnedTestFile {
    path: PathBuf,
}

impl OwnedTestFile {
    fn new(bytes: &[u8]) -> Self {
        let path = std::env::temp_dir().join(format!(
            "atrament-open-profile-import-{}-{}.atrament",
            std::process::id(),
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
        std::fs::remove_file(&self.path).expect("remove owned fixture");
    }
}

fn limits() -> ProfileOpenFileImportLimits {
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
    let payload = b"ok";
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
        File::open(std::env::temp_dir()).expect("OS test directory");
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
    let called = Cell::new(0);
    let malformed = OwnedTestFile::new(b"this is not a zip");
    assert!(matches!(
        inspect_open_profile_import(
            &intent,
            |max| read_bounded_profile_file(malformed.open(), max),
            &[],
            limits(),
            |_verified| called.set(called.get() + 1)
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
            |_verified| called.set(called.get() + 1)
        ),
        Err(Error::RootJson(ProfileZipManifestSyntaxError::Json(
            ProfileJsonSyntaxError::Malformed
        ))),
    ));
    let schema_root = zip(br#"{"not-a-manifest":true}"#, b"data");
    let fixture = OwnedTestFile::new(&schema_root);
    assert!(matches!(
        inspect_open_profile_import(
            &intent,
            |max| read_bounded_profile_file(fixture.open(), max),
            &[],
            limits(),
            |_verified| called.set(called.get() + 1)
        ),
        Err(Error::Manifest(ProfileManifestJsonError::UnknownField(_))),
    ));
    assert_eq!(called.get(), 0);
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
        OwnedTestFile::new(&zip(declared_root.as_bytes(), b"impostor"));
    assert!(matches!(
        inspect_open_profile_import(
            &intent,
            |max| read_bounded_profile_file(replaced.open(), max),
            &[],
            limits(),
            |_verified| { inspected.set(true) }
        ),
        Err(Error::Verification(
            ProfileImportVerificationError::Payload(_)
        )),
    ));
    assert!(!inspected.get());
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
    let payload = b"ok";
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
