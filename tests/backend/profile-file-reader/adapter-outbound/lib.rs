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
//   - File transport, cursor, budget and metadata-drift regressions.
// - Must-Not:
//   - Admit ZIP, trust a selected path, or change accepted session state.
// - Allows:
//   - Inputs: Unique temporary regular files owned by the test process.
//   - Outputs: Exact file-byte identity and source-error assertions.
//   - Side effects: Write and remove only fixtures created by these tests.
// - Split-When:
//   - File opening/provenance policy becomes a trusted host adapter.
// - Merge-When:
//   - Complete file Import tests own this same bounded transport proof.
// - Summary:
//   - Unchanged file length cannot override observed metadata mutation.
// - Description:
//   - A caller's initial file cursor never controls Import byte identity.
// - Usage:
//   - Run with Cargo workspace tests and strict Clippy.
// - Defaults:
//   - Exact-length snapshots with bounded before/after metadata checks.
//

//! File transport tests without any profile content or path authority.

#![expect(
    clippy::expect_used,
    clippy::missing_trait_methods,
    reason = "fail-fast, uniquely owned filesystem test fixtures",
)]

// Compile the same private metadata comparator into the test harness to
// exercise deterministic before/after filesystem mutation evidence. Normal
// transport tests below still call the linked production crate.
#[cfg(unix)]
#[path = "../../../../src/backend/profile-file-reader/adapter-outbound/lib.rs"]
pub mod metadata_gate;

use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{Seek as _, SeekFrom, Write as _};
use std::path::PathBuf;
use std::process;
use std::sync::atomic::{AtomicU64, Ordering};

use atrament_profile_file_reader::{
    ProfileFileReadError, read_bounded_profile_file,
};

static FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct OwnedTempFile {
    path: PathBuf,
}

impl OwnedTempFile {
    fn new(data: &[u8]) -> Self {
        let path = env::temp_dir().join(format!(
            "atrament-profile-file-reader-{}-{}",
            process::id(),
            FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed),
        ));
        let mut source = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)
            .expect("create unique OS test fixture");
        source.write_all(data).expect("test fixture contents");
        Self { path }
    }

    fn open(&self) -> File {
        File::open(&self.path).expect("open owned test fixture")
    }
}

impl Drop for OwnedTempFile {
    fn drop(&mut self) {
        fs::remove_file(&self.path).expect("remove owned test fixture");
    }
}

#[test]
fn bounded_file_source_rewinds_to_zero_and_preserves_exact_bytes() {
    let contents = b"\0b\xff\r\nend";
    let fixture = OwnedTempFile::new(contents);
    let mut opened = fixture.open();
    let _position = opened
        .seek(SeekFrom::End(0))
        .expect("position cursor beyond payload");
    let verified = read_bounded_profile_file(opened, contents.len())
        .expect("exact EOF source");
    assert_eq!(verified.bytes(), contents);
}

#[test]
fn bounded_private_import_snapshot_is_redacted_from_debug() {
    let contents = b"private-import-source-bytes-marker";
    let fixture = OwnedTempFile::new(contents);
    let snapshot = read_bounded_profile_file(fixture.open(), contents.len())
        .expect("read private fixture");
    assert_eq!(snapshot.bytes(), contents);
    let debug = format!("{snapshot:?}");
    assert!(debug.contains("ProfileOpenedFileBytes"));
    assert!(!debug.contains("bytes:"));
    assert!(!debug.contains(&format!("{contents:?}")));
}

#[test]
fn transport_ceiling_refuses_one_byte_below_actual_file_length() {
    let fixture = OwnedTempFile::new(b"123456");
    assert_eq!(
        read_bounded_profile_file(fixture.open(), 5)
            .expect_err("source exceeds caller's bound"),
        ProfileFileReadError::FileTooLarge,
    );
    assert_eq!(
        read_bounded_profile_file(fixture.open(), 6)
            .expect("exact size is permitted")
            .bytes(),
        b"123456",
    );
    let empty = OwnedTempFile::new(b"");
    assert_eq!(
        read_bounded_profile_file(empty.open(), 0)
            .expect("zero-byte source remains a real EOF")
            .bytes(),
        b"",
    );
}

