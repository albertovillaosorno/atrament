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
//   - Independent real-file transport, cursor and budget regressions.
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
//   - File metadata is a resource ceiling, not evidence of complete data.
// - Description:
//   - A caller's initial file cursor never controls Import byte identity.
// - Usage:
//   - Run with Cargo workspace tests and strict Clippy.
// - Defaults:
//   - Exact-length snapshots only; rejects oversized metadata before copy.
//

//! File transport tests without any profile content or path authority.

use std::fs::{File, OpenOptions};
use std::io::{Seek as _, SeekFrom, Write as _};
use std::path::PathBuf;
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
        let path = std::env::temp_dir().join(format!(
            "atrament-profile-file-reader-{}-{}",
            std::process::id(),
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
        std::fs::remove_file(&self.path).expect("remove owned test fixture");
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
