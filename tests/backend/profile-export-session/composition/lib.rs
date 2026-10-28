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
    use std::fs;
    use std::os::unix::fs::{MetadataExt as _, symlink};
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::process;
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;
    use ExplicitOverwriteIntent::{PreserveExisting, ReplaceExisting};

    static NEXT: AtomicU64 = AtomicU64::new(0);

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
        assert_eq!(
            fs::read(&chosen).expect("new exact bytes"),
            original.bytes()
        );
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
    PROFILE_CONTAINER_VERSION, ProfileManifest,
};
use atrament_profile_export_assembly::{
    ProfileExportArtifact, ProfileExportAssemblyLimits,
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
use atrament_profile_manifest_json_writer::ProfileManifestJsonWriteLimits;
use atrament_profile_stored_zip_writer::ProfileStoredZipWriteLimits;
use atrament_semantic_notebook::{IdentityAllocator, RevisionIdentity};

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
