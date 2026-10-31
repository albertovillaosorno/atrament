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
//   - Real-file session Export/receipt/retry integration regressions.
// - Must-Not:
//   - Claim host selection authentication or a production session API.
// - Allows:
//   - Inputs: Test-owned sealed ZIP, revisions and selected filesystem paths.
//   - Outputs: Exact synced receipt, recovery and fail-closed refusal evidence.
//   - Side effects: Writes and cleans only unique fixture directory entries.
// - Split-When:
//   - Host selection and accepted profile session integration exist.
// - Merge-When:
//   - End-to-end explicit profile Export fixtures own all boundaries.
// - Summary:
//   - No retry key repeats a write or recovers an unsynced receipt.
// - Description:
//   - Covers exact retry, changed requests, cancellation, drift and links.
// - Usage:
//   - cargo test -p atrament_profile_export_session
// - Defaults:
//   - No filesystem effects outside fixture-owned temporary directories.
//

//! Exact, disposable profile Export attempt and recovery lifecycle.

#![expect(clippy::expect_used, reason = "test-owned real files")]

#[cfg(target_os = "linux")]
mod linux {
    use std::cell::Cell;
    use std::env;
    use std::ffi::OsString;
    use std::fs;
    use std::io::{self, Read as _};
    use std::os::unix::ffi::OsStringExt as _;
    use std::os::unix::fs::{MetadataExt as _, symlink};
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::process::{self, Command, Stdio};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::thread;
    use std::time::Duration;

    use super::*;
    use ExplicitOverwriteIntent::{PreserveExisting, ReplaceExisting};

    static NEXT: AtomicU64 = AtomicU64::new(0);

    const CRASH_EXPORT_TARGET: &str = "ATRAMENT_TEST_CRASH_EXPORT_TARGET";
    const CRASH_EXPORT_TEST: &str =
        "linux::completed_export_survives_forced_process_death";

