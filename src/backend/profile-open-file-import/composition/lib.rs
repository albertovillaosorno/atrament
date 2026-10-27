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
//   - Import verification of a bounded host-opened file byte snapshot.
// - Must-Not:
//   - Open paths, infer file authority, decode profile sections, accept
//     session state, or write persistent output.
// - Allows:
//   - Inputs: Classified Import intent, file-source callback and limits.
//   - Outputs: Scoped complete ZIP/manifest/SHA proof or typed refusal.
//   - Side effects: Invokes a caller-provided bounded reader once on admission.
// - Split-When:
//   - Host selection/open policy and profile acceptance become implemented.
// - Merge-When:
//   - Complete profile Import owns source selection and accepted state.
// - Summary:
//   - Every declared member must verify before inspection is possible.
// - Description:
//   - Readiness is not a file-selection capability or session acceptance.
// - Usage:
//   - Host must associate the open source with its selected path itself.
// - Defaults:
//   - Wrong purpose never invokes the file-source callback.
//

//! Source-attached profile Import verification without filesystem authority.

use atrament_explicit_file_path_intent::{
    ExplicitFilePathIntent, ExplicitFilePurpose,
};
use atrament_profile_file_reader::{
    ProfileFileReadError, ProfileOpenedFileBytes,
};
use atrament_profile_import_preflight::ProfileImportBudget;
use atrament_profile_import_verification::{
    ProfileImportVerificationError, VerifiedAttachedProfileImport,
    verify_attached_profile_import,
};
use atrament_profile_json_syntax::ProfileJsonSyntaxLimits;
use atrament_profile_manifest_json_codec::{
    ProfileManifestJsonError, ProfileManifestJsonLimits,
    decode_attached_profile_manifest_json,
};
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
    /// The exact ZIP root is not a valid bounded JSON object.
    RootJson(ProfileZipManifestSyntaxError),
    /// The trusted host-opened source could not be read completely.
    Source(ProfileFileReadError),
    /// Features, inventory, declared budgets or SHA-256 did not match.
    Verification(ProfileImportVerificationError),
    /// A notebook or Export path cannot import a profile.
    WrongPathPurpose,
    /// ZIP CRC, framing, member order or resource admission failed.
    Zip(ProfileZipReadError),
}

/// Inspect all verified bytes from an independently opened profile file.
///
/// The trusted host must prove its source was opened from `intent.path()`,
/// with appropriate link, race and path policy. The callback must return
/// a `ProfileOpenedFileBytes` produced by the outbound file reader, not a
/// path string. Wrong purpose refuses before invoking that callback.
/// Source length is checked against independent transport/ZIP ceilings.
/// The strictest ZIP, Import or schema count bound is enforced before
/// the ZIP reader allocates its member index; excess returns ZIP `ResourceLimit`.
/// ZIP framing, JSON, manifest, inventory and every payload SHA-256 must all
/// pass before invoking `inspect`; verified borrows cannot escape inspection.
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
    Ok(inspect(&verified))
}
