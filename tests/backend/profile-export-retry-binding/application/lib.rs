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
//   - Retry key binding regression fixtures for exact profile Export inputs.
// - Must-Not:
//   - Export a file, issue a receipt or bind a real caller's authority.
// - Allows:
//   - Inputs: Test-only revision identities, selected paths and sealed ZIPs.
//   - Outputs: Stable new/same/conflict/budget outcomes without file effects.
//   - Side effects: Process-local fixture data only.
// - Split-When:
//   - Real in-session Export retries need executable file-effect regressions.
// - Merge-When:
//   - Complete Export session recovery covers retry identity and drift.
// - Summary:
//   - A reused key cannot silently switch normalized Export inputs.
// - Description:
//   - Tests path, revision, artifact, overwrite and source conflicts.
// - Usage:
//   - cargo test -p atrament_profile_export_retry_binding
// - Defaults:
//   - Bindings are memory-only and never evidence of an earlier write.
//

//! Pure profile retry binding invariants, without any filesystem effects.

#![expect(clippy::expect_used, reason = "test-owned fixture")]

use std::path::PathBuf;

use atrament_explicit_file_path_intent::{
    ExplicitFilePathIntent, ExplicitFilePathSource, ExplicitFilePurpose,
    ExplicitOverwriteIntent,
    classify_explicit_file_path_intent,
};
use atrament_handwriting_profile_container::{
    PROFILE_CONTAINER_VERSION, ProfileManifest,
};
use atrament_profile_export_assembly::{
    ProfileExportArtifact, ProfileExportAssemblyLimits,
    assemble_profile_export_artifact,
};
use atrament_profile_export_retry_binding::{
    MAXIMUM_PROFILE_RETRY_KEY_BYTES, MAXIMUM_PROFILE_RETRY_PATH_BYTES,
    ProfileExportRetryBindings, ProfileRetryBindingDisposition as Disposition,
    ProfileRetryBindingError as Error,
};
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
    .expect("canonical profile ZIP")
}

fn intent(
    path: PathBuf,
    purpose: ExplicitFilePurpose,
    source: ExplicitFilePathSource,
    overwrite: ExplicitOverwriteIntent,
) -> ExplicitFilePathIntent {
    classify_explicit_file_path_intent(purpose, source, path, overwrite)
        .expect("classified test path")
}

fn export_intent(path: &str) -> ExplicitFilePathIntent {
    intent(
        PathBuf::from(path),
        ExplicitFilePurpose::ExportProfile,
        ExplicitFilePathSource::CallerSelection,
        ExplicitOverwriteIntent::PreserveExisting,
    )
}

fn revisions() -> (RevisionIdentity, RevisionIdentity) {
    let allocator = IdentityAllocator::new();
    (
        allocator.allocate_revision().expect("first revision"),
        allocator.allocate_revision().expect("second revision"),
    )
}

#[test]
fn fresh_then_identical_key_never_implies_prior_export_effect() {
    let (revision, _) = revisions();
    let file = export_intent("selected.atrament");
    let zip = artifact("A");
    let mut journal = ProfileExportRetryBindings::new(2);
    assert!(journal.is_empty());
    assert_eq!(
        journal.bind("key", revision, &file, &zip),
        Ok(Disposition::NewBinding)
    );
    assert_eq!(
        journal.bind("key", revision, &file, &zip),
        Ok(Disposition::SameBinding)
    );
    assert_eq!(journal.len(), 1);
}

#[test]
fn switching_revision_on_same_key_is_conflict_and_does_not_insert() {
    let (first, second) = revisions();
    let file = export_intent("chosen.atrament");
    let zip = artifact("A");
    let mut journal = ProfileExportRetryBindings::new(2);
    assert_eq!(
        journal.bind("retry", first, &file, &zip),
        Ok(Disposition::NewBinding)
    );
    assert_eq!(
        journal.bind("retry", second, &file, &zip),
        Err(Error::ConflictingRequest)
    );
    assert_eq!(
        journal.bind("retry", first, &file, &zip),
        Ok(Disposition::SameBinding)
    );
    assert_eq!(journal.len(), 1);
}