    struct OwnedDir(PathBuf);
    #[expect(clippy::missing_trait_methods, reason = "fixture cleanup only")]
    impl Drop for OwnedDir {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).expect("remove owned fixture");
        }
    }
    #[expect(clippy::create_dir, reason = "unique fixture must not exist")]
    fn create(root: &PathBuf) {
        fs::create_dir(root).expect("unique fixture directory");
    }
    fn fixture() -> OwnedDir {
        let root = env::temp_dir().join(format!(
            "atrament-export-session-{}-{}",
            process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed),
        ));
        create(&root);
        OwnedDir(root)
    }

    #[test]
    fn synced_commit_recovers_original_receipt_without_reopening_writer() {
        let root = fixture();
        let chosen = root.0.join("selected.atrament");
        let request = intent(chosen.clone(), PreserveExisting);
        let original = artifact("writer-a");
        let accepted = revision();
        let mut session = ProfileExportSession::new(2);
        assert!(session.is_empty());
        let result = session
            .export(
                "request-a",
                accepted,
                &request,
                &original,
                original.bytes().len(),
            )
            .expect("first synced commit");
        assert!(matches!(result, Outcome::Committed(_)));
        let receipt = match result {
            Outcome::Committed(value) | Outcome::Recovered(value) => value,
        };
        assert_eq!(receipt.revision(), accepted);
        assert_eq!(receipt.path(), chosen);
        assert_eq!(receipt.overwrite(), PreserveExisting);
        assert_eq!(receipt.retry_key(), "request-a");
        assert_eq!(receipt.archive_sha256(), original.archive_sha256());
        assert_eq!(receipt.root_sha256(), original.root_sha256());
        assert_eq!(receipt.byte_length(), original.bytes().len());
        assert_eq!(receipt.source(), ExplicitFilePathSource::CallerSelection);
        assert_eq!(
            fs::read(&chosen).expect("read committed output"),
            original.bytes()
        );
        let inode_before =
            fs::metadata(&chosen).expect("committed inode").ino();
        assert_eq!(session.len(), 1);
        let called = Cell::new(false);
        let replay = session
            .export_cancellable(
                "request-a",
                accepted,
                &request,
                &original,
                original.bytes().len(),
                || {
                    called.set(true);
                    false
                },
            )
            .expect("read-only receipt recovery");
        assert_eq!(replay, Outcome::Recovered(receipt));
        assert!(!called.get());
        assert_eq!(
            fs::metadata(&chosen).expect("unchanged inode").ino(),
            inode_before
        );
        assert_eq!(fs::read_dir(&root.0).expect("list fixture").count(), 1);
    }

    #[test]
    fn synced_receipt_retains_exact_opaque_target_and_maximum_retry_key() {
        let root = fixture();
        let mut name = b"selected-opaque-".to_vec();
        name.push(0xff);
        let chosen = root.0.join(OsString::from_vec(name));
        let request = intent(chosen.clone(), ReplaceExisting);
        let key = "r".repeat(128);
        let original = artifact("writer-long-retry");
        let accepted = revision();
        let mut session = ProfileExportSession::new(1);
        let first = session.export(
            &key, accepted, &request, &original, 8192,
        ).expect("complete first commit before publishing receipt");
        assert!(matches!(first, Outcome::Committed(_)));
        let receipt = match first {
            Outcome::Committed(value) | Outcome::Recovered(value) => value,
        };
        assert_eq!(receipt.path(), chosen);
        assert_eq!(receipt.retry_key(), key);
        assert_eq!(receipt.revision(), accepted);
        assert_eq!(receipt.byte_length(), original.bytes().len());
        assert_eq!(receipt.archive_sha256(), original.archive_sha256());
        assert_eq!(receipt.root_sha256(), original.root_sha256());
        assert_eq!(receipt.overwrite(), ReplaceExisting);
        let inode = fs::metadata(&chosen).expect("committed inode").ino();
        let second = session.export(
            &key, accepted, &request, &original, 8192,
        ).expect("recover exactly the originally retained receipt");
        assert_eq!(second, Outcome::Recovered(receipt));
        assert_eq!(fs::metadata(&chosen).expect("same inode").ino(), inode);
        assert_eq!(fs::read(&chosen).expect("exact bytes"), original.bytes());
        assert_eq!(session.len(), 1);
    }

    #[test]
    fn debug_of_export_session_and_receipts_redacts_private_identifiers() {
        let root = fixture();
        let selected_name = "private-selected-path-marker.atrament";
        let key = "private-retry-key-marker";
        let chosen = root.0.join(selected_name);
        let request = intent(chosen.clone(), PreserveExisting);
        let original = artifact("private-profile-marker");
        let accepted = revision();
        let mut session = ProfileExportSession::new(2);
        let committed = session.export(
            key, accepted, &request, &original, original.bytes().len(),
        ).expect("commit test profile");
        assert!(matches!(committed, Outcome::Committed(_)));
        let receipt = match committed.clone() {
            Outcome::Committed(value) | Outcome::Recovered(value) => value,
        };
        assert_eq!(receipt.path(), chosen);
        assert_eq!(receipt.retry_key(), key);
        let recovered = session.export(
            key, accepted, &request, &original, original.bytes().len(),
        ).expect("recover original receipt");
        assert_eq!(recovered, Outcome::Recovered(receipt.clone()));
        let debug = format!(
            "{session:?} {receipt:?} {committed:?} {recovered:?}",
        );
        assert!(debug.contains("ProfileExportSession"));
        assert!(debug.contains("ProfileExportReceipt"));
        assert!(debug.contains("Committed"));
        assert!(debug.contains("Recovered"));
        for private in [key, selected_name, "private-profile-marker"] {
            assert!(!debug.contains(private));
        }
        assert_eq!(
            fs::read(&chosen).expect("committed profile persists"),
            original.bytes(),
        );
    }

    #[test]
    fn failed_preserve_existing_never_later_recovers_from_matching_bytes() {
        let root = fixture();
        let chosen = root.0.join("existing.atrament");
        fs::write(&chosen, b"existing bytes").expect("pre-existing target");
        let request = intent(chosen.clone(), PreserveExisting);
        let original = artifact("writer-a");
        let accepted = revision();
        let mut session = ProfileExportSession::new(1);
        assert_eq!(
            session.export("key", accepted, &request, &original, 8192),
            Err(Error::Commit(ProfileFileCommitError::TargetExists)),
        );
        assert_eq!(
            fs::read(&chosen).expect("preserved target"),
            b"existing bytes"
        );
        fs::write(&chosen, original.bytes())
            .expect("external matching replacement");
        assert_eq!(
            session.export("key", accepted, &request, &original, 8192),
            Err(Error::Retry(
                ProfileExportRetryRecoveryDecision::PriorKnownNoCommit
            )),
        );
        assert_eq!(session.len(), 1);
        assert_eq!(
            fs::read(&chosen).expect("external file remains"),
            original.bytes()
        );
    }

    #[test]
    fn cancelled_replace_preserves_target_and_refuses_same_key_write() {
        let root = fixture();
        let chosen = root.0.join("replace.atrament");
        fs::write(&chosen, b"original target").expect("original target");
        let request = intent(chosen.clone(), ReplaceExisting);
        let original = artifact("writer-a");
        let accepted = revision();
        let mut session = ProfileExportSession::new(2);
        assert_eq!(
            session.export_cancellable(
                "cancel",
                accepted,
                &request,
                &original,
                8192,
                || true,
            ),
            Err(Error::Commit(ProfileFileCommitError::CancelledBeforeCommit)),
        );
        assert_eq!(
            fs::read(&chosen).expect("target preserved"),
            b"original target"
        );
        assert_eq!(fs::read_dir(&root.0).expect("only original").count(), 1);
        assert_eq!(
            session.export("cancel", accepted, &request, &original, 8192),
            Err(Error::Retry(
                ProfileExportRetryRecoveryDecision::PriorKnownNoCommit
            )),
        );
        let committed = session
            .export("new-authorized-key", accepted, &request, &original, 8192)
            .expect("distinct authorized attempt");
        assert!(matches!(committed, Outcome::Committed(_)));
        let receipt = match &committed {
            Outcome::Committed(value) | Outcome::Recovered(value) => value,
        };
        assert_eq!(receipt.overwrite(), ReplaceExisting);
        assert_eq!(receipt.retry_key(), "new-authorized-key");
        assert_eq!(receipt.path(), chosen);
        assert_eq!(
            fs::read(&chosen).expect("new exact bytes"),
            original.bytes()
        );
    }

    #[test]
    fn selected_parent_replacement_refuses_receipt_and_same_key_replay() {
        for overwrite in [PreserveExisting, ReplaceExisting] {
            let root = fixture();
            let selected_parent = root.0.join("selected");
            let relocated_parent = root.0.join("relocated");
            fs::create_dir_all(&selected_parent).expect("selected parent");
            let chosen = selected_parent.join("profile.atrament");
            if overwrite == ReplaceExisting {
                fs::write(&chosen, b"original").expect("original file");
            }
            let request = intent(chosen.clone(), overwrite);
            let original = artifact("writer-parent-drift");
            let accepted = revision();
            let mut session = ProfileExportSession::new(2);
            let mut observations = 0usize;
            let refused = session.export_cancellable(
                "first-key",
                accepted,
                &request,
                &original,
                original.bytes().len(),
                || {
                    observations += 1;
                    if observations == 3 {
                        fs::rename(&selected_parent, &relocated_parent)
                            .expect("relocate selected directory");
                        fs::create_dir_all(&selected_parent)
                            .expect("replace selected directory");
                    }
                    false
                },
            );
            assert_eq!(
                refused,
                Err(Error::Commit(ProfileFileCommitError::SelectedParentChanged)),
            );
            assert_eq!(observations, 3);
            assert_eq!(session.len(), 1);
            assert!(!chosen.exists());
            let relocated = relocated_parent.join("profile.atrament");
            if overwrite == ReplaceExisting {
                assert_eq!(fs::read(&relocated).expect("original"), b"original");
            } else {
                assert!(!relocated.exists());
            }
            assert_eq!(
                session.export("first-key", accepted, &request, &original, 8192),
                Err(Error::Retry(
                    ProfileExportRetryRecoveryDecision::PriorKnownNoCommit
                )),
            );
            assert!(!chosen.exists());
            assert!(matches!(
                session.export("fresh-key", accepted, &request, &original, 8192),
                Ok(Outcome::Committed(_)),
            ));
            assert_eq!(fs::read(&chosen).expect("new authorized effect"), original.bytes());
        }
    }

    #[test]
    fn drift_and_link_swap_refuse_replace_retry() {
        let root = fixture();
        let chosen = root.0.join("selected.atrament");
        let victim = root.0.join("unrelated");
        let request = intent(chosen.clone(), ReplaceExisting);
        let original = artifact("writer-a");
        let accepted = revision();
        let mut session = ProfileExportSession::new(1);
        assert!(matches!(
            session.export("key", accepted, &request, &original, 8192),
            Ok(Outcome::Committed(_)),
        ));
        fs::write(&chosen, b"external replacement").expect("drift target");
        assert_eq!(
            session.export("key", accepted, &request, &original, 8192),
            Err(Error::Retry(
                ProfileExportRetryRecoveryDecision::ExternalTargetDrift
            )),
        );
        assert_eq!(
            fs::read(&chosen).expect("external bytes"),
            b"external replacement"
        );
        fs::remove_file(&chosen).expect("remove selected target");
        assert_eq!(
            session.export("key", accepted, &request, &original, 8192),
            Err(Error::Retry(
                ProfileExportRetryRecoveryDecision::ExternalTargetDrift
            )),
        );
        fs::write(&victim, original.bytes()).expect("victim bytes");
        symlink(&victim, &chosen).expect("replace selected with symlink");
        assert!(matches!(
            session.export("key", accepted, &request, &original, 8192),
            Err(Error::Retry(
                ProfileExportRetryRecoveryDecision::TargetInspectionFailed(_)
            )),
        ));
        assert_eq!(fs::read_link(&chosen).expect("symlink remains"), victim);
        assert_eq!(
            fs::read(&victim).expect("victim untouched"),
            original.bytes()
        );
    }

    #[test]
    fn changed_retry_identity_conflicts_without_writes() {
        let root = fixture();
        let chosen = root.0.join("selected.atrament");
        let alternate = root.0.join("alternate.atrament");
        let request = intent(chosen.clone(), PreserveExisting);
        let replacing = intent(chosen.clone(), ReplaceExisting);
        let changed_target = intent(alternate.clone(), PreserveExisting);
        let changed_source = classify_explicit_file_path_intent(
            ExplicitFilePurpose::ExportProfile,
            ExplicitFilePathSource::AdmittedHostPolicy,
            chosen.clone(),
            PreserveExisting,
        )
        .expect("host policy classification");
        let original = artifact("writer-a");
        let different = artifact("writer-b");
        let allocator = IdentityAllocator::new();
        let accepted = allocator.allocate_revision().expect("first revision");
        let changed = allocator.allocate_revision().expect("second revision");
        let mut session = ProfileExportSession::new(1);
        assert!(matches!(
            session.export("key", accepted, &request, &original, 8192),
            Ok(Outcome::Committed(_)),
        ));
        for refusal in [
            session.export("key", changed, &request, &original, 8192),
            session.export("key", accepted, &changed_target, &original, 8192),
            session.export("key", accepted, &replacing, &original, 8192),
            session.export("key", accepted, &changed_source, &original, 8192),
            session.export("key", accepted, &request, &different, 8192),
        ] {
            assert_eq!(
                refusal,
                Err(Error::Binding(
                    ProfileRetryBindingError::ConflictingRequest,
                )),
            );
        }
        assert!(!alternate.exists());
        assert_eq!(
            fs::read(&chosen).expect("unchanged original"),
            original.bytes()
        );
        assert_eq!(
            session.export("second-key", accepted, &request, &original, 8192),
            Err(Error::Binding(ProfileRetryBindingError::JournalFull)),
        );
    }

    #[test]
    fn invalid_purpose_and_byte_budget_refuse_before_file_effects() {
        let root = fixture();
        let chosen = root.0.join("selected.atrament");
        let importing = classify_explicit_file_path_intent(
            ExplicitFilePurpose::ImportProfile,
            ExplicitFilePathSource::CallerSelection,
            chosen.clone(),
            PreserveExisting,
        )
        .expect("classified Import source");
        let request = intent(chosen.clone(), PreserveExisting);
        let original = artifact("writer-a");
        let accepted = revision();
        let mut session = ProfileExportSession::new(2);
        assert_eq!(
            session.export("key", accepted, &importing, &original, 8192),
            Err(Error::Binding(ProfileRetryBindingError::WrongPathPurpose)),
        );
        assert_eq!(
            session.export("", accepted, &request, &original, 8192),
            Err(Error::Binding(ProfileRetryBindingError::InvalidRetryKey)),
        );
        assert!(session.is_empty());
        assert!(!chosen.exists());
        assert_eq!(
            session.export("key", accepted, &request, &original, 0),
            Err(Error::Commit(ProfileFileCommitError::ArtifactTooLarge)),
        );
        assert_eq!(session.len(), 1);
        assert!(!chosen.exists());
        assert_eq!(
            session.export("key", accepted, &request, &original, 8192),
            Err(Error::Retry(
                ProfileExportRetryRecoveryDecision::PriorKnownNoCommit
            )),
        );
        assert!(!chosen.exists());
    }

    #[test]
    fn unwound_cancellation_callback_never_retries_or_recovers() {
        let root = fixture();
        let chosen = root.0.join("selected.atrament");
        fs::write(&chosen, b"original target").expect("original file");
        let request = intent(chosen.clone(), ReplaceExisting);
        let original = artifact("writer-a");
        let accepted = revision();
        let mut session = ProfileExportSession::new(2);
        let calls = Cell::new(0u8);
        let unwind = catch_unwind(AssertUnwindSafe(|| {
            let _result = session.export_cancellable(
                "unwound",
                accepted,
                &request,
                &original,
                8192,
                || {
                    calls.set(calls.get() + 1);
                    assert!(
                        calls.get() != 3,
                        "fixture cancellation callback unwind"
                    );
                    false
                },
            );
        }));
        assert!(unwind.is_err());
        // One before I/O, one before the write, one after temporary sync.
        assert_eq!(calls.get(), 3);
        assert_eq!(session.len(), 1);
        assert_eq!(
            fs::read(&chosen).expect("preserved original file"),
            b"original target"
        );
        assert_eq!(fs::read_dir(&root.0).expect("no stage leaked").count(), 1);
        // A matched external file cannot upgrade the missing effect record.
        fs::write(&chosen, original.bytes()).expect("external matching bytes");
        assert_eq!(
            session.export("unwound", accepted, &request, &original, 8192),
            Err(Error::Retry(
                ProfileExportRetryRecoveryDecision::PriorOutcomeNotRecorded
            )),
        );
        let outcome =
            session.export("distinct-key", accepted, &request, &original, 8192);
        assert!(matches!(outcome, Ok(Outcome::Committed(_))));
    }

    #[test]
    fn selected_import_round_trip_refuses_tampered_export_retry() {
        let root = fixture();
        let chosen = root.0.join("selected.atrament");
        let request = intent(chosen.clone(), PreserveExisting);
        let original = artifact("writer-a");
        let accepted = revision();
        let mut session = ProfileExportSession::new(1);
        let effect =
            session.export("roundtrip", accepted, &request, &original, 8192);
        assert!(matches!(effect, Ok(Outcome::Committed(_))));
        let before = fs::metadata(&chosen).expect("exported inode").ino();
        let selected_import = classify_explicit_file_path_intent(
            ExplicitFilePurpose::ImportProfile,
            ExplicitFilePathSource::CallerSelection,
            chosen.clone(),
            PreserveExisting,
        )
        .expect("test-selected import intent");
        let imported = inspect_selected_profile_import(
            &selected_import,
            &[],
            import_limits(),
            |verified| {
                assert_eq!(verified.path().path(), chosen);
                assert_eq!(verified.verified_entry_count(), 0);
                assert!(verified.manifest().entries.is_empty());
                verified.manifest().profile_identity.clone()
            },
        );
        assert_eq!(imported, Ok(String::from("writer-a")));
        assert_eq!(fs::metadata(&chosen).expect("same inode").ino(), before);
        assert_eq!(
            fs::read(&chosen).expect("original bytes"),
            original.bytes()
        );
        assert_eq!(fs::read_dir(&root.0).expect("only output").count(), 1);
        assert_eq!(session.len(), 1);

        // External tampering cannot pass Import or turn same-key Export into
        // a new file effect, even though the original Export had succeeded.
        let mut tampered = original.bytes().to_vec();
        let first = tampered.first_mut().expect("nonempty sealed ZIP");
        *first ^= 1;
        fs::write(&chosen, &tampered).expect("external archive mutation");
        let inspected = Cell::new(false);
        let rejected = inspect_selected_profile_import(
            &selected_import,
            &[],
            import_limits(),
            |_verified| inspected.set(true),
        );
        assert!(matches!(rejected, Err(ProfileOpenFileImportError::Zip(_))));
        assert!(!inspected.get());
        assert_eq!(
            session.export("roundtrip", accepted, &request, &original, 8192),
            Err(Error::Retry(
                ProfileExportRetryRecoveryDecision::ExternalTargetDrift
            )),
        );
        assert_eq!(fs::read(&chosen).expect("external file"), tampered);
        assert_eq!(session.len(), 1);
    }

    #[test]
    fn completed_export_survives_forced_process_death() {
        // Run the exact same test binary in another process so its retry
        // journal cannot remain reachable after termination.
        if let Some(path) = env::var_os(CRASH_EXPORT_TARGET) {
            let chosen = PathBuf::from(path);
            let original = full_artifact(b"{\"ink\":1}");
            let request = intent(chosen.clone(), PreserveExisting);
            let mut session = ProfileExportSession::new(1);
            let result = session.export(
                "crash-key", revision(), &request, &original, 8192,
            );
            assert!(matches!(result, Ok(Outcome::Committed(_))));
            // The parent only kills the child once the synced commit is
            // complete. No receipt or retry state is serialized here.
            fs::write(chosen.with_extension("ready"), b"synced")
                .expect("publish fixture commit marker");
            let mut release = [0u8; 1];
            let _release = io::stdin().read_exact(&mut release);
            return;
        }

        let root = fixture();
        let chosen = root.0.join("export.atrament");
        let ready = chosen.with_extension("ready");
        let executable = env::current_exe().expect("test executable");
        let mut child = Command::new(executable)
            .arg("--exact")
            .arg(CRASH_EXPORT_TEST)
            .env(CRASH_EXPORT_TARGET, &chosen)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .spawn()
            .expect("spawn Export-owning process");
        let mut synced = false;
        for _ in 0..200u16 {
            if ready.exists() {
                synced = true;
                break;
            }
            if child.try_wait().expect("child liveness").is_some() {
                break;
            }
            thread::sleep(Duration::from_millis(50));
        }
        // Always reap the child before any fixture assertions can panic.
        if synced {
            child.kill().expect("force post-commit process death");
        } else if child.try_wait().expect("child liveness").is_none() {
            child.kill().expect("stop timed-out fixture process");
        }
        let status = child.wait().expect("reap Export process");
        assert!(synced, "child never completed its explicit Export");
        assert!(!status.success(), "the Export process was forcibly stopped");
        assert_eq!(fs::read(&ready).expect("sync marker"), b"synced");
        fs::remove_file(&ready).expect("remove test-only marker");
        assert_eq!(fs::read_dir(&root.0).expect("export files").count(), 1);

        let original = full_artifact(b"{\"ink\":1}");
        assert_eq!(
            fs::read(&chosen).expect("surviving file"),
            original.bytes(),
        );
        let import = classify_explicit_file_path_intent(
            ExplicitFilePurpose::ImportProfile,
            ExplicitFilePathSource::CallerSelection,
            chosen.clone(),
            PreserveExisting,
        )
        .expect("explicit selection after restart");
        assert_eq!(
            inspect_selected_profile_import(
                &import,
                &[],
                import_limits(),
                |verified| {
                    assert_eq!(verified.verified_entry_count(), 2);
                    assert_eq!(
                        verified.verified_entry_bytes("sections/ink.json"),
                        Some(&b"{\"ink\":1}"[..]),
                    );
                    assert_eq!(
                        verified.verified_entry_bytes("assets/sample.bin"),
                        Some(BINARY_ASSET),
                    );
                    verified.archive_sha256()
                },
            ),
            Ok(original.archive_sha256()),
        );

        // A fresh process owner did not inherit the dead process's receipt.
        let mut fresh = ProfileExportSession::new(1);
        assert!(fresh.is_empty());
        let request = intent(chosen.clone(), PreserveExisting);
        assert_eq!(
            fresh.export("crash-key", revision(), &request, &original, 8192),
            Err(Error::Commit(ProfileFileCommitError::TargetExists)),
        );
        assert_eq!(fs::read(&chosen).expect("original file"), original.bytes());
    }

    #[test]
    fn two_member_export_import_refuses_valid_external_replacement() {
        let root = fixture();
        let chosen = root.0.join("full.atrament");
        let request = intent(chosen.clone(), PreserveExisting);
        let first_section = b"{\"ink\":1}";
        let second_section = b"{\"ink\":2}";
        let original = full_artifact(first_section);
        let replacement = full_artifact(second_section);
        assert_eq!(original.bytes().len(), replacement.bytes().len());
        assert_ne!(original.archive_sha256(), replacement.archive_sha256());
        let accepted = revision();
        let mut session = ProfileExportSession::new(1);
        assert!(matches!(
            session.export("full", accepted, &request, &original, 8192),
            Ok(Outcome::Committed(_)),
        ));
        let selected_import = classify_explicit_file_path_intent(
            ExplicitFilePurpose::ImportProfile,
            ExplicitFilePathSource::CallerSelection,
            chosen.clone(),
            PreserveExisting,
        )
        .expect("selected Import path");
        let verified_original = inspect_selected_profile_import(
            &selected_import,
            &[],
            import_limits(),
            |verified| {
                assert_eq!(verified.verified_entry_count(), 2);
                assert_eq!(
                    verified.verified_entry_bytes("sections/ink.json"),
                    Some(&first_section[..]),
                );
                assert_eq!(
                    verified.verified_entry_bytes("assets/sample.bin"),
                    Some(BINARY_ASSET),
                );
                verified.archive_sha256()
            },
        );
        assert_eq!(verified_original, Ok(original.archive_sha256()));

        // External file bytes are independently valid, but not the bytes
        // the original Export session committed under this retry key.
        fs::write(&chosen, replacement.bytes())
            .expect("external valid ZIP replacement");
        let verified_new = inspect_selected_profile_import(
            &selected_import,
            &[],
            import_limits(),
            |verified| {
                assert_eq!(
                    verified.verified_entry_bytes("sections/ink.json"),
                    Some(&second_section[..]),
                );
                verified.archive_sha256()
            },
        );
        assert_eq!(verified_new, Ok(replacement.archive_sha256()));
        assert_eq!(
            session.export("full", accepted, &request, &original, 8192),
            Err(Error::Retry(
                ProfileExportRetryRecoveryDecision::ExternalTargetDrift
            )),
        );
        assert_eq!(
            fs::read(&chosen).expect("unchanged external archive"),
            replacement.bytes(),
        );
        assert_eq!(fs::read_dir(&root.0).expect("one output").count(), 1);
    }

    #[test]
    fn no_persisted_receipt_survives_dropped_session() {
        let root = fixture();
        let chosen = root.0.join("selected.atrament");
        let request = intent(chosen.clone(), PreserveExisting);
        let original = artifact("writer-a");
        let accepted = revision();
        let mut session = ProfileExportSession::new(1);
        assert!(matches!(
            session.export("key", accepted, &request, &original, 8192),
            Ok(Outcome::Committed(_)),
        ));
        drop(session);
        let mut fresh_session = ProfileExportSession::new(1);
        assert_eq!(
            fresh_session.export("key", accepted, &request, &original, 8192),
            Err(Error::Commit(ProfileFileCommitError::TargetExists)),
        );
        assert_eq!(
            fs::read(&chosen).expect("prior explicit export remains"),
            original.bytes()
        );
    }
}

