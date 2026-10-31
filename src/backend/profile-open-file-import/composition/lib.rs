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
//   - Selected Linux source opening and bounded Import verification.
// - Must-Not:
//   - Infer selection authority, decode profile sections, accept session
//     state, or write persistent output.
// - Allows:
//   - Inputs: Classified Import intent, optional file-source callback, limits.
//   - Outputs: Verified ZIP/manifest/SHA, section syntax or typed refusal.
//   - Side effects: Opens selected paths only through secure platform policy;
//     invokes a caller-provided bounded reader on admitted source callbacks.
// - Split-When:
//   - Host selection/open policy and profile acceptance become implemented.
// - Merge-When:
//   - Complete profile Import owns source selection and accepted state.
// - Summary:
//   - Every declared member must verify before inspection is possible.
// - Description:
//   - Readiness is not a file-selection capability or session acceptance.
// - Usage:
//   - Host must authenticate the classified source; selected entrypoint
//     binds its exact path to the opened file without trusting callbacks.
// - Defaults:
//   - Wrong purpose never invokes the file-source callback.
//

//! Selected-path profile Import with bounded section syntax verification.

use std::fmt;
use std::fs::{File, Metadata};

use atrament_explicit_file_path_intent::{
    ExplicitFilePathIntent, ExplicitFilePurpose,
};
#[cfg(test)]
use atrament_handwriting_profile_container as _;
#[cfg(test)]
use atrament_profile_entry_sha256 as _;
use atrament_profile_file_reader::{
    ProfileFileReadError, ProfileOpenedFileBytes, read_bounded_profile_file,
};
use atrament_profile_import_preflight::ProfileImportBudget;
use atrament_profile_import_verification::{
    ProfileImportVerificationError, VerifiedAttachedProfileImport,
    verify_attached_profile_import,
};
use atrament_profile_json_syntax::{
    ProfileJsonSyntaxError, ProfileJsonSyntaxLimits,
    validate_profile_json_object,
};
use atrament_profile_manifest_json_codec::{
    ProfileManifestJsonError, ProfileManifestJsonLimits,
    decode_attached_profile_manifest_json,
};
use atrament_profile_selected_file_open::{
    ProfileSelectedFileOpenError, open_selected_profile_file,
    open_selected_profile_file_from_cwd,
};
#[cfg(test)]
use atrament_profile_stored_zip_writer as _;
use atrament_profile_zip_reader::{
    ProfileZipManifestSyntaxError, ProfileZipReadError, ProfileZipReadLimits,
    read_stored_profile_zip,
};

/// Transport, ZIP, JSON, schema and member-verification ceilings.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProfileOpenFileImportLimits {
    /// Declared non-root member count and byte ceilings.
    pub import: ProfileImportBudget,
    /// Typed manifest entry, feature and string bounds.
    pub manifest: ProfileManifestJsonLimits,
    /// Maximum actual source file length before any ZIP allocation.
    pub maximum_file_bytes: usize,
    /// Root JSON bytes, members, values and nesting ceilings.
    pub root_json: ProfileJsonSyntaxLimits,
    /// Independent syntax ceilings for each declared `sections/` member.
    pub section_json: ProfileJsonSyntaxLimits,
    /// ZIP envelope and member byte bounds.
    pub zip: ProfileZipReadLimits,
}

