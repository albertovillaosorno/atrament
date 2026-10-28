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
//   - Real-path Linux symlink, provenance and end-to-end Import regressions.
// - Must-Not:
//   - Infer that a classified source proves an actual caller gesture.
// - Allows:
//   - Inputs: Deliberate test-owned files, directories and symbolic links.
//   - Outputs: Source descriptor and source-attached manifest verification.
//   - Side effects: Create/clean test-owned paths only.
// - Split-When:
//   - Windows or other Unix secure-open implementations need OS fixtures.
// - Merge-When:
//   - Full trusted host Import integration includes selected open and read.
// - Summary:
//   - Open cannot silently follow links and verified bytes come from it.
// - Description:
//   - A swapped pathname cannot substitute the already opened descriptor.
// - Usage:
//   - Run cargo test -p atrament_profile_selected_file_open.
// - Defaults:
//   - Linux tests require openat2; unsupported kernels refuse closed.
//

//! Selected-file descriptor policy and complete on-disk profile verification.

#[cfg(target_os = "linux")]
use nix as _;

#[cfg(target_os = "linux")]
use std::fs::{self, File};
#[cfg(target_os = "linux")]
use std::io::{Read as _, Write as _};
use std::path::PathBuf;
#[cfg(target_os = "linux")]
use std::sync::atomic::{AtomicU64, Ordering};

use atrament_profile_selected_file_open::{
    ProfileSelectedFileOpenError as Error, open_selected_profile_file,
};

#[cfg(target_os = "linux")]
static SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[cfg(target_os = "linux")]
struct OwnedTestDirectory(PathBuf);

#[cfg(target_os = "linux")]
impl OwnedTestDirectory {
    fn new(in_cwd: bool) -> Self {
        let parent = if in_cwd {
            std::env::current_dir().expect("test cwd")
        } else {
            std::env::temp_dir()
        };
        let path = parent.join(format!(
            "atrament-selected-open-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed),
        ));
        fs::create_dir(&path).expect("unique session test directory");
        Self(path)
    }

    fn write(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.0.join(name);
        let mut file = File::create(&path).expect("create owned test file");
        file.write_all(bytes).expect("write test bytes");
        path
    }
}

#[cfg(target_os = "linux")]
impl Drop for OwnedTestDirectory {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).expect("remove only owned test directory");
    }
}

#[test]
fn parent_traversal_refuses_before_open() {
    let escape = PathBuf::from("local/../missing.atrament");
    assert_eq!(
        open_selected_profile_file(&escape)
            .expect_err("parent component refuses"),
        Error::ParentTraversal,
    );
}

#[cfg(target_os = "linux")]
#[test]
fn symlinked_file_and_parent_directory_are_both_forbidden() {
    use std::os::unix::fs::symlink;

    let owned = OwnedTestDirectory::new(false);
    let real = owned.write("real.atrament", b"untrusted content");
    let leaf = owned.0.join("alias.atrament");
    symlink(&real, &leaf).expect("leaf symlink");
    assert_eq!(
        open_selected_profile_file(&leaf)
            .expect_err("must not follow final link"),
        Error::SymlinkForbidden,
    );
    let nested = owned.0.join("nested");
    fs::create_dir(&nested).expect("nested directory");
    let _child = owned.write("nested/good.atrament", b"real data");
    let alias = owned.0.join("linked-parent");
    symlink(&nested, &alias).expect("parent symlink");
    assert_eq!(
        open_selected_profile_file(&alias.join("good.atrament"))
            .expect_err("must not follow directory component"),
        Error::SymlinkForbidden,
    );
}

#[cfg(target_os = "linux")]
#[test]
fn relative_path_is_resolved_under_cwd_without_link_traversal() {
    let owned = OwnedTestDirectory::new(true);
    let _source = owned.write("notes.atrament", b"literal file data");
    let relative = owned
        .0
        .strip_prefix(std::env::current_dir().expect("test cwd"))
        .expect("test directory under cwd")
        .join("notes.atrament");
    let file =
        open_selected_profile_file(&relative).expect("relative path under cwd");
    let mut source = file;
    let mut bytes = Vec::new();
    let _count = source.read_to_end(&mut bytes).expect("read source");
    assert_eq!(bytes, b"literal file data");
}

#[cfg(target_os = "linux")]
#[test]
fn selected_directory_and_missing_file_are_not_admitted() {
    let owned = OwnedTestDirectory::new(false);
    assert_eq!(
        open_selected_profile_file(&owned.0).expect_err("selected directory"),
        Error::NotRegularFile,
    );
    assert_eq!(
        open_selected_profile_file(&owned.0.join("absent"))
            .expect_err("source does not exist"),
        Error::FileIo(std::io::ErrorKind::NotFound),
    );
}

#[cfg(target_os = "linux")]
#[test]
fn already_opened_descriptor_is_not_redirected_by_name_replacement() {
    let owned = OwnedTestDirectory::new(false);
    let path = owned.write("selected.atrament", b"original bytes");
    let opened =
        open_selected_profile_file(&path).expect("selected original source");
    fs::rename(&path, owned.0.join("moved.atrament")).expect("move old name");
    let _replacement = owned.write("selected.atrament", b"replaced bytes");
    let mut source = opened;
    let mut bytes = Vec::new();
    let _count = source
        .read_to_end(&mut bytes)
        .expect("read original descriptor");
    assert_eq!(bytes, b"original bytes");
}

#[cfg(not(target_os = "linux"))]
#[test]
fn platform_without_verified_open_policy_refuses() {
    let selected = PathBuf::from("profile.atrament");
    assert_eq!(
        open_selected_profile_file(&selected)
            .expect_err("no insecure fallback"),
        Error::UnsupportedOpenPolicy,
    );
}