#[test]
fn exact_target_bytes_and_distinct_retry_keys_stay_independent() {
    let (revision, _) = revisions();
    let file_a = export_intent("./selected.atrament");
    let file_b = export_intent("selected.atrament");
    let zip = artifact("A");
    let mut journal = ProfileExportRetryBindings::new(2);
    assert_eq!(
        journal.bind("a", revision, &file_a, &zip),
        Ok(Disposition::NewBinding)
    );
    assert_eq!(
        journal.bind("a", revision, &file_b, &zip),
        Err(Error::ConflictingRequest)
    );
    assert_eq!(
        journal.bind("b", revision, &file_b, &zip),
        Ok(Disposition::NewBinding)
    );
    assert_eq!(journal.len(), 2);
}

#[test]
fn replacing_an_artifact_with_another_valid_same_size_zip_is_conflict() {
    let (revision, _) = revisions();
    let file = export_intent("chosen.atrament");
    let original = artifact("A");
    let replacement = artifact("B");
    assert_eq!(original.bytes().len(), replacement.bytes().len());
    let mut journal = ProfileExportRetryBindings::new(1);
    assert_eq!(
        journal.bind("id", revision, &file, &original),
        Ok(Disposition::NewBinding)
    );
    assert_eq!(
        journal.bind("id", revision, &file, &replacement),
        Err(Error::ConflictingRequest)
    );
    assert_eq!(
        journal.bind("id", revision, &file, &original),
        Ok(Disposition::SameBinding)
    );
}

#[test]
fn overwrite_change_conflicts_even_if_content_and_path_are_identical() {
    let (revision, _) = revisions();
    let preserve = export_intent("chosen.atrament");
    let replace = intent(
        PathBuf::from("chosen.atrament"),
        ExplicitFilePurpose::ExportProfile,
        ExplicitFilePathSource::CallerSelection,
        ExplicitOverwriteIntent::ReplaceExisting,
    );
    let zip = artifact("A");
    let mut journal = ProfileExportRetryBindings::new(1);
    assert_eq!(
        journal.bind("id", revision, &preserve, &zip),
        Ok(Disposition::NewBinding)
    );
    assert_eq!(
        journal.bind("id", revision, &replace, &zip),
        Err(Error::ConflictingRequest)
    );
}

#[test]
fn selected_origin_classification_is_part_of_retry_identity() {
    let (revision, _) = revisions();
    let selected = export_intent("chosen.atrament");
    let policy = intent(
        PathBuf::from("chosen.atrament"),
        ExplicitFilePurpose::ExportProfile,
        ExplicitFilePathSource::AdmittedHostPolicy,
        ExplicitOverwriteIntent::PreserveExisting,
    );
    let zip = artifact("A");
    let mut journal = ProfileExportRetryBindings::new(1);
    assert_eq!(
        journal.bind("id", revision, &selected, &zip),
        Ok(Disposition::NewBinding)
    );
    assert_eq!(
        journal.bind("id", revision, &policy, &zip),
        Err(Error::ConflictingRequest)
    );
}

#[test]
fn wrong_purpose_and_invalid_keys_refuse_without_binding() {
    let (revision, _) = revisions();
    let zip = artifact("A");
    let import = intent(
        PathBuf::from("chosen.atrament"),
        ExplicitFilePurpose::ImportProfile,
        ExplicitFilePathSource::CallerSelection,
        ExplicitOverwriteIntent::PreserveExisting,
    );
    let export = export_intent("chosen.atrament");
    let mut journal = ProfileExportRetryBindings::new(1);
    assert_eq!(
        journal.bind("id", revision, &import, &zip),
        Err(Error::WrongPathPurpose)
    );
    for key in ["", "key\0bad"] {
        assert_eq!(
            journal.bind(key, revision, &export, &zip),
            Err(Error::InvalidRetryKey)
        );
    }
    let long_key = "z".repeat(MAXIMUM_PROFILE_RETRY_KEY_BYTES + 1);
    assert_eq!(
        journal.bind(&long_key, revision, &export, &zip),
        Err(Error::InvalidRetryKey)
    );
    assert!(journal.is_empty());
}

