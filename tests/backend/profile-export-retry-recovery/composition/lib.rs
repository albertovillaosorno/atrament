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
//   - Pure retry-adjudication regression fixtures for sealed profile Export.
// - Must-Not:
//   - Issue a receipt, authenticate selection or execute a file effect.
// - Allows:
//   - Inputs: Test-owned sealed archives, prior outcome and target evidence.
//   - Outputs: Exact same-key replay, drift and uncertainty decisions.
//   - Side effects: Process-local fixture objects only.
// - Split-When:
//   - Trusted Export session integration needs live filesystem evidence.
// - Merge-When:
//   - End-to-end Export owns session receipts and retry admission.
// - Summary:
//   - Missing, uncertain or conflicting evidence never becomes a replay.
// - Description:
//   - Tests before-commit, postcommit, target drift and byte disagreement.
// - Usage:
//   - cargo test -p atrament_profile_export_retry_recovery
// - Defaults:
//   - A verified prior effect still requires a trusted saved receipt.
//

//! Exact Export retry recovery decision matrix, without file side effects.

#![expect(clippy::expect_used, reason = "test-owned fixture")]

use atrament_handwriting_profile_container::{
    PROFILE_CONTAINER_VERSION, ProfileManifest,
};
use atrament_profile_export_assembly::{
    ProfileExportArtifact, ProfileExportAssemblyLimits,
    ProfileJsonSyntaxLimits, assemble_profile_export_artifact,
};
use atrament_profile_export_retry_binding::{
    ProfileRetryBindingDisposition as Binding, ProfileRetryBindingError,
};
use atrament_profile_export_retry_recovery::{
    ProfileExportPriorEffect as Prior,
    ProfileExportRetryRecoveryDecision as Decision,
    assess_profile_export_retry_recovery as assess,
};
use atrament_profile_export_target_check::{
    ProfileExportTargetCheckError, ProfileExportTargetStatus as Status,
};
use atrament_profile_file_commit::ProfileFileCommitEvidence;
use atrament_profile_manifest_json_writer::ProfileManifestJsonWriteLimits;
use atrament_profile_stored_zip_writer::ProfileStoredZipWriteLimits;

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
    .expect("sealed artifact")
}

fn commit(zip: &ProfileExportArtifact) -> Prior {
    Prior::SyncedCommit(ProfileFileCommitEvidence {
        byte_length: zip.bytes().len(),
        file_device: 11,
        file_inode: 22,
    })
}

fn matches(zip: &ProfileExportArtifact) -> Status {
    Status::Matches {
        archive_sha256: zip.archive_sha256(),
        byte_length: zip.bytes().len(),
        file_device: 11,
        file_inode: 22,
    }
}

#[test]
fn new_binding_never_recovers_even_if_a_matching_file_is_visible() {
    let zip = artifact("A");
    assert_eq!(
        assess(
            Ok(Binding::NewBinding),
            commit(&zip),
            Some(Ok(matches(&zip))),
            &zip
        ),
        Decision::FreshBinding
    );
}

#[test]
fn conflicting_binding_cannot_be_recovered_by_matching_bytes() {
    let zip = artifact("A");
    assert_eq!(
        assess(
            Err(ProfileRetryBindingError::ConflictingRequest),
            commit(&zip),
            Some(Ok(matches(&zip))),
            &zip
        ),
        Decision::BindingRefused(ProfileRetryBindingError::ConflictingRequest)
    );
}

#[test]
fn identical_binding_without_a_trusted_outcome_does_not_recover() {
    let zip = artifact("A");
    assert_eq!(
        assess(
            Ok(Binding::SameBinding),
            Prior::NotRecorded,
            Some(Ok(matches(&zip))),
            &zip
        ),
        Decision::PriorOutcomeNotRecorded
    );
    assert_eq!(
        assess(
            Ok(Binding::SameBinding),
            Prior::KnownNoCommit,
            Some(Ok(matches(&zip))),
            &zip
        ),
        Decision::PriorKnownNoCommit
    );
}

