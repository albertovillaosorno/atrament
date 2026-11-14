// Copyright:
//   - Copyright © 2026 Alberto Villa Osorno.
// SPDX-License-Identifier:
//   - MIT
// Confidential:
//   - false
// License-File:
//   - LICENSE
//
// Boundary-Contract:
// - Owns:
//   - Disposable, read-only local profile source and path-refusal fixtures.
// - Must-Not:
//   - Grant session authorization, parse JSON or commit production files.
// - Allows:
//   - Inputs: Test-owned directories, symlinks and bounded source bytes.
//   - Outputs: Exact read results or explicit fail-closed errors.
//   - Side effects: Create and clean only disposable test-owned files.
// - Split-When:
//   - Platform-specific source opening requires a new security fixture.
// - Merge-When:
//   - A full local-profile store integration subsumes standalone source tests.
// - Summary:
//   - Exact reads and directory selection cannot silently follow a link.
// - Description:
//   - Exercises the local file adapter independent of HTTP admission.
// - Usage:
//   - cargo test -p atrament_profile_local_file_read
// - Defaults:
//   - Unavailable platform policies refuse before returning any bytes.
//

//! Read-only local-profile file adapter integration tests.

#![expect(
    clippy::create_dir,
    clippy::expect_used,
    reason = "fail-fast, test-owned filesystem fixtures"
)]

// Exercise the same private error classifier compiled by the production crate.
// jig-ignore-next-line: Stable test source path cannot be split in Rust.
#[path = "../../../../src/backend/profile-local-file-read/adapter-outbound/lib.rs"]
pub mod read_error_mapping;

#[cfg(target_os = "linux")]
use std::fs::{self, File};
#[cfg(target_os = "linux")]
use std::io::{ErrorKind, Write as _};
#[cfg(target_os = "linux")]
use std::os::unix::fs::symlink;
use std::path::Path;
#[cfg(target_os = "linux")]
use std::path::PathBuf;
#[cfg(target_os = "linux")]
use std::process;
#[cfg(target_os = "linux")]
use std::sync::atomic::{AtomicU64, Ordering};

use atrament_profile_file_reader::ProfileFileReadError;
use atrament_profile_file_reader as _;
use atrament_profile_selected_file_open as _;

use atrament_profile_local_file_read::{
    LocalProfileFileReadError, canonical_local_profile_directory,
    read_local_profile_bytes,
};

#[cfg(target_os = "linux")]
static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

#[cfg(target_os = "linux")]
struct Fixture(PathBuf);

#[cfg(target_os = "linux")]
impl Fixture {
    fn new() -> Self {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.temp");
        fs::create_dir_all(&root).expect("project scratch directory");
        let path = root.join(format!(
            "profile-local-read-{}-{}",
            process::id(),
            NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed),
        ));
        fs::create_dir(&path).expect("new test-owned directory");
        Self(fs::canonicalize(path).expect("canonical test directory"))
    }

    fn write(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.0.join(name);
        let mut file = File::create(&path).expect("new test-owned file");
        file.write_all(bytes).expect("test-owned bytes");
        path
    }
}

#[cfg(target_os = "linux")]
#[expect(
    clippy::missing_trait_methods,
    reason = "unstable pin_drop cannot be implemented on stable Rust"
)]
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).expect("clean test-owned directory");
    }
}

#[cfg(target_os = "linux")]
#[test]
fn real_directory_and_exact_source_are_read_only() {
    let fixture = Fixture::new();
    let selected = canonical_local_profile_directory(&fixture.0)
        .expect("real directory accepted");
    assert!(selected.is_absolute());
    let path = fixture.write("notebook-example.json", b"{\"valid\":true}");
    assert_eq!(
        read_local_profile_bytes(&path, 14).expect("exact ceiling accepted"),
        b"{\"valid\":true}",
    );
    assert_eq!(
        read_local_profile_bytes(&path, 13),
        Err(LocalProfileFileReadError::InvalidInput),
    );
    assert_eq!(
        fs::read(&path).expect("file remains intact"),
        b"{\"valid\":true}"
    );
    assert_eq!(
        read_local_profile_bytes(&fixture.0.join("missing.json"), 100),
        Err(LocalProfileFileReadError::NotFound),
    );
}