use std::path::PathBuf;

use atrament_explicit_file_path_intent::{
    ExplicitFilePathIntent, ExplicitFilePathSource, ExplicitFilePurpose,
    ExplicitOverwriteIntent, classify_explicit_file_path_intent,
};
use atrament_handwriting_profile_container::{
    PROFILE_CONTAINER_VERSION, ProfileManifest, ProfileManifestEntry,
};
use atrament_profile_entry_sha256::sha256_profile_bytes;
use atrament_profile_export_assembly::{
    ProfileExportArtifact, ProfileExportAssemblyLimits, ProfileExportPayload,
    assemble_profile_export_artifact,
};
use atrament_profile_export_retry_binding::ProfileRetryBindingError;
use atrament_profile_export_retry_recovery::ProfileExportRetryRecoveryDecision;
use atrament_profile_export_session::{
    ProfileExportSession, ProfileExportSessionError as Error,
    ProfileExportSessionOutcome as Outcome,
};
use atrament_profile_export_target_check as _;
use atrament_profile_file_commit::ProfileFileCommitError;
use atrament_profile_import_preflight::ProfileImportBudget;
use atrament_profile_json_syntax::ProfileJsonSyntaxLimits;
use atrament_profile_manifest_json_codec::ProfileManifestJsonLimits;
use atrament_profile_manifest_json_writer::ProfileManifestJsonWriteLimits;
use atrament_profile_open_file_import::{
    ProfileOpenFileImportError, ProfileOpenFileImportLimits,
    inspect_selected_profile_import,
};
use atrament_profile_stored_zip_writer::ProfileStoredZipWriteLimits;
use atrament_profile_zip_reader::ProfileZipReadLimits;
use atrament_semantic_notebook::{IdentityAllocator, RevisionIdentity};