#[cfg(unix)]
#[test]
fn read_gate_accepts_unchanged_open_file_metadata() {
    let fixture = OwnedTempFile::new(b"stable-private-bytes");
    let source = fixture.open();
    let before = source.metadata().expect("initial opened file metadata");
    let after = source.metadata().expect("unchanged file metadata");
    assert!(metadata_gate::source_metadata_unchanged(&before, &after));
    assert_eq!(
        metadata_gate::read_bounded_profile_file(fixture.open(), 20)
            .expect("compiled production reader also accepts stable source")
            .bytes(),
        b"stable-private-bytes",
    );
    assert_eq!(
        read_bounded_profile_file(source, 20)
            .expect("stable regular file accepted")
            .bytes(),
        b"stable-private-bytes",
    );
}

#[cfg(unix)]
#[test]
fn read_gate_refuses_same_length_in_place_write_with_changed_metadata() {
    use std::fs::FileTimes;
    use std::os::unix::fs::MetadataExt as _;
    use std::time::{Duration, SystemTime};

    let fixture = OwnedTempFile::new(b"before-1234");
    let source = fixture.open();
    let before = source.metadata().expect("initial metadata");
    fs::write(&fixture.path, b"after--5678")
        .expect("same-length in-place rewrite");
    source.set_times(FileTimes::new().set_modified(
        SystemTime::UNIX_EPOCH + Duration::from_secs(1),
    )).expect("force a distinct mtime without altering the file length");
    let after = source.metadata().expect("post-rewrite metadata");
    assert_eq!(before.len(), after.len());
    assert_eq!(before.ino(), after.ino());
    assert_ne!(before.modified().ok(), after.modified().ok());
    assert!(!metadata_gate::source_metadata_unchanged(&before, &after));
}

#[cfg(unix)]
#[test]
fn read_gate_refuses_same_length_different_inode_evidence() {
    use std::os::unix::fs::MetadataExt as _;

    let first = OwnedTempFile::new(b"private1");
    let second = OwnedTempFile::new(b"private2");
    let first_metadata = first.open().metadata().expect("first inode");
    let second_metadata = second.open().metadata().expect("second inode");
    assert_eq!(first_metadata.len(), second_metadata.len());
    assert_ne!(first_metadata.ino(), second_metadata.ino());
    assert!(!metadata_gate::source_metadata_unchanged(
        &first_metadata, &second_metadata,
    ));
}

#[cfg(target_os = "linux")]
#[test]
fn read_gate_rejects_rewrite_with_restored_mtime() {
    use std::fs::FileTimes;
    use std::os::unix::fs::MetadataExt as _;
    use std::thread;
    use std::time::Duration;

    let fixture = OwnedTempFile::new(b"private-preimage-47");
    let source = fixture.open();
    let before = source.metadata().expect("pre-rewrite metadata");
    let previous_modified = before.modified().expect("regular file mtime");
    // Ensure the mutation is distinguishable at the test filesystem's
    // nanosecond timestamp resolution before restoring the prior mtime.
    thread::sleep(Duration::from_millis(3));
    fs::write(&fixture.path, b"private-postimage47")
        .expect("same-size direct rewrite");
    source.set_times(FileTimes::new().set_modified(previous_modified))
        .expect("restore original mtime");
    let after = source.metadata().expect("post-rewrite metadata");
    assert_eq!(before.len(), after.len());
    assert_eq!(before.modified().ok(), after.modified().ok());
    assert_ne!(
        (before.ctime(), before.ctime_nsec()),
        (after.ctime(), after.ctime_nsec()),
    );
    assert!(!metadata_gate::source_metadata_unchanged(&before, &after));
}
