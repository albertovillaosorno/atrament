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
//   - Real-file atomic profile commit and failure-preservation regressions.
// - Must-Not:
//   - Admit revision/selection policy or mutate non-test-owned files.
// - Allows:
//   - Inputs: Test-owned paths, complete bytes and exact overwrite policy.
//   - Outputs: Assertions over target content and temp cleanup.
//   - Side effects: Test-owned directory and file creation/removal.
// - Split-When:
//   - Other platforms gain equivalent secure file operations.
// - Merge-When:
//   - End-to-end Export owns a host-admitted commit fixture.
// - Summary:
//   - Exact new and replacement commits preserve failures without temp leaks.
// - Description:
//   - Tests no-overwrite races, symlinks and bounded artifacts on Linux.
// - Usage:
//   - cargo test -p atrament_profile_file_commit
// - Defaults:
//   - Outside Linux, refusal is expected without a weak fallback.
//

//! Real filesystem checks for the low-level, non-authorizing Export adapter.

#![expect(clippy::expect_used, reason = "test-owned fixture")]

#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use getrandom as _;
    use nix as _;
    use std::env;
    use std::ffi::OsString;
    use std::fs::{self, OpenOptions};
    use std::io::{self, Read as _, Write as _};
    use std::os::unix::fs::{PermissionsExt as _, symlink};
    use std::path::{Path, PathBuf};
    use std::process::{self, Command, Stdio};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::thread;
    use std::time::Duration;

    static NEXT: AtomicU64 = AtomicU64::new(0);

    const CRASH_STAGE_TARGET: &str = "ATRAMENT_TEST_ANONYMOUS_STAGE_TARGET";
    const CRASH_STAGE_REPLACE: &str = "ATRAMENT_TEST_ANONYMOUS_REPLACE";
    const CRASH_STAGE_TEST: &str =
        "linux::forced_death_during_write_leaves_no_named_stage";

    struct OwnedDir(PathBuf);
    #[expect(
        clippy::arbitrary_source_item_ordering,
        reason = "fixture setup and observation grouped for readability"
    )]
    impl OwnedDir {
        #[expect(
            clippy::create_dir,
            reason = "must fail if unique fixture directory already exists"
        )]
        fn new() -> Self {
            let root = env::temp_dir().join(format!(
                "atrament-commit-{}-{}",
                process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed),
            ));
            fs::create_dir(&root).expect("unique test-owned directory");
            Self(root)
        }
        fn target(&self) -> PathBuf {
            self.0.join("chosen.atrament")
        }
        fn entries(&self) -> Vec<String> {
            fs::read_dir(&self.0)
                .expect("read test directory")
                .map(|entry| {
                    entry
                        .expect("read entry")
                        .file_name()
                        .to_string_lossy()
                        .into_owned()
                })
                .collect()
        }
    }
    #[expect(
        clippy::missing_trait_methods,
        reason = "Drop provides only its required cleanup method"
    )]
    impl Drop for OwnedDir {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).expect("remove only owned fixture");
        }
    }

    #[test]
    fn creates_exact_selected_bytes_without_intermediate_leaks() {
        let dir = OwnedDir::new();
        let target = dir.target();
        let payload = vec![0x93u8; 131_072];
        let evidence = commit_profile_file(
            &target,
            &payload,
            payload.len(),
            Mode::PreserveExisting,
        )
        .expect("no-existing-target commit");
        assert_eq!(evidence.byte_length, payload.len());
        assert_eq!(fs::read(&target).expect("read target"), payload);
        assert_eq!(dir.entries(), vec![String::from("chosen.atrament")]);
        assert_eq!(
            fs::metadata(&target)
                .expect("file metadata")
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }

    #[test]
    fn forced_death_during_write_leaves_no_named_stage() {
        // The child blocks after its first 64 KiB chunk, while the file is
        // still incomplete. SIGKILL bypasses all Rust destructor cleanup.
        if let Some(path) = env::var_os(CRASH_STAGE_TARGET) {
            let target = PathBuf::from(path);
            let mode = if env::var_os(CRASH_STAGE_REPLACE).is_some() {
                Mode::ReplaceExisting
            } else {
                Mode::PreserveExisting
            };
            let payload = vec![0x83u8; 200_000];
            let mut observations = 0usize;
            let result = commit_profile_file_cancellable(
                &target,
                &payload,
                payload.len(),
                mode,
                || {
                    observations += 1;
                    if observations == 3 {
                        fs::write(target.with_extension("ready"), b"writing")
                            .expect("notify parent after first chunk");
                        let mut release = [0u8; 1];
                        let _ignored = io::stdin().read_exact(&mut release);
                    }
                    false
                },
            );
            assert!(
                result.is_err(),
                "fixture must not reach a successful file commit",
            );
        }

        for mode in [Mode::PreserveExisting, Mode::ReplaceExisting] {
            let dir = OwnedDir::new();
            let target = dir.target();
            let marker = target.with_extension("ready");
            if mode == Mode::ReplaceExisting {
                fs::write(&target, b"original").expect("seed target");
            }
            let executable = env::current_exe().expect("test executable");
            let mut command = Command::new(executable);
            let _configured = command
                .arg("--exact")
                .arg(CRASH_STAGE_TEST)
                .env(CRASH_STAGE_TARGET, &target)
                .env_remove(CRASH_STAGE_REPLACE)
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null());
            if mode == Mode::ReplaceExisting {
                let _replace_flag = command.env(CRASH_STAGE_REPLACE, "yes");
            }
            let mut child = command.spawn().expect("spawn midwrite process");
            let mut writing = false;
            for _ in 0..200u16 {
                if marker.exists() {
                    writing = true;
                    break;
                }
                if child.try_wait().expect("check child status").is_some() {
                    break;
                }
                thread::sleep(Duration::from_millis(50));
            }
            if writing {
                child.kill().expect("kill midwrite child");
            } else if child.try_wait().expect("child liveness").is_none() {
                child.kill().expect("stop timed-out child");
            }
            let status = child.wait().expect("reap killed child");
            assert!(writing, "child did not stage its first data chunk");
            assert!(!status.success(), "child must die without unwinding");
            fs::remove_file(&marker).expect("remove test-only marker");
            if mode == Mode::ReplaceExisting {
                assert_eq!(
                    dir.entries(),
                    vec![String::from("chosen.atrament")],
                );
                assert_eq!(fs::read(&target).expect("old target"), b"original");
            } else {
                assert!(dir.entries().is_empty());
                assert!(!target.exists());
            }
        }
    }

    #[test]
    fn existing_target_refuses_preserve_without_overwrite_or_temp_leak() {
        let dir = OwnedDir::new();
        let target = dir.target();
        fs::write(&target, b"original").expect("seed original");
        assert_eq!(
            commit_profile_file(
                &target,
                b"changed",
                20,
                Mode::PreserveExisting
            ),
            Err(Error::TargetExists)
        );
        assert_eq!(fs::read(&target).expect("original survived"), b"original");
        assert_eq!(dir.entries(), vec![String::from("chosen.atrament")]);
    }

    #[test]
    fn authorized_replacement_is_atomic_and_leaves_no_temp() {
        let dir = OwnedDir::new();
        let target = dir.target();
        fs::write(&target, b"original").expect("seed original");
        let replacement = b"new complete artifact";
        assert_eq!(
            commit_profile_file(
                &target,
                replacement,
                100,
                Mode::ReplaceExisting
            )
            .expect("explicit replacement")
            .byte_length,
            replacement.len()
        );
        assert_eq!(fs::read(&target).expect("read replacement"), replacement);
        assert_eq!(dir.entries(), vec![String::from("chosen.atrament")]);
    }

    #[test]
    fn refused_oversize_preserves_target_without_creating_intermediate() {
        let dir = OwnedDir::new();
        let target = dir.target();
        fs::write(&target, b"original").expect("seed original");
        assert_eq!(
            commit_profile_file(&target, b"12345", 4, Mode::ReplaceExisting),
            Err(Error::ArtifactTooLarge)
        );
        assert_eq!(fs::read(&target).expect("original survived"), b"original");
        assert_eq!(dir.entries(), vec![String::from("chosen.atrament")]);
    }

    #[test]
    fn concurrent_preserve_writers_commit_exactly_one_complete_artifact() {
        use std::sync::{Arc, Barrier};
        use std::thread;
        let dir = OwnedDir::new();
        let target = dir.target();
        let barrier = Arc::new(Barrier::new(2));
        let outcomes = thread::scope(|scope| {
            let handles =
                [b"first complete".as_slice(), b"other complete".as_slice()]
                    .map(|payload| {
                        let wait = Arc::clone(&barrier);
                        let selected = target.clone();
                        scope.spawn(move || {
                            let _generation = wait.wait();
                            commit_profile_file(
                                &selected,
                                payload,
                                64,
                                Mode::PreserveExisting,
                            )
                        })
                    });
            handles.map(|handle| handle.join().expect("writer thread"))
        });
        assert_eq!(
            outcomes.iter().filter(|outcome| outcome.is_ok()).count(),
            1
        );
        assert_eq!(
            outcomes
                .iter()
                .filter(|outcome| **outcome == Err(Error::TargetExists))
                .count(),
            1,
        );
        let actual = fs::read(&target).expect("one committed target");
        assert!(actual == b"first complete" || actual == b"other complete");
        assert_eq!(dir.entries(), vec![String::from("chosen.atrament")]);
    }

    #[test]
    fn refuses_existing_symlink_without_following_it() {
        let dir = OwnedDir::new();
        let separate = dir.0.join("actual");
        fs::write(&separate, b"secret").expect("seed linked file");
        let target = dir.target();
        symlink(&separate, &target).expect("link selected target");
        assert_eq!(
            commit_profile_file(&target, b"new", 3, Mode::ReplaceExisting),
            Err(Error::InvalidExistingTarget)
        );
        assert_eq!(
            fs::read(&separate).expect("linked file untouched"),
            b"secret"
        );
        assert_eq!(dir.entries().len(), 2);
        assert_eq!(
            commit_profile_file(&target, b"new", 3, Mode::PreserveExisting),
            Err(Error::TargetExists)
        );
        assert_eq!(
            fs::read(&separate).expect("linked file untouched"),
            b"secret"
        );
        assert_eq!(dir.entries().len(), 2);
    }

    #[test]
    fn refuses_symlink_in_parent_and_parent_escape() {
        let dir = OwnedDir::new();
        let linked_parent = dir.0.join("alias");
        symlink(&dir.0, &linked_parent).expect("create parent link");
        let linked_target = linked_parent.join("chosen.atrament");
        assert!(
            commit_profile_file(
                &linked_target,
                b"a",
                1,
                Mode::PreserveExisting
            )
            .is_err()
        );
        assert!(!dir.target().exists());
        let escape = dir.0.join("..").join("escape.atrament");
        assert_eq!(
            commit_profile_file(&escape, b"a", 1, Mode::PreserveExisting),
            Err(Error::InvalidTarget)
        );
        assert!(!dir.target().exists());
    }

    #[test]
    fn refuses_directory_target_and_missing_parent() {
        let dir = OwnedDir::new();
        let target = dir.0.join("directory");
        fs::create_dir_all(&target).expect("create fixture directory");
        assert_eq!(
            commit_profile_file(&target, b"x", 1, Mode::ReplaceExisting),
            Err(Error::InvalidExistingTarget)
        );
        assert!(
            commit_profile_file(
                &dir.0.join("missing/nope.atrament"),
                b"x",
                1,
                Mode::PreserveExisting
            )
            .is_err()
        );
        assert_eq!(dir.entries(), vec![String::from("directory")]);
    }

    #[test]
    fn supports_non_utf8_target_without_normalizing_bytes() {
        use std::os::unix::ffi::OsStringExt as _;
        let dir = OwnedDir::new();
        let target = dir
            .0
            .join(OsString::from_vec(b"raw-\xff.atrament".to_vec()));
        assert_eq!(
            commit_profile_file(&target, b"valid", 5, Mode::PreserveExisting)
                .expect("opaque name is preserved")
                .byte_length,
            5
        );
        assert_eq!(fs::read(&target).expect("opaque target"), b"valid");
        assert_eq!(dir.entries().len(), 1);
    }

    #[test]
    fn refuses_directory_markers_and_root() {
        for target in ["", "/", "not-a-file/", "not-a-file/.", "not-a-file/.."]
        {
            assert_eq!(
                commit_profile_file(
                    Path::new(target),
                    b"x",
                    1,
                    Mode::PreserveExisting
                ),
                Err(Error::InvalidTarget)
            );
        }
    }

    #[test]
    fn cancellation_preflight_refuses_without_file_io() {
        let dir = OwnedDir::new();
        let target = dir.target();
        let mut observations: usize = 0;
        assert_eq!(
            commit_profile_file_cancellable(
                &target,
                b"data",
                4,
                Mode::PreserveExisting,
                || {
                    observations += 1;
                    true
                },
            ),
            Err(Error::CancelledBeforeCommit)
        );
        assert_eq!(observations, 1);
        assert!(dir.entries().is_empty());
        assert_eq!(
            commit_profile_file_cancellable(
                &target,
                b"oversize",
                4,
                Mode::PreserveExisting,
                || {
                    observations += 1;
                    true
                },
            ),
            Err(Error::ArtifactTooLarge)
        );
        assert_eq!(
            commit_profile_file_cancellable(
                Path::new("../escape.atrament"),
                b"data",
                4,
                Mode::PreserveExisting,
                || {
                    observations += 1;
                    true
                },
            ),
            Err(Error::InvalidTarget)
        );
        assert_eq!(observations, 1);
        assert!(dir.entries().is_empty());
    }

    #[test]
    fn cancellation_after_file_sync_preserves_existing_target_and_cleans_temp()
    {
        let dir = OwnedDir::new();
        let target = dir.target();
        fs::write(&target, b"original").expect("original file");
        let mut observations: usize = 0;
        assert_eq!(
            commit_profile_file_cancellable(
                &target,
                b"replacement",
                11,
                Mode::ReplaceExisting,
                || {
                    observations += 1;
                    observations == 3
                },
            ),
            Err(Error::CancelledBeforeCommit)
        );
        assert_eq!(observations, 3);
        assert_eq!(fs::read(&target).expect("original remains"), b"original");
        assert_eq!(dir.entries(), vec![String::from("chosen.atrament")]);
    }

    #[test]
    fn cancellation_midwrite_discards_partial_staging_and_preserves_target() {
        let dir = OwnedDir::new();
        let target = dir.target();
        fs::write(&target, b"original").expect("original file");
        let mut observations: usize = 0;
        let payload = vec![0x93u8; 200_000];
        assert_eq!(
            commit_profile_file_cancellable(
                &target,
                &payload,
                payload.len(),
                Mode::ReplaceExisting,
                || {
                    observations += 1;
                    observations == 3
                },
            ),
            Err(Error::CancelledBeforeCommit)
        );
        assert_eq!(observations, 3);
        assert_eq!(fs::read(&target).expect("original remains"), b"original");
        assert_eq!(dir.entries(), vec![String::from("chosen.atrament")]);
    }

    #[test]
    fn cancellation_after_final_observation_cannot_revoke_committed_bytes() {
        let dir = OwnedDir::new();
        let target = dir.target();
        let mut observations: usize = 0;
        let payload = vec![0x47u8; 131_072];
        let evidence = commit_profile_file_cancellable(
            &target,
            &payload,
            payload.len(),
            Mode::PreserveExisting,
            || {
                observations += 1;
                false
            },
        )
        .expect("complete commit");
        // 1 preflight, 2 data chunks and 1 pre-rename check; none after.
        assert_eq!(observations, 4);
        assert_eq!(evidence.byte_length, payload.len());
        assert_eq!(fs::read(&target).expect("committed file"), payload);
        assert_eq!(dir.entries(), vec![String::from("chosen.atrament")]);
    }

    #[test]
    fn preserve_conflict_does_not_modify_original_open_handle() {
        let dir = OwnedDir::new();
        let target = dir.target();
        let mut original = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&target)
            .expect("create original");
        original.write_all(b"before").expect("write original");
        assert_eq!(
            commit_profile_file(&target, b"after", 5, Mode::PreserveExisting),
            Err(Error::TargetExists)
        );
        assert_eq!(fs::read(&target).expect("read original"), b"before");
        assert_eq!(dir.entries().len(), 1);
    }
}

use atrament_profile_file_commit::{
    ProfileFileCommitDisposition as Mode, ProfileFileCommitError as Error,
    commit_profile_file, commit_profile_file_cancellable,
};

#[cfg(not(target_os = "linux"))]
use std::path::Path;

#[cfg(not(target_os = "linux"))]
#[test]
fn other_platforms_fail_closed() {
    assert_eq!(
        commit_profile_file(
            Path::new("profile.atrament"),
            b"data",
            4,
            Mode::PreserveExisting
        ),
        Err(Error::UnsupportedPlatform)
    );
}