const BINARY_ASSET: &[u8] = b"\x00\xff\x1bopaque-data";

fn artifact(identity: &str) -> ProfileExportArtifact {
    assemble_profile_export_artifact(
        &ProfileManifest {
            container_version: PROFILE_CONTAINER_VERSION.into(),
            entries: Vec::new(),
            optional_features: Vec::new(),
            profile_identity: identity.into(),
            required_features: Vec::new(),
        },
        &[],
        &[],
        ProfileExportAssemblyLimits {
            manifest_json: ProfileManifestJsonWriteLimits {
                maximum_bytes: 4096,
                maximum_entries: 2,
                maximum_features: 2,
                maximum_string_bytes: 128,
            },
            section_json: ProfileJsonSyntaxLimits {
                maximum_bytes: 4096,
                maximum_nesting: 32,
                maximum_object_members: 64,
                maximum_values: 1024,
            },
            zip: ProfileStoredZipWriteLimits {
                maximum_archive_bytes: 8192,
                maximum_entries: 2,
                maximum_entry_bytes: 4096,
                maximum_total_bytes: 4096,
            },
        },
    )
    .expect("complete canonical profile ZIP")
}

fn full_artifact(section_bytes: &[u8]) -> ProfileExportArtifact {
    let manifest = ProfileManifest {
        container_version: PROFILE_CONTAINER_VERSION.into(),
        entries: vec![
            ProfileManifestEntry {
                byte_length: u64::try_from(section_bytes.len())
                    .expect("section size fits u64"),
                digest: sha256_profile_bytes(section_bytes),
                media_type: String::from("application/json"),
                path: String::from("sections/ink.json"),
            },
            ProfileManifestEntry {
                byte_length: u64::try_from(BINARY_ASSET.len())
                    .expect("asset size fits u64"),
                digest: sha256_profile_bytes(BINARY_ASSET),
                media_type: String::from("application/octet-stream"),
                path: String::from("assets/sample.bin"),
            },
        ],
        optional_features: Vec::new(),
        profile_identity: String::from("complete-profile"),
        required_features: Vec::new(),
    };
    let payloads = [
        ProfileExportPayload {
            bytes: BINARY_ASSET,
            path: "assets/sample.bin",
        },
        ProfileExportPayload {
            bytes: section_bytes,
            path: "sections/ink.json",
        },
    ];
    assemble_profile_export_artifact(
        &manifest,
        &payloads,
        &[],
        ProfileExportAssemblyLimits {
            manifest_json: ProfileManifestJsonWriteLimits {
                maximum_bytes: 4096,
                maximum_entries: 2,
                maximum_features: 2,
                maximum_string_bytes: 128,
            },
            section_json: ProfileJsonSyntaxLimits {
                maximum_bytes: 4096,
                maximum_nesting: 32,
                maximum_object_members: 64,
                maximum_values: 1024,
            },
            zip: ProfileStoredZipWriteLimits {
                maximum_archive_bytes: 8192,
                maximum_entries: 3,
                maximum_entry_bytes: 4096,
                maximum_total_bytes: 4096,
            },
        },
    )
    .expect("complete two-member portable profile")
}

