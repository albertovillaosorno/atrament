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
//   - Transport-neutral source and path-shape classification for explicit
//     notebook or handwriting-profile import and export intent.
// - Must-Not:
//   - Grant filesystem authority, follow links, parse notebook prose as
//     permission, serialize documents, read files, or commit an export.
// - Allows:
//   - Inputs: An independently classified intent source, exact selected path,
//     file purpose, and caller-specified overwrite disposition.
//   - Outputs: An immutable classification or typed rejection before any I/O.
//   - Side effects: None.
// - Split-When:
//   - Host path selection or file-commit adapters need separate authority.
// - Merge-When:
//   - A complete explicit import/export admission owner subsumes this gate.
// - Summary:
//   - Keeps notebook text from becoming a filesystem target by implication.
// - Description:
//   - Rejects untrusted intent sources and malformed file-path requests.
// - Usage:
//   - Classify source authority at a trusted caller or host boundary first.
// - Defaults:
//   - Untrusted text and unselected paths grant no file operation intent.
//

//! Read-only intent classification for explicit file import and export paths.

use std::path::{MAIN_SEPARATOR_STR, Path, PathBuf};

/// Operation and file family for an explicitly selected local path.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExplicitFilePurpose {
    /// Save an accepted notebook artifact to an explicitly selected path.
    ExportNotebook,
    /// Save an `.atrament` handwriting profile to an explicit path.
    ExportProfile,
    /// Open a notebook bundle explicitly chosen by the caller.
    ImportNotebook,
    /// Open an `.atrament` handwriting profile selected by the caller.
    ImportProfile,
}

impl ExplicitFilePurpose {
    /// Whether this purpose requests a persistent file commit.
    #[must_use]
    pub const fn is_export(self) -> bool {
        matches!(self, Self::ExportNotebook | Self::ExportProfile)
    }
}

/// Where the trusted adapter established the path selection request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExplicitFilePathSource {
    /// A separate, admitted host policy selected the target independently.
    AdmittedHostPolicy,
    /// A deliberate operating-system picker or equivalent caller selection.
    CallerSelection,
    /// Content that arrived through the clipboard.
    ClipboardText,
    /// An error, diagnostic, or remediation description.
    DiagnosticText,
    /// Text returned by a model before or after notebook acceptance.
    ModelResponseText,
    /// Text occurring in source material or an accepted notebook.
    NotebookOrSourceText,
}

/// Whether the caller independently admitted replacement of an existing file.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExplicitOverwriteIntent {
    /// Preserve any existing target rather than replacing it.
    PreserveExisting,
    /// Explicit caller or admitted host policy permits replacement.
    ReplaceExisting,
}

/// Rejection before a file path can participate in import/export preflight.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExplicitFilePathIntentError {
    /// Import has no file-overwrite effect to authorize.
    ImportCannotOverwrite,
    /// A directory-marked or nameless target cannot identify a file.
    NoSelectedFile,
    /// An operating-system path contains a forbidden embedded NUL byte.
    NulByteInPath,
    /// A document, model, clipboard, or diagnostic cannot select a path.
    UntrustedSource,
}

/// One immutable, source-classified request; it does not authorize I/O.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExplicitFilePathIntent {
    overwrite: ExplicitOverwriteIntent,
    path: PathBuf,
    purpose: ExplicitFilePurpose,
    source: ExplicitFilePathSource,
}

impl ExplicitFilePathIntent {
    /// Return the independently supplied overwrite intent.
    #[must_use]
    pub const fn overwrite(&self) -> ExplicitOverwriteIntent {
        self.overwrite
    }

    /// Return the exact, non-normalized path selected by the trusted adapter.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Return the requested import or export purpose.
    #[must_use]
    pub const fn purpose(&self) -> ExplicitFilePurpose {
        self.purpose
    }

    /// Return the independently established path-intent source.
    #[must_use]
    pub const fn source(&self) -> ExplicitFilePathSource {
        self.source
    }
}

/// Classify path intent without touching the filesystem or granting access.
///
/// The caller is responsible for establishing that `source` actually arose
/// from a trusted selection action or admitted host policy. In particular,
/// passing content-derived text as `CallerSelection` cannot make it trusted.
/// File adapters must still check revision, capability, path policy, symlinks,
/// format, concurrency, overwrite safety, and commit results independently.
///
/// # Errors
///
/// Rejects untrusted sources, NUL-containing or directory-marked paths, and
/// import requests that incorrectly attempt to authorize overwriting a file.
pub fn classify_explicit_file_path_intent(
    purpose: ExplicitFilePurpose,
    source: ExplicitFilePathSource,
    path: PathBuf,
    overwrite: ExplicitOverwriteIntent,
) -> Result<ExplicitFilePathIntent, ExplicitFilePathIntentError> {
    if !matches!(
        source,
        ExplicitFilePathSource::CallerSelection
            | ExplicitFilePathSource::AdmittedHostPolicy
    ) {
        return Err(ExplicitFilePathIntentError::UntrustedSource);
    }
    let path_bytes = path.as_os_str().as_encoded_bytes();
    if path_bytes.contains(&0) {
        return Err(ExplicitFilePathIntentError::NulByteInPath);
    }
    let selected_directory = path_bytes.ends_with(
        MAIN_SEPARATOR_STR.as_bytes(),
    ) || cfg!(windows) && path_bytes.ends_with(b"/");
    if path.file_name().is_none() || selected_directory {
        return Err(ExplicitFilePathIntentError::NoSelectedFile);
    }
    if !purpose.is_export()
        && matches!(overwrite, ExplicitOverwriteIntent::ReplaceExisting)
    {
        return Err(ExplicitFilePathIntentError::ImportCannotOverwrite);
    }
    Ok(ExplicitFilePathIntent {
        overwrite,
        path,
        purpose,
        source,
    })
}
