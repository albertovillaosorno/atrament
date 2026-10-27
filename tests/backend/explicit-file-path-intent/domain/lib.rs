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
//   - Exhaustive regression evidence for explicit file-path intent sources.
// - Must-Not:
//   - Read, modify, or create any selected file or grant filesystem access.
// - Allows:
//   - Inputs: Every source, purpose, and overwrite combination.
//   - Outputs: Exact classification and rejection assertions.
//   - Side effects: Process-local test allocations only.
// - Split-When:
//   - A real host file-picker or file-commit fixture needs separate tests.
// - Merge-When:
//   - Explicit import/export integration tests subsume classification cases.
// - Summary:
//   - Prevents notebook prose from selecting an import/export filesystem path.
// - Description:
//   - Verifies all 48 combinations and exact selected-path preservation.
// - Usage:
//   - Run through the Rust workspace integration suite.
// - Defaults:
//   - Content-origin text cannot authorize either import or export.
//

//! Explicit file-path intent source and path-shape regression tests.

use std::path::PathBuf;

use atrament_explicit_file_path_intent::{
    ExplicitFilePathIntentError, ExplicitFilePathSource, ExplicitFilePurpose,
    ExplicitOverwriteIntent, classify_explicit_file_path_intent,
};

const PURPOSES: [ExplicitFilePurpose; 4] = [
    ExplicitFilePurpose::ImportNotebook,
    ExplicitFilePurpose::ImportProfile,
    ExplicitFilePurpose::ExportNotebook,
    ExplicitFilePurpose::ExportProfile,
];

const SOURCES: [ExplicitFilePathSource; 6] = [
    ExplicitFilePathSource::CallerSelection,
    ExplicitFilePathSource::AdmittedHostPolicy,
    ExplicitFilePathSource::NotebookOrSourceText,
    ExplicitFilePathSource::ModelResponseText,
    ExplicitFilePathSource::ClipboardText,
    ExplicitFilePathSource::DiagnosticText,
];

const OVERWRITES: [ExplicitOverwriteIntent; 2] = [
    ExplicitOverwriteIntent::PreserveExisting,
    ExplicitOverwriteIntent::ReplaceExisting,
];

#[test]
fn every_source_purpose_and_overwrite_combination_is_classified() {
    let mut cases = 0usize;
    for source in SOURCES {
        for purpose in PURPOSES {
            for overwrite in OVERWRITES {
                let path = PathBuf::from("notebooks/session.atrament");
                let classified = classify_explicit_file_path_intent(
                    purpose, source, path.clone(), overwrite,
                );
                cases += 1;
                if !matches!(
                    source,
                    ExplicitFilePathSource::CallerSelection
                        | ExplicitFilePathSource::AdmittedHostPolicy
                ) {
                    assert_eq!(
                        classified,
                        Err(ExplicitFilePathIntentError::UntrustedSource),
                        "untrusted path source={source:?} purpose={purpose:?}",
                    );
                } else if !purpose.is_export()
                    && matches!(
                        overwrite, ExplicitOverwriteIntent::ReplaceExisting
                    )
                {
                    assert_eq!(
                        classified,
                        Err(ExplicitFilePathIntentError::ImportCannotOverwrite),
                        "source={source:?} purpose={purpose:?}",
                    );
                } else {
                    let Ok(intent) = classified else {
                        panic!("trusted selection must classify: {source:?}");
                    };
                    assert_eq!(intent.path(), path.as_path());
                    assert_eq!(intent.purpose(), purpose);
                    assert_eq!(intent.source(), source);
                    assert_eq!(intent.overwrite(), overwrite);
                }
            }
        }
    }
    assert_eq!(cases, 48);
}

#[test]
fn selection_requires_a_nonempty_file_name_component() {
    for selected in [
        "", ".", "..", "/", "../..", "notes/", "./notes/", "notes/sub/",
        "notes/.", "notes/sub/.", "notes/..", "notes/sub/..",
    ] {
        for source in [
            ExplicitFilePathSource::CallerSelection,
            ExplicitFilePathSource::AdmittedHostPolicy,
        ] {
            for purpose in PURPOSES {
                let outcome = classify_explicit_file_path_intent(
                    purpose,
                    source,
                    PathBuf::from(selected),
                    ExplicitOverwriteIntent::PreserveExisting,
                );
                assert_eq!(
                    outcome,
                    Err(ExplicitFilePathIntentError::NoSelectedFile),
                    "path={selected:?} purpose={purpose:?} source={source:?}",
                );
            }
        }
    }
}

#[test]
fn embedded_nul_never_reaches_file_target_classification() {
    for value in ["\0", "target\0.atrament", "dir/\0name", "/tmp/a\0b"] {
        for source in [
            ExplicitFilePathSource::CallerSelection,
            ExplicitFilePathSource::AdmittedHostPolicy,
        ] {
            for purpose in PURPOSES {
                let result = classify_explicit_file_path_intent(
                    purpose,
                    source,
                    PathBuf::from(value),
                    ExplicitOverwriteIntent::PreserveExisting,
                );
                assert_eq!(
                    result,
                    Err(ExplicitFilePathIntentError::NulByteInPath),
                    "value={value:?} purpose={purpose:?}",
                );
            }
        }
    }
}

#[test]
fn selected_paths_remain_opaque_without_normalization() {
    let selected = PathBuf::from("./données/cuaderno ñ/plan🖋.atrament");
    let intent = classify_explicit_file_path_intent(
        ExplicitFilePurpose::ExportProfile,
        ExplicitFilePathSource::CallerSelection,
        selected.clone(),
        ExplicitOverwriteIntent::ReplaceExisting,
    )
    .expect("explicit path classification succeeds without file I/O");
    assert_eq!(intent.path(), selected.as_path());
}

#[cfg(unix)]
#[test]
fn selected_non_utf8_paths_are_not_converted_or_normalized() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt as _;

    let mut bytes = b"artifacts/snapshot-".to_vec();
    bytes.push(0xff);
    let selected = PathBuf::from(OsString::from_vec(bytes));
    let classified = classify_explicit_file_path_intent(
        ExplicitFilePurpose::ImportNotebook,
        ExplicitFilePathSource::AdmittedHostPolicy,
        selected.clone(),
        ExplicitOverwriteIntent::PreserveExisting,
    )
    .expect("path classification does not require UTF-8");
    assert_eq!(classified.path(), selected.as_path());
}