#[test]
fn unknown_postrename_durability_cannot_be_upgraded_by_matching_bytes() {
    let zip = artifact("A");
    assert_eq!(
        assess(
            Ok(Binding::SameBinding),
            Prior::DurabilityUnknown,
            Some(Ok(matches(&zip))),
            &zip
        ),
        Decision::PriorDurabilityUnknown
    );
}

#[test]
fn synced_prior_commit_requires_a_fresh_read_and_exact_match() {
    let zip = artifact("A");
    assert_eq!(
        assess(Ok(Binding::SameBinding), commit(&zip), None, &zip),
        Decision::TargetInspectionRequired
    );
    assert_eq!(
        assess(
            Ok(Binding::SameBinding),
            commit(&zip),
            Some(Ok(matches(&zip))),
            &zip
        ),
        Decision::PriorCommitBytesVerified
    );
}

#[test]
fn drift_or_missing_target_never_replays_or_rewrites() {
    let zip = artifact("A");
    for status in [Status::Missing, Status::Drifted] {
        assert_eq!(
            assess(
                Ok(Binding::SameBinding),
                commit(&zip),
                Some(Ok(status)),
                &zip
            ),
            Decision::ExternalTargetDrift
        );
    }
}

#[test]
fn unsafe_target_check_is_not_a_drift_or_success_claim() {
    let zip = artifact("A");
    assert_eq!(
        assess(
            Ok(Binding::SameBinding),
            commit(&zip),
            Some(Err(ProfileExportTargetCheckError::WrongPathPurpose)),
            &zip
        ),
        Decision::TargetInspectionFailed(
            ProfileExportTargetCheckError::WrongPathPurpose
        )
    );
}

#[test]
fn forged_commit_length_or_target_match_does_not_recover() {
    let zip = artifact("A");
    let replacement = artifact("B");
    assert_eq!(zip.bytes().len(), replacement.bytes().len());
    assert_eq!(
        assess(
            Ok(Binding::SameBinding),
            Prior::SyncedCommit(ProfileFileCommitEvidence {
                byte_length: 0,
                file_device: 11,
                file_inode: 22,
            }),
            Some(Ok(matches(&zip))),
            &zip
        ),
        Decision::InconsistentEvidence
    );
    assert_eq!(
        assess(
            Ok(Binding::SameBinding),
            commit(&zip),
            Some(Ok(matches(&replacement))),
            &zip
        ),
        Decision::InconsistentEvidence
    );
    assert_eq!(
        assess(
            Ok(Binding::SameBinding),
            commit(&zip),
            Some(Ok(Status::Matches {
                archive_sha256: zip.archive_sha256(),
                byte_length: 0,
                file_device: 11,
                file_inode: 22,
            })),
            &zip
        ),
        Decision::InconsistentEvidence
    );
}

#[test]
fn matching_bytes_under_another_inode_cannot_recover_a_synced_commit() {
    let zip = artifact("A");
    for (device, inode) in [(11, 23), (12, 22)] {
        assert_eq!(
            assess(
                Ok(Binding::SameBinding),
                commit(&zip),
                Some(Ok(Status::Matches {
                    archive_sha256: zip.archive_sha256(),
                    byte_length: zip.bytes().len(),
                    file_device: device,
                    file_inode: inode,
                })),
                &zip,
            ),
            Decision::ExternalTargetDrift,
        );
    }
}