#[cfg(target_os = "linux")]
#[test]
fn nonregular_linked_and_parent_linked_sources_are_refused() {
    let fixture = Fixture::new();
    let actual = fixture.write("real.json", b"private");
    let linked = fixture.0.join("alias.json");
    symlink(&actual, &linked).expect("test-owned leaf link");
    assert_eq!(
        read_local_profile_bytes(&linked, 100),
        Err(LocalProfileFileReadError::InvalidInput),
    );
    assert_eq!(
        canonical_local_profile_directory(&linked)
            .expect_err("linked file is not a directory")
            .kind(),
        ErrorKind::InvalidInput,
    );
    let directory_link = fixture.0.join("linked-dir");
    symlink(&fixture.0, &directory_link).expect("test-owned directory link");
    assert_eq!(
        canonical_local_profile_directory(&directory_link)
            .expect_err("linked directory cannot be a selected leaf")
            .kind(),
        ErrorKind::InvalidInput,
    );
    assert_eq!(
        read_local_profile_bytes(&directory_link.join("real.json"), 100),
        Err(LocalProfileFileReadError::InvalidInput),
    );
    assert_eq!(
        read_local_profile_bytes(&fixture.0, 100),
        Err(LocalProfileFileReadError::InvalidInput),
    );
    assert_eq!(
        read_local_profile_bytes(&fixture.0.join("../missing.json"), 100),
        Err(LocalProfileFileReadError::InvalidInput),
    );
}

#[cfg(target_os = "linux")]
#[test]
fn transient_selected_directory_symlink_does_not_redirect_canonical_root() {
    let fixture = Fixture::new();
    let chosen = fixture.0.join("selected");
    let parked = fixture.0.join("parked");
    let foreign = fixture.0.join("foreign");
    fs::create_dir(&chosen).expect("original directory");
    fs::create_dir(&foreign).expect("foreign directory");

    let selected = read_error_mapping::
        canonical_local_profile_directory_with(&chosen, |path| {
            fs::rename(&chosen, &parked).expect("park selected inode");
            symlink(&foreign, &chosen).expect("substitute selected link");
            let redirected = fs::canonicalize(path);
            fs::remove_file(&chosen).expect("remove substitute");
            fs::rename(&parked, &chosen).expect("restore selected inode");
            redirected
        });
    assert_eq!(
        selected.expect_err("temporary path redirection must refuse").kind(),
        ErrorKind::InvalidInput,
    );
    assert_eq!(
        canonical_local_profile_directory(&chosen)
            .expect("restored originally selected directory"),
        fs::canonicalize(&chosen).expect("original physical path"),
    );
}

#[test]
fn read_error_categories_preserve_host_failures_and_input_refusals() {
    use std::io::ErrorKind;

    for error in [
        ProfileFileReadError::AllocationFailed,
        ProfileFileReadError::FileIo(ErrorKind::PermissionDenied),
        ProfileFileReadError::FileIo(ErrorKind::UnexpectedEof),
    ] {
        assert_eq!(
            read_error_mapping::classify_bounded_profile_read_error(error),
            read_error_mapping::LocalProfileFileReadError::FileIo,
            "host error {error:?} must not be attributed to caller input",
        );
    }
    for error in [
        ProfileFileReadError::FileLengthChanged,
        ProfileFileReadError::FileMetadataChanged,
        ProfileFileReadError::FileTooLarge,
        ProfileFileReadError::NotRegularFile,
    ] {
        assert_eq!(
            read_error_mapping::classify_bounded_profile_read_error(error),
            read_error_mapping::LocalProfileFileReadError::InvalidInput,
            "bad source {error:?} must refuse without file effects",
        );
    }
}

#[test]
fn invalid_directory_cannot_be_admitted_as_root() {
    let result = canonical_local_profile_directory(Path::new(""));
    assert!(result.is_err());
}
