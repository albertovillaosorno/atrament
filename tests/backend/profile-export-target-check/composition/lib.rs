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
//   - Real-file read-only comparison fixtures against canonical profile ZIP.
// - Must-Not:
//   - Admit an Export retry or mutate files outside isolated fixtures.
// - Allows:
//   - Inputs: Fixture-owned selected paths, verified ZIP bytes and limits.
//   - Outputs: Exact match, drift, missing and unsafe-path evidence.
//   - Side effects: Test-only isolated files and cleanup.
// - Split-When:
//   - Session retry receipt state becomes an executable application feature.
// - Merge-When:
//   - End-to-end Export retry tests cover equivalent file observations.
// - Summary:
//   - Checks that recovery evidence never overwrites external file changes.
// - Description:
//   - Covers sealed archive identity, sizes, links and nonregular sources.
// - Usage:
//   - cargo test -p atrament_profile_export_target_check
// - Defaults:
//   - Untrusted file locations and unavailable files never prove a match.
//

//! Read-only Export target drift evidence against test-owned on-disk files.

#![expect(clippy::expect_used, reason = "test-owned fixture")]

#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use std::env;
    use std::fs;
    use std::os::unix::fs::{MetadataExt as _, symlink};
    use std::path::Path;
    use std::process::{self, Command};
    use std::sync::atomic::{AtomicU64, Ordering};

    use atrament_profile_file_commit::{
        ProfileFileCommitDisposition, commit_profile_file,
    };
    use atrament_profile_selected_file_open::ProfileSelectedFileOpenError;

    static NEXT: AtomicU64 = AtomicU64::new(0);
    const PROBE_CWD_ALTERNATE: &str = "ATRAMENT_TARGET_PROBE_CWD_ALTERNATE";
    const PROBE_CWD_TEST: &str =
        "linux::opened_relative_probe_refuses_changed_cwd_even_for_same_inode";

    struct OwnedDir(PathBuf);

    impl OwnedDir {
        #[expect(clippy::create_dir, reason = "unique test-only directory")]
        fn new() -> Self {
            let path = env::temp_dir().join(format!(
                "atrament-target-check-{}-{}",
                process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed),
            ));
            fs::create_dir(&path).expect("isolated test dir");
            Self(path)
        }

        fn target(&self) -> PathBuf {
            self.0.join("selected.atrament")
        }
    }

    #[expect(clippy::missing_trait_methods, reason = "test fixture cleanup")]
    impl Drop for OwnedDir {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).expect("remove test-owned dir");
        }
    }

    fn matching_status(
        profile: &ProfileExportArtifact,
        selected: &Path,
    ) -> Status {
        let metadata = fs::metadata(selected).expect("matched file metadata");
        Status::Matches {
            archive_sha256: profile.archive_sha256(),
            byte_length: profile.bytes().len(),
            file_device: metadata.dev(),
            file_inode: metadata.ino(),
        }
    }

    #[test]
    fn opened_probe_refuses_identical_bytes_after_pathname_rebinding() {
        let dir = OwnedDir::new();
        let original = artifact("private-rebound-profile");
        let selected = intent(dir.target(), ExplicitFilePurpose::ExportProfile);
        fs::write(selected.path(), original.bytes())
            .expect("selected original bytes");
        let opened = open_profile_export_target_probe(
            &selected,
            &original,
            original.bytes().len(),
        )
        .expect("secure first open")
        .expect("selected target exists");
        let debug = format!("{opened:?}");
        assert!(!debug.contains("private-rebound-profile"));
        assert!(!debug.contains("selected.atrament"));
        let parked = dir.0.join("original-inode");
        fs::rename(selected.path(), &parked).expect("move opened inode away");
        fs::write(selected.path(), original.bytes())
            .expect("substitute identical-byte target");
        assert_ne!(
            fs::metadata(&parked).expect("old inode").ino(),
            fs::metadata(selected.path()).expect("new inode").ino(),
        );
        assert_eq!(opened.verify(), Ok(Status::Drifted));
        // A fresh unbound byte check cannot attest a historical commit.
        assert_eq!(
            check_profile_export_target(
                &selected,
                &original,
                original.bytes().len(),
            ),
            Ok(matching_status(&original, selected.path()))
        );
        assert_eq!(
            fs::read(&parked).expect("parked original unchanged"),
            original.bytes(),
        );
    }

    #[test]
    fn opened_probe_refuses_symlink_swapped_after_secure_first_open() {
        let dir = OwnedDir::new();
        let original = artifact("A");
        let selected = intent(dir.target(), ExplicitFilePurpose::ExportProfile);
        fs::write(selected.path(), original.bytes())
            .expect("selected original bytes");
        let opened = open_profile_export_target_probe(
            &selected,
            &original,
            original.bytes().len(),
        )
        .expect("secure first open")
        .expect("selected target exists");
        let parked = dir.0.join("original-inode");
        fs::rename(selected.path(), &parked).expect("move opened inode away");
        symlink(&parked, selected.path()).expect("inject final symlink");
        assert_eq!(
            opened.verify(),
            Err(Error::Open(ProfileSelectedFileOpenError::SymlinkForbidden)),
        );
        assert_eq!(
            fs::read(&parked).expect("original remains intact"),
            original.bytes(),
        );
    }

    #[test]
    fn opened_probe_refuses_identical_bytes_after_parent_directory_replacement()
    {
        let dir = OwnedDir::new();
        let original = artifact("parent-replacement");
        let selected_parent = dir.0.join("selected-parent");
        fs::create_dir_all(&selected_parent).expect("first parent");
        let selected = intent(
            selected_parent.join("selected.atrament"),
            ExplicitFilePurpose::ExportProfile,
        );
        fs::write(selected.path(), original.bytes())
            .expect("original selected target");
        let opened = open_profile_export_target_probe(
            &selected,
            &original,
            original.bytes().len(),
        )
        .expect("open selected target under first parent")
        .expect("target exists");
        let parked_parent = dir.0.join("parked-parent");
        fs::rename(&selected_parent, &parked_parent)
            .expect("detach first parent");
        fs::create_dir_all(&selected_parent).expect("new selected parent");
        fs::write(selected.path(), original.bytes())
            .expect("identical ZIP in replacement directory");
        assert_ne!(
            fs::metadata(parked_parent.join("selected.atrament"))
                .expect("original inode")
                .ino(),
            fs::metadata(selected.path()).expect("new inode").ino(),
        );
        assert_eq!(opened.verify(), Ok(Status::Drifted));
        assert_eq!(
            fs::read(parked_parent.join("selected.atrament"))
                .expect("old target stays intact"),
            original.bytes(),
        );
    }

    #[test]
    fn opened_probe_refuses_parent_symlink_rebinding_to_original_inode() {
        let dir = OwnedDir::new();
        let original = artifact("parent-link-replacement");
        let selected_parent = dir.0.join("selected-parent");
        fs::create_dir_all(&selected_parent).expect("first parent");
        let selected = intent(
            selected_parent.join("selected.atrament"),
            ExplicitFilePurpose::ExportProfile,
        );
        fs::write(selected.path(), original.bytes())
            .expect("original selected target");
        let opened = open_profile_export_target_probe(
            &selected,
            &original,
            original.bytes().len(),
        )
        .expect("open selected target under first parent")
        .expect("target exists");
        let parked_parent = dir.0.join("parked-parent");
        fs::rename(&selected_parent, &parked_parent)
            .expect("detach first parent");
        symlink(&parked_parent, &selected_parent)
            .expect("alias selected parent to original target");
        assert_eq!(
            opened.verify(),
            Err(Error::Open(ProfileSelectedFileOpenError::SymlinkForbidden)),
        );
        assert_eq!(
            fs::read(parked_parent.join("selected.atrament"))
                .expect("original bytes stay intact"),
            original.bytes(),
        );
    }

    #[test]
    fn opened_probe_reports_missing_if_selected_target_removed_during_check() {
        let dir = OwnedDir::new();
        let original = artifact("A");
        let selected = intent(dir.target(), ExplicitFilePurpose::ExportProfile);
        fs::write(selected.path(), original.bytes())
            .expect("selected original bytes");
        let opened = open_profile_export_target_probe(
            &selected,
            &original,
            original.bytes().len(),
        )
        .expect("secure first open")
        .expect("selected target exists");
        fs::remove_file(selected.path()).expect("remove selected name");
        assert_eq!(opened.verify(), Ok(Status::Missing));
        assert!(!selected.path().exists());
    }

    #[test]
    fn opened_relative_probe_refuses_changed_cwd_even_for_same_inode() {
        if let Some(alternate) = env::var_os(PROBE_CWD_ALTERNATE) {
            let profile = artifact("relative-probe");
            let selected = intent(
                PathBuf::from("selected.atrament"),
                ExplicitFilePurpose::ExportProfile,
            );
            let original_probe = open_profile_export_target_probe(
                &selected,
                &profile,
                profile.bytes().len(),
            )
            .expect("open relative target under initial CWD")
            .expect("relative target exists");
            assert_eq!(
                original_probe.verify(),
                Ok(matching_status(&profile, selected.path())),
            );
            let after_move_probe = open_profile_export_target_probe(
                &selected,
                &profile,
                profile.bytes().len(),
            )
            .expect("open another probe in unchanged CWD")
            .expect("relative target exists");
            env::set_current_dir(alternate).expect("change child CWD");
            assert_eq!(
                after_move_probe.verify(),
                Err(Error::Open(
                    ProfileSelectedFileOpenError::WorkingDirectoryChanged,
                )),
            );
            return;
        }
        let dir = OwnedDir::new();
        let first = dir.0.join("first-cwd");
        let alternate = dir.0.join("second-cwd");
        fs::create_dir_all(&first).expect("first CWD");
        fs::create_dir_all(&alternate).expect("second CWD");
        let profile = artifact("relative-probe");
        let original = first.join("selected.atrament");
        let other = alternate.join("selected.atrament");
        fs::write(&original, profile.bytes()).expect("initial target");
        fs::hard_link(&original, &other).expect("same inode at other CWD");
        assert_eq!(
            fs::metadata(&original).expect("original inode").ino(),
            fs::metadata(&other).expect("other inode").ino(),
        );
        let output = Command::new(env::current_exe().expect("test executable"))
            .arg("--exact")
            .arg(PROBE_CWD_TEST)
            .env(PROBE_CWD_ALTERNATE, &alternate)
            .current_dir(&first)
            .output()
            .expect("isolated relative probe child");
        assert!(
            output.status.success(),
            "child refused changed CWD: {}",
            String::from_utf8_lossy(&output.stdout),
        );
        assert_eq!(
            fs::read(original).expect("original remains"),
            profile.bytes()
        );
        assert_eq!(fs::read(other).expect("hardlink remains"), profile.bytes());
    }

    #[test]
    fn absent_target_is_missing_then_atomic_commit_matches_exact_artifact() {
        let dir = OwnedDir::new();
        let profile = artifact("A");
        let selected = intent(dir.target(), ExplicitFilePurpose::ExportProfile);
        assert_eq!(
            check_profile_export_target(
                &selected,
                &profile,
                profile.bytes().len(),
            ),
            Ok(Status::Missing)
        );
        let _committed = commit_profile_file(
            selected.path(),
            profile.bytes(),
            profile.bytes().len(),
            ProfileFileCommitDisposition::PreserveExisting,
        )
        .expect("test file commit");
        assert_eq!(
            check_profile_export_target(
                &selected,
                &profile,
                profile.bytes().len(),
            ),
            Ok(matching_status(&profile, selected.path()))
        );
        assert_eq!(
            fs::read(selected.path()).expect("no source mutation"),
            profile.bytes()
        );
    }

    #[test]
    fn equal_length_mutation_and_longer_external_file_both_report_drift() {
        let dir = OwnedDir::new();
        let profile = artifact("A");
        let selected = intent(dir.target(), ExplicitFilePurpose::ExportProfile);
        let mut mutated = profile.bytes().to_vec();
        let index = mutated.len() - 1;
        *mutated.get_mut(index).expect("ZIP last byte") ^= 1;
        fs::write(selected.path(), &mutated).expect("same-size mutation");
        assert_eq!(
            check_profile_export_target(
                &selected,
                &profile,
                profile.bytes().len(),
            ),
            Ok(Status::Drifted)
        );
        mutated.push(0);
        fs::write(selected.path(), &mutated).expect("append external byte");
        assert_eq!(
            check_profile_export_target(
                &selected,
                &profile,
                profile.bytes().len(),
            ),
            Ok(Status::Drifted)
        );
        assert_eq!(
            fs::read(selected.path()).expect("read test-owned file"),
            mutated
        );
    }

    #[test]
    fn another_valid_profile_does_not_recover_a_prior_artifact() {
        let dir = OwnedDir::new();
        let first = artifact("A");
        let second = artifact("B");
        assert_eq!(first.bytes().len(), second.bytes().len());
        let selected = intent(dir.target(), ExplicitFilePurpose::ExportProfile);
        fs::write(selected.path(), second.bytes()).expect("external valid ZIP");
        let first_size = first.bytes().len();
        assert_eq!(
            check_profile_export_target(&selected, &first, first_size),
            Ok(Status::Drifted)
        );
        assert_eq!(
            check_profile_export_target(
                &selected,
                &second,
                second.bytes().len(),
            ),
            Ok(matching_status(&second, selected.path()))
        );
    }

    #[test]
    fn artifact_over_budget_refuses_even_when_file_is_missing() {
        let dir = OwnedDir::new();
        let profile = artifact("A");
        let selected = intent(dir.target(), ExplicitFilePurpose::ExportProfile);
        assert_eq!(
            check_profile_export_target(
                &selected,
                &profile,
                profile.bytes().len() - 1,
            ),
            Err(Error::ArtifactTooLarge)
        );
        assert!(!selected.path().exists());
    }

    #[test]
    fn wrong_purpose_refuses_before_any_source_lookup() {
        let dir = OwnedDir::new();
        let profile = artifact("A");
        for purpose in [
            ExplicitFilePurpose::ImportProfile,
            ExplicitFilePurpose::ImportNotebook,
            ExplicitFilePurpose::ExportNotebook,
        ] {
            let selected = intent(dir.target(), purpose);
            assert_eq!(
                check_profile_export_target(
                    &selected,
                    &profile,
                    profile.bytes().len(),
                ),
                Err(Error::WrongPathPurpose)
            );
        }
        assert!(!dir.target().exists());
    }

    #[test]
    fn final_symlink_is_refused_without_reading_or_following() {
        let dir = OwnedDir::new();
        let profile = artifact("A");
        let true_file = dir.0.join("actual");
        fs::write(&true_file, profile.bytes()).expect("target content");
        symlink(&true_file, dir.target()).expect("link selected path");
        let selected = intent(dir.target(), ExplicitFilePurpose::ExportProfile);
        assert_eq!(
            check_profile_export_target(
                &selected,
                &profile,
                profile.bytes().len(),
            ),
            Err(Error::Open(ProfileSelectedFileOpenError::SymlinkForbidden))
        );
        assert_eq!(
            fs::read(&true_file).expect("read test-owned file"),
            profile.bytes()
        );
    }

    #[test]
    fn parent_symlink_and_nonregular_targets_do_not_prove_matches() {
        let dir = OwnedDir::new();
        let profile = artifact("A");
        fs::write(dir.target(), profile.bytes()).expect("chosen bytes");
        let alias = dir.0.join("parent-link");
        symlink(&dir.0, &alias).expect("parent alias");
        let selected = intent(
            alias.join("selected.atrament"),
            ExplicitFilePurpose::ExportProfile,
        );
        assert_eq!(
            check_profile_export_target(
                &selected,
                &profile,
                profile.bytes().len(),
            ),
            Err(Error::Open(ProfileSelectedFileOpenError::SymlinkForbidden))
        );
        let directory_intent =
            intent(dir.0.clone(), ExplicitFilePurpose::ExportProfile);
        assert_eq!(
            check_profile_export_target(
                &directory_intent,
                &profile,
                profile.bytes().len(),
            ),
            Err(Error::Open(ProfileSelectedFileOpenError::NotRegularFile))
        );
        assert_eq!(
            fs::read(dir.target()).expect("read test-owned file"),
            profile.bytes()
        );
    }

    #[test]
    fn traversal_and_missing_parent_cannot_be_substituted_for_a_valid_file() {
        let dir = OwnedDir::new();
        let profile = artifact("A");
        fs::write(dir.target(), profile.bytes()).expect("chosen bytes");
        let traversal = intent(
            dir.0.join("subdir/../selected.atrament"),
            ExplicitFilePurpose::ExportProfile,
        );
        assert_eq!(
            check_profile_export_target(
                &traversal,
                &profile,
                profile.bytes().len(),
            ),
            Err(Error::Open(ProfileSelectedFileOpenError::ParentTraversal))
        );
        let missing = intent(
            dir.0.join("missing/selected.atrament"),
            ExplicitFilePurpose::ExportProfile,
        );
        assert_eq!(
            check_profile_export_target(
                &missing,
                &profile,
                profile.bytes().len(),
            ),
            Ok(Status::Missing)
        );
        assert_eq!(
            fs::read(dir.target()).expect("read test-owned file"),
            profile.bytes()
        );
    }

    #[test]
    fn opaque_selected_filename_is_compared_without_rewriting_it() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt as _;

        let dir = OwnedDir::new();
        let profile = artifact("A");
        let selected = intent(
            dir.0.join(OsString::from_vec(b"opaque-\xff".to_vec())),
            ExplicitFilePurpose::ExportProfile,
        );
        fs::write(selected.path(), profile.bytes()).expect("opaque file");
        assert!(matches!(
            check_profile_export_target(
                &selected,
                &profile,
                profile.bytes().len(),
            ),
            Ok(Status::Matches { .. })
        ));
        assert_eq!(
            fs::read(selected.path()).expect("read test-owned file"),
            profile.bytes()
        );
    }

    #[test]
    fn a_real_directory_never_counts_as_missing_or_drift() {
        let dir = OwnedDir::new();
        let profile = artifact("A");
        let directory = dir.0.join("selected.atrament");
        fs::create_dir_all(&directory).expect("test directory");
        let selected = intent(directory, ExplicitFilePurpose::ExportProfile);
        assert_eq!(
            check_profile_export_target(
                &selected,
                &profile,
                profile.bytes().len(),
            ),
            Err(Error::Open(ProfileSelectedFileOpenError::NotRegularFile))
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
    ProfileJsonSyntaxLimits, assemble_profile_export_artifact,
};
use atrament_profile_export_target_check::{
    ProfileExportTargetCheckError as Error,
    ProfileExportTargetStatus as Status, check_profile_export_target,
    open_profile_export_target_probe,
};
use atrament_profile_file_reader as _;
use atrament_profile_manifest_json_writer::ProfileManifestJsonWriteLimits;
use atrament_profile_stored_zip_writer::ProfileStoredZipWriteLimits;

#[cfg(not(target_os = "linux"))]
#[test]
fn unsupported_platform_refuses_read_only_target_check() {
    use atrament_profile_selected_file_open::ProfileSelectedFileOpenError;

    let profile = artifact("A");
    let selected = intent(
        PathBuf::from("selected.atrament"),
        ExplicitFilePurpose::ExportProfile,
    );
    let expected_size = profile.bytes().len();
    assert_eq!(
        check_profile_export_target(&selected, &profile, expected_size),
        Err(Error::Open(
            ProfileSelectedFileOpenError::UnsupportedOpenPolicy
        ))
    );
}

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
    .expect("canonical profile ZIP")
}

fn intent(
    path: PathBuf,
    purpose: ExplicitFilePurpose,
) -> ExplicitFilePathIntent {
    classify_explicit_file_path_intent(
        purpose,
        ExplicitFilePathSource::CallerSelection,
        path,
        ExplicitOverwriteIntent::PreserveExisting,
    )
    .expect("fixture classified path")
}