/// Refusal without exposing any partially verified profile entries.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProfileOpenFileImportError {
    /// A bounded inventory slot reservation failed.
    AllocationFailed,
    /// ZIP-root-backed JSON failed typed schema decoding.
    Manifest(ProfileManifestJsonError),
    /// A verified declared section was unexpectedly unavailable.
    MissingVerifiedSection {
        /// Original manifest index with the absent verified payload.
        index: usize,
    },
    /// Secure opening of the exact selected path failed.
    Open(ProfileSelectedFileOpenError),
    /// The exact ZIP root is not a valid bounded JSON object.
    RootJson(ProfileZipManifestSyntaxError),
    /// A digest-verified section was not a bounded JSON object.
    SectionJson {
        /// Original manifest declaration index of the invalid section.
        index: usize,
        /// Syntax or resource ceiling failure before callback invocation.
        reason: ProfileJsonSyntaxError,
    },
    /// Declared section media type is not the first-release JSON type.
    SectionMediaType {
        /// Original manifest index, without reflecting untrusted type text.
        index: usize,
    },
    /// The securely reopened selected name no longer matches the source.
    SelectedSourceChanged,
    /// The trusted host-opened source could not be read completely.
    Source(ProfileFileReadError),
    /// Features, inventory, declared budgets or SHA-256 did not match.
    Verification(ProfileImportVerificationError),
    /// A notebook or Export path cannot import a profile.
    WrongPathPurpose,
    /// ZIP CRC, framing, member order or resource admission failed.
    Zip(ProfileZipReadError),
}

/// An opened selected profile, bound to its initial pathname observation.
///
/// The probe can be consumed only once; a successful callback still requires
/// complete ZIP, manifest, member-SHA and section JSON admission. A later
/// secure pathname reopen must identify the same inode and metadata before
/// verified content can reach the callback. This is race detection, not a
/// locked pathname or a snapshot of a concurrently modified file.
#[must_use]
pub struct ProfileSelectedImportProbe<'selection> {
    intent: &'selection ExplicitFilePathIntent,
    opened: File,
    original_metadata: Metadata,
    selected_cwd: Option<File>,
}

impl fmt::Debug for ProfileSelectedImportProbe<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProfileSelectedImportProbe")
            .finish_non_exhaustive()
    }
}

impl ProfileSelectedImportProbe<'_> {
    /// Fully verify and recheck a selected source before invoking inspection.
    ///
    /// Reopening only after every ZIP and payload/section check avoids giving
    /// the caller even a previously valid payload if the selected name moved
    /// while verification ran. Relative paths retain the original CWD anchor.
    /// The final observation cannot prevent a future pathname mutation.
    ///
    /// # Errors
    ///
    /// Refuses failed source, ZIP, manifest, SHA, section or pathname checks.
    pub fn inspect<R, Inspect>(
        self,
        supported_required_features: &[&str],
        limits: ProfileOpenFileImportLimits,
        inspect: Inspect,
    ) -> Result<R, ProfileOpenFileImportError>
    where
        Inspect: FnOnce(&VerifiedAttachedProfileImport<'_, '_, '_, '_>) -> R,
    {
        let Self {
            intent,
            opened,
            original_metadata,
            selected_cwd,
        } = self;
        // Retain the original descriptor through verification so the
        // selected inode cannot be reclaimed and reused during the recheck.
        let readable = opened.try_clone().map_err(|error| {
            ProfileOpenFileImportError::Source(
                ProfileFileReadError::FileIo(error.kind()),
            )
        })?;
        inspect_open_profile_import(
            intent,
            |maximum| read_bounded_profile_file(readable, maximum),
            supported_required_features,
            limits,
            |verified| {
                let current = selected_cwd
                    .as_ref()
                    .map_or_else(
                        || open_selected_profile_file(intent.path()),
                        |cwd| {
                            open_selected_profile_file_from_cwd(
                                intent.path(), cwd,
                            )
                        },
                    )
                    .map_err(ProfileOpenFileImportError::Open)?;
                let current_metadata = current.metadata().map_err(|error| {
                    ProfileOpenFileImportError::Open(
                        ProfileSelectedFileOpenError::FileIo(error.kind()),
                    )
                })?;
                let opened_metadata = opened.metadata().map_err(|error| {
                    ProfileOpenFileImportError::Open(
                        ProfileSelectedFileOpenError::FileIo(error.kind()),
                    )
                })?;
                if !same_selected_source_metadata(
                    &original_metadata,
                    &opened_metadata,
                ) || !same_selected_source_metadata(
                    &original_metadata,
                    &current_metadata,
                ) {
                    return Err(
                        ProfileOpenFileImportError::SelectedSourceChanged
                    );
                }
                Ok(inspect(verified))
            },
        )?
    }
}

