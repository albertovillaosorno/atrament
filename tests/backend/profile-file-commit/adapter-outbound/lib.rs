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
    use nix::libc::SIGKILL;
    use nix::sys::stat::{Mode as FileMode, umask};
    use std::env;
    use std::ffi::OsString;
    use std::fs::{self, OpenOptions};
    use std::io::{self, Read as _, Write as _};
    use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _, symlink};
    use std::os::unix::process::ExitStatusExt as _;
    use std::path::{Path, PathBuf};
    use std::process::{self, Command, Stdio};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::thread;
    use std::time::Duration;

    static NEXT: AtomicU64 = AtomicU64::new(0);

    const CRASH_STAGE_TARGET: &str = "ATRAMENT_TEST_ANONYMOUS_STAGE_TARGET";
    const CRASH_STAGE_REPLACE: &str = "ATRAMENT_TEST_ANONYMOUS_REPLACE";
    const CRASH_STAGE_AFTER_SYNC: &str = "ATRAMENT_TEST_AFTER_STAGE_SYNC";
    const CRASH_STAGE_TEST: &str =
        "linux::forced_death_during_write_leaves_no_named_stage";
    const CWD_DRIFT_ALTERNATE: &str =
        "ATRAMENT_TEST_EXPORT_CWD_DRIFT_ALTERNATE";
    const CWD_DRIFT_REPLACE: &str = "ATRAMENT_TEST_EXPORT_CWD_DRIFT_REPLACE";
    const CWD_DRIFT_TEST: &str =
        "linux::relative_cwd_change_during_stage_refuses_selected_destination";

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
        let metadata = fs::symlink_metadata(&target)
            .expect("committed selected entry without link traversal");
        assert!(metadata.is_file());
        assert_eq!(evidence.file_device, metadata.dev());
        assert_eq!(evidence.file_inode, metadata.ino());
        assert_eq!(
            evidence.file_modified,
            (metadata.mtime(), metadata.mtime_nsec()),
        );
        assert_eq!(
            evidence.file_changed,
            (metadata.ctime(), metadata.ctime_nsec()),
        );
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
    fn committed_permissions_ignore_restrictive_process_umask() {
        const CHILD: &str = "ATRAMENT_TEST_COMMIT_UMASK_CHILD";
        const SELF: &str =
            "linux::committed_permissions_ignore_restrictive_process_umask";
        if env::var_os(CHILD).is_none() {
            let result = Command::new(
                env::current_exe().expect("running test executable"),
            )
            .arg("--exact")
            .arg(SELF)
            .env(CHILD, "yes")
            .output()
            .expect("isolated child for process-wide umask mutation");
            assert!(
                result.status.success(),
                "umask child refused:\nstdout: {}\nstderr: {}",
                String::from_utf8_lossy(&result.stdout),
                String::from_utf8_lossy(&result.stderr),
            );
            return;
        }

        // Create and seed test directories *before* making the process umask
        // restrictive. The child changes global state with no other tests.
        let preserve = OwnedDir::new();
        let replace = OwnedDir::new();
        fs::write(replace.target(), b"previous target")
            .expect("preexisting replace target");
        let previous = umask(
            FileMode::from_bits_truncate(0o777),
        );
        let payload = b"complete, private profile";
        for (dir, disposition) in [
            (&preserve, Mode::PreserveExisting),
            (&replace, Mode::ReplaceExisting),
        ] {
            let target = dir.target();
            let evidence = commit_profile_file(
                &target, payload, payload.len(), disposition,
            ).expect("commit under restrictive process umask");
            assert_eq!(evidence.byte_length, payload.len());
            assert_eq!(fs::read(&target).expect("complete file"), payload);
            assert_eq!(
                fs::metadata(&target).expect("output metadata")
                    .permissions().mode() & 0o777,
                0o600,
                "selected output mode must not depend on process umask",
            );
            assert_eq!(dir.entries(), vec![String::from("chosen.atrament")]);
        }
        let _restored = umask(previous);
    }

    #[test]
    fn staged_inode_mutated_after_sync_cannot_publish() {
        use std::time::SystemTime;

        for disposition in [Mode::PreserveExisting, Mode::ReplaceExisting] {
            let dir = OwnedDir::new();
            let target = dir.target();
            if disposition == Mode::ReplaceExisting {
                fs::write(&target, b"previous output")
                    .expect("seed explicit replacement target");
            }
            let original = b"original complete artifact";
            let mut observations = 0usize;
            let result = commit_profile_file_cancellable(
                &target,
                original,
                original.len(),
                disposition,
                || {
                    observations += 1;
                    if observations == 3 {
                        // The caller's final cancellation callback runs
                        // after anonymous file sync, before directory linking.
                        // /proc provides another writable handle to that
                        // still-unnamed inode in this isolated Linux fixture.
                        let stage = fs::read_dir("/proc/self/fd")
                            .expect("inspect own process descriptors")
                            .find_map(|candidate| {
                                let entry = candidate.ok()?;
                                let path = fs::read_link(entry.path()).ok()?;
                                (path.starts_with(&dir.0)
                                    && path.to_string_lossy().ends_with(
                                        " (deleted)",
                                    ))
                                .then(|| entry.path())
                            })
                            .expect("test-owned O_TMPFILE descriptor");
                        let mut writable = OpenOptions::new()
                            .write(true)
                            .open(stage)
                            .expect("reopen own anonymous stage");
                        writable.write_all(b"tamper")
                            .expect("rewrite same-length staged prefix");
                        writable.set_modified(SystemTime::UNIX_EPOCH)
                            .expect("prove stage mtime changed after sync");
                    }
                    false
                },
            );
            assert_eq!(observations, 3);
            assert_eq!(result, Err(Error::StagedFileChanged));
            assert!(!result.is_err_and(Error::may_have_committed));
            if disposition == Mode::ReplaceExisting {
                assert_eq!(
                    fs::read(&target).expect("old target survives"),
                    b"previous output",
                );
                assert_eq!(
                    dir.entries(),
                    vec![String::from("chosen.atrament")],
                );
            } else {
                assert!(!target.exists());
                assert!(dir.entries().is_empty());
            }
        }
    }

    #[test]
    fn staged_bytes_changed_during_write_cannot_publish() {
        // A change made before sync is already included in the staged
        // metadata snapshot. Comparing metadata only after sync misses it.
        for disposition in [Mode::PreserveExisting, Mode::ReplaceExisting] {
            let dir = OwnedDir::new();
            let target = dir.target();
            if disposition == Mode::ReplaceExisting {
                fs::write(&target, b"original selected profile")
                    .expect("preexisting replacement target");
            }
            let artifact = vec![0x93u8; 2 * 64 * 1024];
            let mut observations = 0usize;
            let result = commit_profile_file_cancellable(
                &target,
                &artifact,
                artifact.len(),
                disposition,
                || {
                    observations += 1;
                    if observations == 3 {
                        // The first 64 KiB has been written but the final
                        // 64 KiB and sync have not. Mutating the first bytes
                        // leaves a complete same-length file after sync.
                        let stage = fs::read_dir("/proc/self/fd")
                            .expect("inspect own file descriptors")
                            .find_map(|candidate| {
                                let entry = candidate.ok()?;
                                let path = fs::read_link(entry.path()).ok()?;
                                (path.starts_with(&dir.0)
                                    && path.to_string_lossy().ends_with(
                                        " (deleted)",
                                    ))
                                .then(|| entry.path())
                            })
                            .expect("test-owned anonymous stage descriptor");
                        let mut writable = OpenOptions::new()
                            .write(true)
                            .open(stage)
                            .expect("reopen test-owned stage for mutation");
                        writable
                            .write_all(b"tamper")
                            .expect("alter staged bytes before final sync");
                    }
                    false
                },
            );
            assert_eq!(observations, 4);
            assert_eq!(result, Err(Error::StagedFileChanged));
            assert!(!result.is_err_and(Error::may_have_committed));
            if disposition == Mode::ReplaceExisting {
                assert_eq!(
                    fs::read(&target).expect("old selected file survives"),
                    b"original selected profile",
                );
                assert_eq!(
                    dir.entries(),
                    vec![String::from("chosen.atrament")],
                );
            } else {
                assert!(!target.exists());
                assert!(dir.entries().is_empty());
            }
        }
    }

    #[test]
    fn staged_file_extended_during_write_cannot_publish() {
        // Readback of only the expected prefix cannot reject unexpected
        // trailing bytes appended before the first post-sync observation.
        for disposition in [Mode::PreserveExisting, Mode::ReplaceExisting] {
            let dir = OwnedDir::new();
            let target = dir.target();
            if disposition == Mode::ReplaceExisting {
                fs::write(&target, b"original selected profile")
                    .expect("seed existing target");
            }
            let artifact = vec![0x93u8; 2 * 64 * 1024];
            let mut observations = 0usize;
            let result = commit_profile_file_cancellable(
                &target,
                &artifact,
                artifact.len(),
                disposition,
                || {
                    observations += 1;
                    if observations == 3 {
                        // The first chunk is complete but the second is not
                        // yet written. Extend the anonymous file beyond the
                        // artifact's final size using its process descriptor.
                        let stage = fs::read_dir("/proc/self/fd")
                            .expect("inspect own file descriptors")
                            .find_map(|candidate| {
                                let entry = candidate.ok()?;
                                let path = fs::read_link(entry.path()).ok()?;
                                (path.starts_with(&dir.0)
                                    && path.to_string_lossy().ends_with(
                                        " (deleted)",
                                    ))
                                .then(|| entry.path())
                            })
                            .expect("test-owned anonymous stage descriptor");
                        let writable = OpenOptions::new()
                            .write(true)
                            .open(stage)
                            .expect("reopen test-owned stage for mutation");
                        let extra = artifact
                            .len()
                            .checked_add(16)
                            .and_then(|size| u64::try_from(size).ok())
                            .expect("small fixture length");
                        writable.set_len(extra)
                            .expect("extend staged file during write");
                    }
                    false
                },
            );
            assert!(observations >= 3);
            assert_eq!(result, Err(Error::StagedFileChanged));
            assert!(!result.is_err_and(Error::may_have_committed));
            if disposition == Mode::ReplaceExisting {
                assert_eq!(
                    fs::read(&target).expect("old selected target remains"),
                    b"original selected profile",
                );
                assert_eq!(
                    dir.entries(),
                    vec![String::from("chosen.atrament")],
                );
            } else {
                assert!(!target.exists());
                assert!(dir.entries().is_empty());
            }
        }
    }

    #[test]
    fn staged_permission_widening_before_sync_refuses_export() {
        for (disposition, mutate_at) in [
            (Mode::PreserveExisting, 2usize),
            (Mode::PreserveExisting, 3usize),
            (Mode::ReplaceExisting, 2usize),
            (Mode::ReplaceExisting, 3usize),
        ] {
            let dir = OwnedDir::new();
            let target = dir.target();
            if disposition == Mode::ReplaceExisting {
                fs::write(&target, b"previous target")
                    .expect("original target");
            }
            let payload = b"strictly private profile contents";
            let mut observations = 0usize;
            let result = commit_profile_file_cancellable(
                &target,
                payload,
                payload.len(),
                disposition,
                || {
                    observations += 1;
                    if observations == mutate_at {
                        // The writer has already set 0600. Mutate just
                        // before the first or final metadata observation.
                        let stage = fs::read_dir("/proc/self/fd")
                            .expect("test process fds")
                            .find_map(|candidate| {
                                let entry = candidate.ok()?;
                                let path = fs::read_link(entry.path()).ok()?;
                                (path.starts_with(&dir.0)
                                    && path.to_string_lossy().ends_with(
                                        " (deleted)",
                                    ))
                                .then(|| entry.path())
                            })
                            .expect("anonymous stage descriptor");
                        let writable = OpenOptions::new()
                            .read(true)
                            .open(stage)
                            .expect("reopen test-owned stage");
                        writable.set_permissions(
                            fs::Permissions::from_mode(0o666),
                        )
                        .expect("simulate concurrent permission widening");
                    }
                    false
                },
            );
            assert_eq!(result, Err(Error::StagedFileChanged));
            assert!(observations >= 2);
            if disposition == Mode::ReplaceExisting {
                assert_eq!(
                    fs::read(&target).expect("original unchanged"),
                    b"previous target",
                );
            } else {
                assert!(!target.exists());
            }
            assert_eq!(
                dir.entries().len(),
                usize::from(disposition == Mode::ReplaceExisting),
            );
        }
    }

    #[test]
    fn externally_linked_anonymous_stage_must_not_publish_selected_entry() {
        use nix::fcntl::AtFlags;
        use nix::unistd::linkat;

        for (disposition, link_on_observation) in [
            (Mode::PreserveExisting, 2usize),
            (Mode::PreserveExisting, 3usize),
            (Mode::ReplaceExisting, 2usize),
            (Mode::ReplaceExisting, 3usize),
        ] {
            let dir = OwnedDir::new();
            let target = dir.target();
            if disposition == Mode::ReplaceExisting {
                fs::write(&target, b"previous target")
                    .expect("seed selected original");
            }
            let private_link = dir.0.join("external.atrament");
            let destination = fs::File::open(&dir.0)
                .expect("test-owned parent directory");
            let mut observations = 0usize;
            let payload = b"sealed, private profile artifact";
            let result = commit_profile_file_cancellable(
                &target,
                payload,
                payload.len(),
                disposition,
                || {
                    observations += 1;
                    if observations == link_on_observation {
                        // Expose another name before writing or after sync,
                        // covering both independent anonymous-stage guards.
                        // The rogue name must never become a successful Export
                        // of the still-selected pathname.
                        let stage = fs::read_dir("/proc/self/fd")
                            .expect("inspect process descriptors")
                            .find_map(|candidate| {
                                let entry = candidate.ok()?;
                                let path = fs::read_link(entry.path()).ok()?;
                                (path.starts_with(&dir.0)
                                    && path.to_string_lossy().ends_with(
                                        " (deleted)",
                                    ))
                                .then(|| entry.path())
                            })
                            .expect("test-owned anonymous staging fd");
                        let leaked_fd = OpenOptions::new()
                            .read(true)
                            .open(stage)
                            .expect("open test-owned stage through proc fd");
                        linkat(
                            &leaked_fd,
                            "",
                            &destination,
                            "external.atrament",
                            AtFlags::AT_EMPTY_PATH,
                        )
                        .expect("create attacker-controlled prelinked name");
                    }
                    false
                },
            );
            assert_eq!(result, Err(Error::StagedFileChanged));
            assert!(observations >= 2);
            if disposition == Mode::ReplaceExisting {
                assert_eq!(
                    fs::read(&target).expect("old target stays"),
                    b"previous target",
                );
            } else {
                assert!(!target.exists());
            }
            assert_eq!(
                fs::read(&private_link).expect("attacker link remains"),
                payload,
            );
        }
    }

    #[test]
    fn forced_death_during_write_leaves_no_named_stage() {
        // The child test process runs this same test function. Its parent
        // chooses which precommit boundary blocks until a forced SIGKILL.
        if let Some(path) = env::var_os(CRASH_STAGE_TARGET) {
            let target = PathBuf::from(path);
            let mode = if env::var_os(CRASH_STAGE_REPLACE).is_some() {
                Mode::ReplaceExisting
            } else {
                Mode::PreserveExisting
            };
            let after_sync = env::var_os(CRASH_STAGE_AFTER_SYNC).is_some();
            // 200,000 bytes take four 64-KiB writes: observations 2-5
            // precede those writes and observation 6 follows file sync.
            let stop_at = if after_sync { 6 } else { 3 };
            let payload = vec![0x83u8; 200_000];
            let mut observations = 0usize;
            let result = commit_profile_file_cancellable(
                &target,
                &payload,
                payload.len(),
                mode,
                || {
                    observations += 1;
                    if observations == stop_at {
                        // Step 3 follows the first chunk, while step 6
                        // follows full file sync but precedes the first link.
                        fs::write(target.with_extension("ready"), b"blocked")
                            .expect("notify parent at designated stage");
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
        assert_kill_before_commit(false);
    }

    #[test]
    fn forced_death_after_file_sync_before_link_leaves_no_named_stage() {
        // A complete synced O_TMPFILE still has no public directory entry.
        // This is not the later replacement link-before-rename window.
        assert_kill_before_commit(true);
    }

    fn assert_kill_before_commit(after_sync: bool) {
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
                .env_remove(CRASH_STAGE_AFTER_SYNC)
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null());
            if mode == Mode::ReplaceExisting {
                let _replace_flag = command.env(CRASH_STAGE_REPLACE, "yes");
            }
            if after_sync {
                let _after_sync = command.env(CRASH_STAGE_AFTER_SYNC, "yes");
            }
            let mut child = command.spawn().expect("spawn stage process");
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
            assert!(writing, "child did not reach the designated stage");
            assert_eq!(
                status.signal(),
                Some(SIGKILL),
                "child died by SIGKILL",
            );
            fs::remove_file(&marker).expect("remove test-only marker");
            if mode == Mode::ReplaceExisting {
                assert_eq!(
                    dir.entries(),
                    vec![String::from("chosen.atrament")],
                    "only the original target can remain after forced death",
                );
                assert_eq!(
                    fs::read(&target).expect("old target"),
                    b"original",
                    "the original target survives precommit process death",
                );
            } else {
                assert!(
                    dir.entries().is_empty(),
                    "no temporary or target name may survive forced death",
                );
                assert!(
                    !target.exists(),
                    "preserve mode cannot publish before the final link",
                );
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
        let evidence = commit_profile_file(
            &target,
            replacement,
            100,
            Mode::ReplaceExisting,
        ).expect("explicit replacement");
        assert_eq!(evidence.byte_length, replacement.len());
        let metadata = fs::symlink_metadata(&target)
            .expect("replacement selected entry without link traversal");
        assert!(metadata.is_file());
        assert_eq!(evidence.file_device, metadata.dev());
        assert_eq!(evidence.file_inode, metadata.ino());
        assert_eq!(
            evidence.file_modified,
            (metadata.mtime(), metadata.mtime_nsec()),
        );
        assert_eq!(
            evidence.file_changed,
            (metadata.ctime(), metadata.ctime_nsec()),
        );
        assert_eq!(fs::read(&target).expect("read replacement"), replacement);
        assert_eq!(dir.entries(), vec![String::from("chosen.atrament")]);
    }

    #[test]
    fn effect_classification_exhaustively_distinguishes_postcommit_errors() {
        assert!(Error::CommittedDurabilityUnknown(io::ErrorKind::Other)
            .may_have_committed());
        assert!(Error::CommittedPathUnverified.may_have_committed());
        for no_effect in [
            Error::ArtifactTooLarge,
            Error::CancelledBeforeCommit,
            Error::FileIo(io::ErrorKind::Other),
            Error::InvalidExistingTarget,
            Error::InvalidTarget,
            Error::RandomUnavailable,
            Error::SelectedParentChanged,
            Error::SelectedTargetChanged,
            Error::StagedFileChanged,
            Error::TargetExists,
            Error::UnsupportedPlatform,
        ] {
            assert!(!no_effect.may_have_committed(), "{no_effect:?}");
        }
    }

    #[test]
    fn renamed_selected_parent_never_reports_success_for_another_directory() {
        for disposition in [Mode::PreserveExisting, Mode::ReplaceExisting] {
            let dir = OwnedDir::new();
            let selected_parent = dir.0.join("selected");
            let relocated_parent = dir.0.join("relocated");
            fs::create_dir_all(&selected_parent).expect("selected parent");
            let target = selected_parent.join("chosen.atrament");
            if disposition == Mode::ReplaceExisting {
                fs::write(&target, b"old").expect("original target");
            }
            let mut observations = 0usize;
            let outcome = commit_profile_file_cancellable(
                &target,
                b"new complete artifact",
                64,
                disposition,
                || {
                    observations += 1;
                    if observations == 3 {
                        // Deterministic race after the directory was opened
                        // and the anonymous file fully synced, before link.
                        fs::rename(&selected_parent, &relocated_parent)
                            .expect("move original selected directory");
                        fs::create_dir_all(&selected_parent)
                            .expect("replace selected directory");
                    }
                    false
                },
            );
            assert_eq!(outcome, Err(Error::SelectedParentChanged));
            assert_eq!(observations, 3);
            assert!(!target.exists());
            let selected_count = fs::read_dir(&selected_parent)
                .expect("new directory")
                .count();
            assert_eq!(selected_count, 0);
            let original_in_relocated =
                relocated_parent.join("chosen.atrament");
            if disposition == Mode::ReplaceExisting {
                assert_eq!(
                    fs::read(&original_in_relocated)
                        .expect("original unchanged"),
                    b"old"
                );
            } else {
                assert!(!original_in_relocated.exists());
            }
            assert_eq!(
                fs::read_dir(&relocated_parent)
                    .expect("no staged output")
                    .count(),
                usize::from(disposition == Mode::ReplaceExisting),
            );
        }
    }

    #[test]
    fn symlink_redirect_of_selected_parent_never_writes_a_victim() {
        for disposition in [Mode::PreserveExisting, Mode::ReplaceExisting] {
            let dir = OwnedDir::new();
            let selected_parent = dir.0.join("selected");
            let relocated_parent = dir.0.join("relocated");
            let victim_parent = dir.0.join("victim");
            fs::create_dir_all(&selected_parent).expect("selected directory");
            fs::create_dir_all(&victim_parent).expect("victim directory");
            let target = selected_parent.join("chosen.atrament");
            let victim_target = victim_parent.join("chosen.atrament");
            fs::write(&victim_target, b"unrelated victim")
                .expect("victim bytes");
            if disposition == Mode::ReplaceExisting {
                fs::write(&target, b"original").expect("selected original");
            }
            let mut observations = 0usize;
            let result = commit_profile_file_cancellable(
                &target,
                b"replacement bytes",
                64,
                disposition,
                || {
                    observations += 1;
                    if observations == 3 {
                        fs::rename(&selected_parent, &relocated_parent)
                            .expect("relocate opened parent");
                        symlink(&victim_parent, &selected_parent)
                            .expect("redirect selected pathname");
                    }
                    false
                },
            );
            assert_eq!(result, Err(Error::SelectedParentChanged));
            assert_eq!(observations, 3);
            assert!(fs::symlink_metadata(&selected_parent)
                .expect("redirect remains")
                .file_type()
                .is_symlink());
            assert_eq!(
                fs::read(&victim_target).expect("unchanged victim"),
                b"unrelated victim",
            );
            let relocated_target = relocated_parent.join("chosen.atrament");
            if disposition == Mode::ReplaceExisting {
                assert_eq!(
                    fs::read(&relocated_target).expect("original"),
                    b"original",
                );
            } else {
                assert!(!relocated_target.exists());
            }
            assert_eq!(
                fs::read_dir(&relocated_parent)
                    .expect("no export stage")
                    .count(),
                usize::from(disposition == Mode::ReplaceExisting),
            );
        }
    }

    #[test]
    fn empty_artifact_still_rechecks_selected_parent_before_link() {
        for mode in [Mode::PreserveExisting, Mode::ReplaceExisting] {
            let dir = OwnedDir::new();
            let selected_parent = dir.0.join("selected");
            let relocated_parent = dir.0.join("relocated");
            fs::create_dir_all(&selected_parent).expect("selected parent");
            let target = selected_parent.join("chosen.atrament");
            if mode == Mode::ReplaceExisting {
                fs::write(&target, b"original").expect("original target");
            }
            let mut observations = 0usize;
            let result = commit_profile_file_cancellable(
                &target,
                b"",
                0,
                mode,
                || {
                    observations += 1;
                    if observations == 2 {
                        // With no write chunks, the second cancellation
                        // observation follows anonymous-file sync directly.
                        fs::rename(&selected_parent, &relocated_parent)
                            .expect("move original parent");
                        fs::create_dir_all(&selected_parent)
                            .expect("replace selected parent");
                    }
                    false
                },
            );
            assert_eq!(result, Err(Error::SelectedParentChanged));
            assert_eq!(observations, 2);
            assert!(!target.exists());
            let relocated = relocated_parent.join("chosen.atrament");
            if mode == Mode::ReplaceExisting {
                assert_eq!(
                    fs::read(&relocated).expect("original"),
                    b"original",
                );
            } else {
                assert!(!relocated.exists());
            }
            assert_eq!(
                fs::read_dir(&relocated_parent)
                    .expect("no unnamed leak")
                    .count(),
                usize::from(mode == Mode::ReplaceExisting),
            );
        }
    }

    #[test]
    fn relative_cwd_change_during_stage_refuses_selected_destination() {
        if let Some(alternate) = env::var_os(CWD_DRIFT_ALTERNATE) {
            let mode = if env::var_os(CWD_DRIFT_REPLACE).is_some() {
                Mode::ReplaceExisting
            } else {
                Mode::PreserveExisting
            };
            let stable = Path::new("nested/stable.atrament");
            assert_eq!(
                commit_profile_file(
                    stable,
                    b"stable",
                    6,
                    Mode::PreserveExisting,
                )
                .expect("unchanged CWD admits relative output")
                .byte_length,
                6,
            );
            let mut observations = 0usize;
            let result = commit_profile_file_cancellable(
                Path::new("nested/chosen.atrament"),
                b"complete relative output",
                64,
                mode,
                || {
                    observations += 1;
                    if observations == 3 {
                        // Child-only process CWD mutation avoids changing
                        // the shared test runner's working directory.
                        env::set_current_dir(&alternate)
                            .expect("change relative path resolution");
                    }
                    false
                },
            );
            assert_eq!(result, Err(Error::SelectedParentChanged));
            assert_eq!(observations, 3);
            return;
        }
        for mode in [Mode::PreserveExisting, Mode::ReplaceExisting] {
            let dir = OwnedDir::new();
            let original_cwd = dir.0.join("original-cwd");
            let alternate_cwd = dir.0.join("alternate-cwd");
            let original_parent = original_cwd.join("nested");
            let alternate_parent = alternate_cwd.join("nested");
            fs::create_dir_all(&original_parent)
                .expect("selected relative directory");
            fs::create_dir_all(&alternate_parent)
                .expect("alternate relative directory");
            let original = original_parent.join("chosen.atrament");
            let victim = alternate_parent.join("chosen.atrament");
            fs::write(&victim, b"unrelated bytes").expect("unrelated target");
            if mode == Mode::ReplaceExisting {
                fs::write(&original, b"original bytes")
                    .expect("selected original");
            }
            let executable = env::current_exe().expect("test executable");
            let mut command = Command::new(executable);
            let _configured = command
                .arg("--exact")
                .arg(CWD_DRIFT_TEST)
                .env(CWD_DRIFT_ALTERNATE, &alternate_cwd)
                .env_remove(CWD_DRIFT_REPLACE)
                .current_dir(&original_cwd);
            if mode == Mode::ReplaceExisting {
                let _mode = command.env(CWD_DRIFT_REPLACE, "yes");
            }
            let output = command.output().expect("run isolated cwd child");
            assert!(
                output.status.success(),
                "child must refuse relative output after cwd drift: {}",
                String::from_utf8_lossy(&output.stderr),
            );
            assert_eq!(
                fs::read(&victim).expect("victim untouched"),
                b"unrelated bytes",
            );
            assert_eq!(
                fs::read(original_parent.join("stable.atrament"))
                    .expect("unchanged relative control"),
                b"stable",
            );
            if mode == Mode::ReplaceExisting {
                assert_eq!(
                    fs::read(&original).expect("original"),
                    b"original bytes",
                );
            } else {
                assert!(!original.exists());
            }
            assert_eq!(
                fs::read_dir(&original_parent).expect("no named stage").count(),
                1 + usize::from(mode == Mode::ReplaceExisting),
            );
        }
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
    fn replacement_target_swap_to_symlink_refuses_before_link() {
        let dir = OwnedDir::new();
        let target = dir.target();
        let victim = dir.0.join("victim");
        fs::write(&target, b"original target").expect("seed regular target");
        fs::write(&victim, b"unrelated bytes").expect("seed victim");
        let mut observations = 0usize;
        let committed = commit_profile_file_cancellable(
            &target,
            b"new bytes",
            9,
            Mode::ReplaceExisting,
            || {
                observations += 1;
                if observations == 3 {
                    // The initial nofollow stat already saw a regular file.
                    fs::remove_file(&target).expect("remove observed target");
                    symlink(&victim, &target).expect("swap target to link");
                }
                false
            },
        );
        assert_eq!(committed, Err(Error::InvalidExistingTarget));
        assert_eq!(observations, 3);
        assert_eq!(
            fs::read(&victim).expect("unrelated file"),
            b"unrelated bytes",
        );
        assert_eq!(
            fs::read_link(&target).expect("late symlink preserved"),
            victim,
        );
        assert_eq!(dir.entries().len(), 2);
    }

    #[test]
    fn newly_created_symlink_during_replace_stage_is_not_removed() {
        let dir = OwnedDir::new();
        let target = dir.target();
        let victim = dir.0.join("victim");
        fs::write(&victim, b"unrelated bytes").expect("seed victim");
        let mut observations = 0usize;
        let result = commit_profile_file_cancellable(
            &target,
            b"new bytes",
            9,
            Mode::ReplaceExisting,
            || {
                observations += 1;
                if observations == 3 {
                    symlink(&victim, &target).expect("insert late link");
                }
                false
            },
        );
        assert_eq!(result, Err(Error::InvalidExistingTarget));
        assert_eq!(observations, 3);
        assert_eq!(
            fs::read_link(&target).expect("late link preserved"),
            victim,
        );
        assert_eq!(
            fs::read(&victim).expect("victim untouched"),
            b"unrelated bytes",
        );
        assert_eq!(dir.entries().len(), 2);
    }

    #[test]
    fn late_regular_target_creation_never_clobbers_unselected_bytes() {
        let dir = OwnedDir::new();
        let target = dir.target();
        let mut observations = 0usize;
        let outcome = commit_profile_file_cancellable(
            &target,
            b"intended profile",
            16,
            Mode::ReplaceExisting,
            || {
                observations += 1;
                if observations == 3 {
                    fs::write(&target, b"unselected external file")
                        .expect("create external target during staging");
                }
                false
            },
        );
        assert_eq!(outcome, Err(Error::SelectedTargetChanged));
        assert_eq!(observations, 3);
        assert_eq!(
            fs::read(&target).expect("late target preserved"),
            b"unselected external file",
        );
        assert_eq!(dir.entries(), vec![String::from("chosen.atrament")]);
    }

    #[test]
    fn swapped_regular_target_never_clobbers_new_entry_or_old_inode() {
        let dir = OwnedDir::new();
        let target = dir.target();
        let parked = dir.0.join("parked");
        fs::write(&target, b"original selection").expect("initial target");
        let mut observations = 0usize;
        let outcome = commit_profile_file_cancellable(
            &target,
            b"intended profile",
            16,
            Mode::ReplaceExisting,
            || {
                observations += 1;
                if observations == 3 {
                    fs::rename(&target, &parked).expect("park original");
                    fs::write(&target, b"another external file")
                        .expect("create new regular target");
                }
                false
            },
        );
        assert_eq!(outcome, Err(Error::SelectedTargetChanged));
        assert_eq!(observations, 3);
        assert_eq!(fs::read(&parked).expect("original"), b"original selection");
        assert_eq!(
            fs::read(&target).expect("late target"),
            b"another external file",
        );
        assert_eq!(dir.entries().len(), 2);
    }

    #[test]
    fn removed_replacement_target_during_staging_is_not_recreated() {
        let dir = OwnedDir::new();
        let target = dir.target();
        fs::write(&target, b"original selection").expect("initial target");
        let mut observations = 0usize;
        let outcome = commit_profile_file_cancellable(
            &target,
            b"intended profile",
            16,
            Mode::ReplaceExisting,
            || {
                observations += 1;
                if observations == 3 {
                    fs::remove_file(&target).expect("remove selected target");
                }
                false
            },
        );
        assert_eq!(outcome, Err(Error::SelectedTargetChanged));
        assert_eq!(observations, 3);
        assert!(!target.exists());
        assert!(dir.entries().is_empty());
    }

    #[test]
    fn modified_same_inode_during_replace_staging_is_not_overwritten() {
        let dir = OwnedDir::new();
        let target = dir.target();
        fs::write(&target, b"original selection").expect("initial target");
        let original_inode = fs::metadata(&target).expect("inode").ino();
        let mut observations = 0usize;
        let outcome = commit_profile_file_cancellable(
            &target,
            b"intended profile",
            16,
            Mode::ReplaceExisting,
            || {
                observations += 1;
                if observations == 3 {
                    fs::write(&target, b"externally updated longer content")
                        .expect("write same external inode");
                }
                false
            },
        );
        assert_eq!(outcome, Err(Error::SelectedTargetChanged));
        assert_eq!(observations, 3);
        assert_eq!(fs::metadata(&target).expect("inode").ino(), original_inode);
        assert_eq!(
            fs::read(&target).expect("changed target preserved"),
            b"externally updated longer content",
        );
        assert_eq!(dir.entries(), vec![String::from("chosen.atrament")]);
    }

    #[test]
    fn same_length_inplace_rewrite_with_restored_mtime_refuses_replace() {
        use std::time::UNIX_EPOCH;

        let dir = OwnedDir::new();
        let target = dir.target();
        fs::write(&target, b"original").expect("original target");
        let before = fs::metadata(&target).expect("original metadata");
        let original_mtime = UNIX_EPOCH
            + Duration::new(
                before.mtime().try_into().expect("positive seconds"),
                before.mtime_nsec().try_into().expect("valid nanos"),
            );
        let mut observations = 0usize;
        let result = commit_profile_file_cancellable(
            &target,
            b"exported",
            8,
            Mode::ReplaceExisting,
            || {
                observations += 1;
                if observations == 3 {
                    thread::sleep(Duration::from_millis(20));
                    fs::write(&target, b"external")
                        .expect("rewrite selected inode");
                    fs::File::options()
                        .write(true)
                        .open(&target)
                        .expect("open for timestamp restoration")
                        .set_modified(original_mtime)
                        .expect("restore modification timestamp");
                    let current = fs::metadata(&target)
                        .expect("externally modified metadata");
                    assert_eq!(before.dev(), current.dev());
                    assert_eq!(before.ino(), current.ino());
                    assert_eq!(before.len(), current.len());
                    assert_eq!(
                        (before.mtime(), before.mtime_nsec()),
                        (current.mtime(), current.mtime_nsec()),
                    );
                    assert_ne!(
                        (before.ctime(), before.ctime_nsec()),
                        (current.ctime(), current.ctime_nsec()),
                    );
                }
                false
            },
        );
        assert_eq!(result, Err(Error::SelectedTargetChanged));
        assert_eq!(observations, 3);
        assert_eq!(fs::read(&target).expect("external bytes"), b"external");
        assert_eq!(dir.entries(), vec![String::from("chosen.atrament")]);
    }

    #[test]
    fn replacement_target_swap_to_directory_refuses_without_stage_leak() {
        let dir = OwnedDir::new();
        let target = dir.target();
        fs::write(&target, b"original target").expect("seed regular target");
        let mut observations = 0usize;
        let denied = commit_profile_file_cancellable(
            &target,
            b"new bytes",
            9,
            Mode::ReplaceExisting,
            || {
                observations += 1;
                if observations == 3 {
                    fs::remove_file(&target).expect("remove observed target");
                    fs::create_dir_all(&target)
                        .expect("swap target to directory");
                }
                false
            },
        );
        assert_eq!(denied, Err(Error::InvalidExistingTarget));
        assert_eq!(observations, 3);
        assert!(target.is_dir());
        assert_eq!(dir.entries(), vec![String::from("chosen.atrament")]);
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
    fn embedded_nul_target_refuses_before_cancellation_or_file_io() {
        use std::os::unix::ffi::OsStringExt as _;

        let dir = OwnedDir::new();
        for name in [
            b"bad\0name.atrament".as_slice(),
            b"part\0name/file".as_slice(),
        ] {
            let target = dir.0.join(OsString::from_vec(name.to_vec()));
            for mode in [Mode::PreserveExisting, Mode::ReplaceExisting] {
                let mut cancellation_checks = 0usize;
                assert_eq!(
                    commit_profile_file_cancellable(
                        &target,
                        b"private profile bytes",
                        21,
                        mode,
                        || {
                            cancellation_checks += 1;
                            false
                        },
                    ),
                    Err(Error::InvalidTarget),
                );
                assert_eq!(cancellation_checks, 0);
            }
        }
        assert!(dir.entries().is_empty());
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