#[test]
fn capacity_exhaustion_never_overwrites_an_existing_retry_binding() {
    let (revision, _) = revisions();
    let zip = artifact("A");
    let file = export_intent("chosen.atrament");
    let mut journal = ProfileExportRetryBindings::new(1);
    assert_eq!(
        journal.bind("first", revision, &file, &zip),
        Ok(Disposition::NewBinding)
    );
    assert_eq!(
        journal.bind("second", revision, &file, &zip),
        Err(Error::JournalFull)
    );
    assert_eq!(
        journal.bind("first", revision, &file, &zip),
        Ok(Disposition::SameBinding)
    );
    assert_eq!(journal.len(), 1);
    let mut zero = ProfileExportRetryBindings::new(0);
    assert_eq!(
        zero.bind("id", revision, &file, &zip),
        Err(Error::JournalFull)
    );
    assert!(zero.is_empty());
}

#[test]
fn selected_path_budget_is_checked_before_insertion() {
    let (revision, _) = revisions();
    let zip = artifact("A");
    let path = "p".repeat(MAXIMUM_PROFILE_RETRY_PATH_BYTES + 1);
    let file = export_intent(&path);
    let mut journal = ProfileExportRetryBindings::new(1);
    assert_eq!(
        journal.bind("id", revision, &file, &zip),
        Err(Error::SelectedPathTooLong)
    );
    assert!(journal.is_empty());
}

#[test]
fn retry_key_limit_counts_bytes_and_accepts_exact_boundary() {
    let (revision, _) = revisions();
    let zip = artifact("A");
    let file = export_intent("chosen.atrament");
    let mut journal = ProfileExportRetryBindings::new(1);
    let boundary_key = "\u{e9}".repeat(64);
    assert_eq!(boundary_key.len(), MAXIMUM_PROFILE_RETRY_KEY_BYTES);
    let too_large = format!("{boundary_key}\u{e9}");
    assert_eq!(
        journal.bind(&too_large, revision, &file, &zip),
        Err(Error::InvalidRetryKey)
    );
    assert_eq!(
        journal.bind(&boundary_key, revision, &file, &zip),
        Ok(Disposition::NewBinding)
    );
    assert_eq!(journal.len(), 1);
}

#[test]
fn selected_path_limit_accepts_exact_boundary() {
    let (revision, _) = revisions();
    let zip = artifact("A");
    let file = export_intent(&"p".repeat(MAXIMUM_PROFILE_RETRY_PATH_BYTES));
    let mut journal = ProfileExportRetryBindings::new(1);
    assert_eq!(
        journal.bind("id", revision, &file, &zip),
        Ok(Disposition::NewBinding)
    );
    assert_eq!(journal.len(), 1);
}

#[cfg(unix)]
#[test]
fn opaque_filename_bytes_remain_distinct_in_retry_identity() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt as _;

    let (revision, _) = revisions();
    let zip = artifact("A");
    let opaque = intent(
        PathBuf::from(OsString::from_vec(b"opaque-\xff".to_vec())),
        ExplicitFilePurpose::ExportProfile,
        ExplicitFilePathSource::CallerSelection,
        ExplicitOverwriteIntent::PreserveExisting,
    );
    let different = intent(
        PathBuf::from(OsString::from_vec(b"opaque-\xfe".to_vec())),
        ExplicitFilePurpose::ExportProfile,
        ExplicitFilePathSource::CallerSelection,
        ExplicitOverwriteIntent::PreserveExisting,
    );
    let mut journal = ProfileExportRetryBindings::new(1);
    assert_eq!(
        journal.bind("id", revision, &opaque, &zip),
        Ok(Disposition::NewBinding)
    );
    assert_eq!(
        journal.bind("id", revision, &different, &zip),
        Err(Error::ConflictingRequest)
    );
    assert_eq!(
        journal.bind("id", revision, &opaque, &zip),
        Ok(Disposition::SameBinding)
    );
}