/// Inspect all verified bytes from an independently opened profile file.
///
/// The trusted host must prove its source was opened from `intent.path()`,
/// with appropriate link, race and path policy. The callback must return
/// a `ProfileOpenedFileBytes` produced by the outbound file reader, not a
/// path string. Wrong purpose refuses before invoking that callback.
/// Source length is checked against independent transport/ZIP ceilings.
/// The strictest ZIP, Import or schema count bound is enforced before
/// ZIP member-index allocation; excess returns ZIP `ResourceLimit`.
/// ZIP framing, JSON, manifest, inventory and every payload SHA-256 must all
/// pass before any declared `sections/` member is checked for its exact
/// `application/json` media type and bounded JSON object syntax. The callback
/// runs only after **all** sections pass both checks;
/// opaque `assets/` members remain byte-verified but uninterpreted. This is
/// not a typed handwriting section schema. Verified borrows cannot escape.
/// Same-length concurrent file rewrites do not prove a stable disk snapshot.
/// This is not a session profile acceptance or persistent Export operation.
///
/// # Errors
///
/// Refuses invalid purpose without reading, and fails on any source, archive,
/// schema or payload mismatch without inspecting partially admitted members.
pub fn inspect_open_profile_import<R, Source, Inspect>(
    intent: &ExplicitFilePathIntent,
    read: Source,
    supported_required_features: &[&str],
    limits: ProfileOpenFileImportLimits,
    inspect: Inspect,
) -> Result<R, ProfileOpenFileImportError>
where
    Source:
        FnOnce(usize) -> Result<ProfileOpenedFileBytes, ProfileFileReadError>,
    Inspect: FnOnce(&VerifiedAttachedProfileImport<'_, '_, '_, '_>) -> R,
{
    if intent.purpose() != ExplicitFilePurpose::ImportProfile {
        return Err(ProfileOpenFileImportError::WrongPathPurpose);
    }
    let source = read(
        limits
            .maximum_file_bytes
            .min(limits.zip.maximum_archive_bytes),
    )
    .map_err(ProfileOpenFileImportError::Source)?;
    if source.bytes().len()
        > limits
            .maximum_file_bytes
            .min(limits.zip.maximum_archive_bytes)
    {
        return Err(ProfileOpenFileImportError::Source(
            ProfileFileReadError::FileTooLarge,
        ));
    }
    // The ZIP index is an allocation-bearing boundary. Do not build an
    // index that the tighter Import or typed-manifest count budget
    // must reject after parsing; reserve one slot for the root.
    let maximum_entries = limits.zip.maximum_entries.min(
        limits
            .import
            .maximum_entries
            .min(limits.manifest.maximum_entries)
            .saturating_add(1),
    );
    let zip_limits = ProfileZipReadLimits {
        maximum_entries,
        ..limits.zip
    };
    let archive = read_stored_profile_zip(source.bytes(), zip_limits)
        .map_err(ProfileOpenFileImportError::Zip)?;
    let root = archive
        .admitted_root_json(limits.root_json)
        .map_err(ProfileOpenFileImportError::RootJson)?;
    let decoded = decode_attached_profile_manifest_json(&root, limits.manifest)
        .map_err(ProfileOpenFileImportError::Manifest)?;
    let mut names = Vec::new();
    names
        .try_reserve_exact(archive.entries().len())
        .map_err(|_error| ProfileOpenFileImportError::AllocationFailed)?;
    names.resize(archive.entries().len(), "");
    let verified = verify_attached_profile_import(
        intent,
        &decoded,
        supported_required_features,
        limits.import,
        &mut names,
    )
    .map_err(ProfileOpenFileImportError::Verification)?;
    for (index, entry) in verified.manifest().entries.iter().enumerate() {
        if !entry.path.starts_with("sections/") {
            continue;
        }
        // First-release sections are JSON, not arbitrary opaque assets.
        // Type checks follow all payload digests, before any inspection.
        if entry.media_type != "application/json" {
            return Err(ProfileOpenFileImportError::SectionMediaType { index });
        }
        let bytes = verified.verified_entry_bytes(&entry.path).ok_or(
            ProfileOpenFileImportError::MissingVerifiedSection { index },
        )?;
        let _syntax =
            validate_profile_json_object(bytes, limits.section_json).map_err(
                |reason| ProfileOpenFileImportError::SectionJson {
                    index,
                    reason,
                },
            )?;
    }
    Ok(inspect(&verified))
}