#[cfg(target_os = "linux")]
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one committed file, drift and symlink replay lifecycle"
)]
fn real_file_commit_then_exact_recovery_and_external_drift() {
    use std::env;
    use std::fs;
    use std::os::unix::fs::symlink;
    use std::path::PathBuf;
    use std::process;
    use std::sync::atomic::{AtomicU64, Ordering};

    use atrament_explicit_file_path_intent::{
        ExplicitFilePathSource, ExplicitFilePurpose, ExplicitOverwriteIntent,
        classify_explicit_file_path_intent,
    };
    use atrament_profile_export_retry_binding::ProfileExportRetryBindings;
    use atrament_profile_export_target_check::check_profile_export_target;
    use atrament_profile_file_commit::{
        ProfileFileCommitDisposition, commit_profile_file,
    };
    use atrament_semantic_notebook::IdentityAllocator;

    static NEXT: AtomicU64 = AtomicU64::new(0);

    struct OwnedDir(PathBuf);
    #[expect(
        clippy::missing_trait_methods,
        reason = "fixture cleanup is only implemented through Drop"
    )]
    impl Drop for OwnedDir {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).expect("remove owned fixture");
        }
    }
    #[expect(clippy::create_dir, reason = "unique test fixture must not exist")]
    fn create_owned_dir(root: &PathBuf) {
        fs::create_dir(root).expect("unique fixture directory");
    }
    let root = env::temp_dir().join(format!(
        "atrament-recovery-{}-{}",
        process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed),
    ));
    create_owned_dir(&root);
    let _owned = OwnedDir(root.clone());
    let path = root.join("chosen.atrament");
    let intent = classify_explicit_file_path_intent(
        ExplicitFilePurpose::ExportProfile,
        ExplicitFilePathSource::CallerSelection,
        path.clone(),
        ExplicitOverwriteIntent::PreserveExisting,
    )
    .expect("selected path classification");
    let allocator = IdentityAllocator::new();
    let revision = allocator
        .allocate_revision()
        .expect("accepted revision identity");
    let other_revision = allocator
        .allocate_revision()
        .expect("another accepted revision identity");
    let zip = artifact("A");
    let mut bindings = ProfileExportRetryBindings::new(1);
    assert_eq!(
        bindings.bind("key", revision, &intent, &zip),
        Ok(Binding::NewBinding)
    );
    let evidence = commit_profile_file(
        &path,
        zip.bytes(),
        zip.bytes().len(),
        ProfileFileCommitDisposition::PreserveExisting,
    )
    .expect("synced complete file commit");
    let bound = bindings.bind("key", revision, &intent, &zip);
    let current = check_profile_export_target(&intent, &zip, zip.bytes().len());
    assert_eq!(
        assess(bound, Prior::SyncedCommit(evidence), Some(current), &zip),
        Decision::PriorCommitBytesVerified
    );
    // Matching complete bytes do not forgive a conflicting revision retry.
    assert_eq!(
        assess(
            bindings.bind("key", other_revision, &intent, &zip),
            Prior::SyncedCommit(evidence),
            Some(Ok(matches(&zip))),
            &zip
        ),
        Decision::BindingRefused(ProfileRetryBindingError::ConflictingRequest)
    );
    fs::write(&path, b"external change").expect("simulate external drift");
    let observed =
        check_profile_export_target(&intent, &zip, zip.bytes().len());
    assert_eq!(
        assess(
            bindings.bind("key", revision, &intent, &zip),
            Prior::SyncedCommit(evidence),
            Some(observed),
            &zip
        ),
        Decision::ExternalTargetDrift
    );
    assert_eq!(
        fs::read(&path).expect("read external change"),
        b"external change"
    );

    // A path swapped for a link cannot become recovered output evidence.
    let linked = root.join("unrelated.atrament");
    fs::write(&linked, b"must remain intact").expect("seed other file");
    fs::remove_file(&path).expect("remove externally changed target");
    symlink(&linked, &path).expect("substitute a link after commit");
    let inspected =
        check_profile_export_target(&intent, &zip, zip.bytes().len());
    assert!(matches!(
        assess(
            bindings.bind("key", revision, &intent, &zip),
            Prior::SyncedCommit(evidence),
            Some(inspected),
            &zip
        ),
        Decision::TargetInspectionFailed(
            ProfileExportTargetCheckError::Open(_)
        )
    ));
    assert_eq!(
        fs::read(&linked).expect("read linked file"),
        b"must remain intact"
    );
}