fn revision() -> RevisionIdentity {
    IdentityAllocator::new()
        .allocate_revision()
        .expect("revision identity")
}

fn intent(
    path: PathBuf,
    overwrite: ExplicitOverwriteIntent,
) -> ExplicitFilePathIntent {
    classify_explicit_file_path_intent(
        ExplicitFilePurpose::ExportProfile,
        ExplicitFilePathSource::CallerSelection,
        path,
        overwrite,
    )
    .expect("test-selected path classification")
}

const fn import_limits() -> ProfileOpenFileImportLimits {
    ProfileOpenFileImportLimits {
        import: ProfileImportBudget {
            maximum_declared_bytes: 4096,
            maximum_entries: 2,
            maximum_entry_bytes: 4096,
        },
        manifest: ProfileManifestJsonLimits {
            maximum_entries: 2,
            maximum_features: 2,
            maximum_string_bytes: 128,
        },
        maximum_file_bytes: 8192,
        root_json: ProfileJsonSyntaxLimits {
            maximum_bytes: 4096,
            maximum_nesting: 8,
            maximum_object_members: 20,
            maximum_values: 64,
        },
        section_json: ProfileJsonSyntaxLimits {
            maximum_bytes: 4096,
            maximum_nesting: 8,
            maximum_object_members: 20,
            maximum_values: 64,
        },
        zip: ProfileZipReadLimits {
            maximum_archive_bytes: 8192,
            maximum_entries: 3,
            maximum_entry_bytes: 4096,
            maximum_total_bytes: 4096,
        },
    }
}