/// Inspect a source opened only from the classified selected profile path.
///
/// This composition checks purpose before any filesystem access, opens only
/// `intent.path()` via the secure platform policy, then reads and verifies the
/// returned descriptor. Complete verified data reaches the caller only after
/// securely reopening the selected name and comparing its inode and metadata.
/// It cannot be redirected to an unrelated file-source callback. A selected
/// path or `CallerSelection` label is still **not** proof
/// of a real caller gesture: a trusted host must authenticate that provenance
/// before invoking this method. Linux requires `openat2` with no symlinks;
/// other platforms and unsupported kernels fail closed. No session mutation
/// or file Export occurs, and concurrent in-place writes are not snapshotted.
///
/// # Errors
///
/// Rejects wrong purpose before opening and preserves secure-open, bounded
/// source, ZIP, manifest, SHA and section syntax refusals with no partial
/// inspection.
pub fn inspect_selected_profile_import<R, Inspect>(
    intent: &ExplicitFilePathIntent,
    supported_required_features: &[&str],
    limits: ProfileOpenFileImportLimits,
    inspect: Inspect,
) -> Result<R, ProfileOpenFileImportError>
where
    Inspect: FnOnce(&VerifiedAttachedProfileImport<'_, '_, '_, '_>) -> R,
{
    open_selected_profile_import_probe(intent)?.inspect(
        supported_required_features,
        limits,
        inspect,
    )
}

/// Securely open an independently selected path for one later Import check.
///
/// No classified path grants Import on its own: the host must authenticate
/// the caller's selection before requesting a probe. On Linux both opens
/// reject every symlink component and parent traversal; a relative path also
/// retains its original process CWD identity. This read-only probe does not
/// admit handwriting sections or mutate session state.
///
/// # Errors
///
/// Refuses wrong purpose, unsupported platforms, unsafe or invalid paths and
/// failure to obtain the initial selected file metadata.
pub fn open_selected_profile_import_probe(
    intent: &ExplicitFilePathIntent,
) -> Result<ProfileSelectedImportProbe<'_>, ProfileOpenFileImportError> {
    if intent.purpose() != ExplicitFilePurpose::ImportProfile {
        return Err(ProfileOpenFileImportError::WrongPathPurpose);
    }
    let selected_cwd = (!intent.path().is_absolute())
        .then(|| File::open("."))
        .transpose()
        .map_err(|error| {
            ProfileOpenFileImportError::Open(
                ProfileSelectedFileOpenError::FileIo(error.kind()),
            )
        })?;
    let opened = selected_cwd
        .as_ref()
        .map_or_else(
            || open_selected_profile_file(intent.path()),
            |cwd| open_selected_profile_file_from_cwd(intent.path(), cwd),
        )
        .map_err(ProfileOpenFileImportError::Open)?;
    let original_metadata = opened.metadata().map_err(|error| {
        ProfileOpenFileImportError::Open(
            ProfileSelectedFileOpenError::FileIo(error.kind()),
        )
    })?;
    Ok(ProfileSelectedImportProbe {
        intent,
        opened,
        original_metadata,
        selected_cwd,
    })
}

fn same_selected_source_metadata(before: &Metadata, after: &Metadata) -> bool {
    if !before.is_file() || !after.is_file() || before.len() != after.len() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt as _;

        before.dev() == after.dev()
            && before.ino() == after.ino()
            && before.mtime() == after.mtime()
            && before.mtime_nsec() == after.mtime_nsec()
            && before.ctime() == after.ctime()
            && before.ctime_nsec() == after.ctime_nsec()
    }
    #[cfg(not(unix))]
    {
        before.modified().is_ok() && before.modified() == after.modified()
    }
}
