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
//   - Process startup, browser launch and deliberate local profile CLI effects.
// - Must-Not:
//   - Implement HTTP admission, browser process mechanics, session state,
//     typed handwriting Import or revision-bound session Export.
// - Allows:
//   - Inputs: Runtime startup results or exact local profile CLI arguments.
//   - Outputs: Secret-free startup, verification, inspection, or
//     repack completion.
//   - Side effects: Stdout/stderr, runtime coordination, verified Import and
//     caller-selected, explicit-overwrite canonical profile repack only.
// - Split-When:
//   - Profile CLI grows independent typed-section or session policy.
// - Merge-When:
//   - Atrament no longer composes separate runtime and launcher components.
// - Summary:
//   - Wires disposable runtime and explicit verified profile CLI operations.
// - Description:
//   - Keeps cross-adapter orchestration in the process composition root.
// - Usage:
//   - Run atrament, profile verify/inspect PATH or repack [--replace] SRC DEST.
// - Defaults:
//   - Reject unexpected command arguments before opening a browser.
//   - Never publishes the session credential for manual recovery.
//

//! Atrament startup and caller-selected, independent profile CLI operations.

use std::env;
use std::ffi::{OsStr, OsString};
use std::io::{self, Write as _};
use std::path::PathBuf;
use std::str::from_utf8;

use atrament_browser_launch as browser_launch;
use atrament_explicit_file_path_intent::{
    ExplicitFilePathSource, ExplicitFilePurpose, ExplicitOverwriteIntent,
    classify_explicit_file_path_intent,
};
use atrament_profile_export_assembly::{
    ProfileExportArtifact, ProfileExportAssemblyLimits,
    ProfileExportPayload, assemble_profile_export_artifact,
};
use atrament_profile_file_commit::{
    ProfileFileCommitDisposition, commit_profile_file,
};
use atrament_profile_import_preflight::ProfileImportBudget;
use atrament_profile_json_syntax::ProfileJsonSyntaxLimits;
use atrament_profile_manifest_json_codec::ProfileManifestJsonLimits;
use atrament_profile_manifest_json_writer::{
    ProfileManifestJsonWriteLimits, write_canonical_profile_manifest_json,
};
use atrament_profile_open_file_import::{
    ProfileOpenFileImportLimits, inspect_selected_profile_import,
};
use atrament_profile_stored_zip_writer::ProfileStoredZipWriteLimits;
use atrament_profile_zip_reader::ProfileZipReadLimits;

#[cfg(test)]
use atrament_profile_entry_sha256 as _;
use atrament_session_application::SessionApplication;
use atrament_session_handshake::{
    HandshakeService, PRODUCT_VERSION, PROTOCOL_VERSION,
};
use atrament_session_runtime::Runtime;
use atrament_session_secret::SessionSecret;

// This is bounded source-verification policy, not a general profile ingestion
// or handwriting-section schema. Inspection never emits opaque asset bytes.
const PROFILE_INSPECT_MAX_ARCHIVE_BYTES: usize = 4 * 1024 * 1024;
const PROFILE_INSPECT_MAX_ARCHIVE_BYTES_U64: u64 = 4 * 1024 * 1024;
const PROFILE_INSPECT_MAX_ENTRIES: usize = 64;
const PROFILE_INSPECT_MAX_MEMBER_BYTES: usize = 2 * 1024 * 1024;
const PROFILE_INSPECT_MAX_MEMBER_BYTES_U64: u64 = 2 * 1024 * 1024;
const PROFILE_INSPECT_MAX_ROOT_BYTES: usize = 128 * 1024;

fn publish_startup(state: &str, origin: Option<&str>) -> io::Result<()> {
    let origin_json = origin
        .map_or_else(|| String::from("null"), |value| format!("\"{value}\""));
    let stdout = io::stdout();
    let mut output = stdout.lock();
    writeln!(
        output,
        concat!(
            "{{\"product\":\"atrament\",",
            "\"process_version\":\"{}\",",
            "\"protocol_version\":\"{}\",",
            "\"origin\":{},\"state\":\"{}\"}}",
        ),
        PRODUCT_VERSION, PROTOCOL_VERSION, origin_json, state,
    )?;
    output.flush()
}

pub(crate) fn browser_launch_failure(
    error: &browser_launch::LaunchError,
) -> io::Error {
    io::Error::other(format!(
        concat!(
            "Atrament browser launch failed: {}. ",
            "Fix automatic browser launch and restart Atrament; ",
            "the session credential is intentionally not published for ",
            "manual recovery.",
        ),
        error,
    ))
}

fn bind_runtime() -> io::Result<Runtime> {
    Runtime::bind().map_err(|error| {
        io::Error::new(
            error.kind(),
            format!("Atrament loopback startup failed: {error}"),
        )
    })
}

fn launch_url(origin: &str, secret: &SessionSecret) -> String {
    format!("{origin}#session={}", secret.encoded())
}

fn new_session_secret() -> io::Result<SessionSecret> {
    SessionSecret::generate().map_err(|error| {
        io::Error::other(format!(
            "Atrament session credential generation failed: {error}",
        ))
    })
}

const fn profile_inspection_limits() -> ProfileOpenFileImportLimits {
    ProfileOpenFileImportLimits {
        maximum_file_bytes: PROFILE_INSPECT_MAX_ARCHIVE_BYTES,
        import: ProfileImportBudget {
            maximum_declared_bytes: PROFILE_INSPECT_MAX_ARCHIVE_BYTES_U64,
            maximum_entries: PROFILE_INSPECT_MAX_ENTRIES - 1,
            maximum_entry_bytes: PROFILE_INSPECT_MAX_MEMBER_BYTES_U64,
        },
        manifest: ProfileManifestJsonLimits {
            maximum_entries: PROFILE_INSPECT_MAX_ENTRIES - 1,
            maximum_features: 32,
            maximum_string_bytes: 1024,
        },
        root_json: ProfileJsonSyntaxLimits {
            maximum_bytes: PROFILE_INSPECT_MAX_ROOT_BYTES,
            maximum_nesting: 32,
            maximum_object_members: 64,
            maximum_values: 4096,
        },
        section_json: ProfileJsonSyntaxLimits {
            maximum_bytes: PROFILE_INSPECT_MAX_MEMBER_BYTES,
            maximum_nesting: 32,
            maximum_object_members: 64,
            maximum_values: 4096,
        },
        zip: ProfileZipReadLimits {
            maximum_archive_bytes: PROFILE_INSPECT_MAX_ARCHIVE_BYTES,
            maximum_entries: PROFILE_INSPECT_MAX_ENTRIES,
            maximum_entry_bytes: PROFILE_INSPECT_MAX_MEMBER_BYTES_U64,
            maximum_total_bytes: PROFILE_INSPECT_MAX_ARCHIVE_BYTES_U64,
        },
    }
}

const fn profile_manifest_write_limits() -> ProfileManifestJsonWriteLimits {
    ProfileManifestJsonWriteLimits {
        maximum_bytes: PROFILE_INSPECT_MAX_ROOT_BYTES,
        maximum_entries: PROFILE_INSPECT_MAX_ENTRIES - 1,
        maximum_features: 32,
        maximum_string_bytes: 1024,
    }
}

const fn profile_repack_limits() -> ProfileExportAssemblyLimits {
    ProfileExportAssemblyLimits {
        manifest_json: profile_manifest_write_limits(),
        section_json: profile_inspection_limits().section_json,
        zip: ProfileStoredZipWriteLimits {
            maximum_archive_bytes: PROFILE_INSPECT_MAX_ARCHIVE_BYTES,
            maximum_entries: PROFILE_INSPECT_MAX_ENTRIES,
            maximum_entry_bytes: PROFILE_INSPECT_MAX_MEMBER_BYTES_U64,
            maximum_total_bytes: PROFILE_INSPECT_MAX_ARCHIVE_BYTES_U64,
        },
    }
}

/// Repack a fully verified, explicitly selected profile at a new exact path.
///
/// The two local CLI arguments are caller selections, not paths extracted
/// from profile metadata or model, notebook, clipboard or diagnostic text.
/// Every declared ZIP member, digest and JSON section must pass Import before
/// any output commit. Unknown optional features and opaque asset bytes are
/// preserved, but no handwriting sections are typed or accepted into session
/// state. The output is a new canonical archive, not a profile modification
/// or revision-bound session Export. Targets are preserved unless the caller
/// explicitly supplies `--replace` as a separate CLI overwrite instruction.
///
/// # Errors
///
/// Refuses source selection, full archive verification, canonical framing or
/// secure target creation without printing partial output or private paths.
pub(crate) fn repack_profile(
    source_path: PathBuf,
    destination_path: PathBuf,
    overwrite: ExplicitOverwriteIntent,
) -> io::Result<()> {
    let source = classify_explicit_file_path_intent(
        ExplicitFilePurpose::ImportProfile,
        ExplicitFilePathSource::CallerSelection,
        source_path,
        ExplicitOverwriteIntent::PreserveExisting,
    ).map_err(|error| io::Error::new(
        io::ErrorKind::InvalidInput,
        format!("invalid profile source: {error:?}"),
    ))?;
    let destination = classify_explicit_file_path_intent(
        ExplicitFilePurpose::ExportProfile,
        ExplicitFilePathSource::CallerSelection,
        destination_path,
        overwrite,
    ).map_err(|error| io::Error::new(
        io::ErrorKind::InvalidInput,
        format!("invalid profile destination: {error:?}"),
    ))?;

    // Fully verify the selected source before any writer can publish a file.
    // The source-backed manifest and payload borrows cannot escape this gate.
    let artifact = inspect_selected_profile_import(
        &source,
        &[],
        profile_inspection_limits(),
        |verified| -> io::Result<ProfileExportArtifact> {
            let mut payloads = Vec::new();
            payloads.try_reserve_exact(verified.manifest().entries.len())
                .map_err(|_error| io::Error::other(
                    "profile payload inventory allocation refused",
                ))?;
            for entry in &verified.manifest().entries {
                let bytes = verified.verified_entry_bytes(&entry.path)
                    .ok_or_else(|| io::Error::other(
                        "missing SHA-verified profile member",
                    ))?;
                payloads.push(ProfileExportPayload {
                    path: &entry.path,
                    bytes,
                });
            }
            assemble_profile_export_artifact(
                verified.manifest(), &payloads, &[],
                profile_repack_limits(),
            ).map_err(|error| io::Error::other(format!(
                "profile canonicalization refused: {error:?}",
            )))
        },
    ).map_err(|error| io::Error::new(
        io::ErrorKind::InvalidData,
        format!("profile repack source refused: {error:?}"),
    ))??;

    let _commit = commit_profile_file(
        destination.path(),
        artifact.bytes(),
        PROFILE_INSPECT_MAX_ARCHIVE_BYTES,
        match destination.overwrite() {
            ExplicitOverwriteIntent::PreserveExisting => {
                ProfileFileCommitDisposition::PreserveExisting
            }
            ExplicitOverwriteIntent::ReplaceExisting => {
                ProfileFileCommitDisposition::ReplaceExisting
            }
        },
    ).map_err(|error| io::Error::other(format!(
        "profile repack target refused: {error:?}",
    )))?;
    Ok(())
}

/// Verify an explicitly selected profile without exposing any profile text.
///
/// The deliberate CLI pathname is the only selected source. Full bounded
/// ZIP, manifest, inventory, SHA-256 and section JSON admission must succeed
/// before the caller can report success. Unlike inspection, this command
/// does not publish manifest identities, section paths or content. It does
/// not import handwriting into a session or grant profile Export authority.
///
/// # Errors
///
/// Refuses malformed or unsafe selected input without publishing bytes.
pub(crate) fn verify_profile(path: PathBuf) -> io::Result<()> {
    let intent = classify_explicit_file_path_intent(
        ExplicitFilePurpose::ImportProfile,
        ExplicitFilePathSource::CallerSelection,
        path,
        ExplicitOverwriteIntent::PreserveExisting,
    )
    .map_err(|error| io::Error::new(
        io::ErrorKind::InvalidInput,
        format!("invalid profile selection: {error:?}"),
    ))?;
    inspect_selected_profile_import(
        &intent,
        &[],
        profile_inspection_limits(),
        |_verified| (),
    )
    .map_err(|error| io::Error::new(
        io::ErrorKind::InvalidData,
        format!("profile verification refused: {error:?}"),
    ))
}

/// Inspect an explicitly selected local profile before exposing readable JSON.
///
/// The local command arguments are the caller's selection. Archive paths are
/// never treated as filesystem paths. Only complete ZIP, manifest, inventory
/// and SHA admission permits section inspection; sections receive additional
/// syntax validation, **not** a typed handwriting schema or session acceptance.
///
/// # Errors
///
/// Rejects invalid path selection, source opening, archive, manifest and
/// section JSON without returning partial printable data.
pub(crate) fn inspect_profile(path: PathBuf) -> io::Result<String> {
    let intent = classify_explicit_file_path_intent(
        ExplicitFilePurpose::ImportProfile,
        ExplicitFilePathSource::CallerSelection,
        path,
        ExplicitOverwriteIntent::PreserveExisting,
    )
    .map_err(|error| io::Error::new(
        io::ErrorKind::InvalidInput,
        format!("invalid profile selection: {error:?}"),
    ))?;
    inspect_selected_profile_import(
        &intent,
        &[],
        profile_inspection_limits(),
        |verified| -> io::Result<String> {
            const MANIFEST_LABEL: &str = "manifest.json (verified):\n";
            const SECTION_LABEL: &str = " (syntax-checked; untyped):\n";

            let manifest = write_canonical_profile_manifest_json(
                verified.manifest(),
                &[],
                profile_manifest_write_limits(),
            )
            .map_err(|error| io::Error::other(format!(
                "cannot encode admitted manifest: {error:?}",
            )))?;
            let manifest_text = from_utf8(&manifest).map_err(io::Error::other)?;
            // Reserve the bounded section index fallibly. No unchecked Vec
            // growth is allowed from an untrusted declared inventory.
            let mut sections = Vec::new();
            sections.try_reserve_exact(verified.manifest().entries.len())
                .map_err(|_error| io::Error::other(
                    "profile inspection section allocation refused",
                ))?;
            for entry in &verified.manifest().entries {
                if !entry.path.starts_with("sections/") {
                    continue;
                }
                let bytes = verified.verified_entry_bytes(&entry.path)
                    .ok_or_else(|| {
                        io::Error::other("missing verified section")
                    })?;
                sections.push((
                    entry.path.as_str(),
                    from_utf8(bytes).map_err(io::Error::other)?,
                ));
            }
            sections.sort_unstable_by(|left, right| left.0.cmp(right.0));

            // Exact planned output length includes section path labels and
            // newlines. Fallible one-time allocation precedes all writes;
            // every subsequent append fits the already reserved capacity.
            let length_error = || {
                io::Error::other("profile inspection output length refused")
            };
            let mut output_len = MANIFEST_LABEL.len()
                .checked_add(manifest_text.len())
                .and_then(|length| length.checked_add(1))
                .ok_or_else(length_error)?;
            for (section_path, value) in &sections {
                output_len = output_len.checked_add(section_path.len())
                    .and_then(|length| length.checked_add(SECTION_LABEL.len()))
                    .and_then(|length| length.checked_add(value.len()))
                    .and_then(|length| length.checked_add(1))
                    .ok_or_else(length_error)?;
            }
            let mut readable = String::new();
            readable.try_reserve_exact(output_len).map_err(|_error| {
                io::Error::other("profile inspection output allocation refused")
            })?;
            readable.push_str(MANIFEST_LABEL);
            readable.push_str(manifest_text);
            readable.push('\n');
            for (section_path, value) in sections {
                readable.push_str(section_path);
                readable.push_str(SECTION_LABEL);
                readable.push_str(value);
                readable.push('\n');
            }
            Ok(readable)
        },
    )
    .map_err(|error| io::Error::new(
        io::ErrorKind::InvalidData,
        format!("profile inspection refused: {error:?}"),
    ))?
}

/// Dispatch bounded CLI arguments before starting the disposable session.
///
/// `None` means normal browser startup; `Some` holds printable output from
/// inspected profile, completed verification or independent repack.
/// Repack arguments are exact OS-level local caller selections; no profile
/// archive path can determine the selected filesystem target.
///
/// # Errors
///
/// Rejects incomplete or unknown arguments before opening a browser. No more
/// than five arguments are read for a default command, or six with explicit
/// `repack --replace`; failed verification and repack publish no success.
pub(crate) fn inspect_cli_args(
    args: impl IntoIterator<Item = OsString>,
) -> io::Result<Option<String>> {
    let mut arguments = args.into_iter();
    let command = arguments.next();
    let action = arguments.next();
    let source = arguments.next();
    let destination = arguments.next();
    if command.as_deref() == Some(OsStr::new("profile")) {
        if let Some(path) = source.as_deref()
            && destination.is_none()
            && !path.as_encoded_bytes().starts_with(b"-")
        {
            if action.as_deref() == Some(OsStr::new("inspect")) {
                return inspect_profile(PathBuf::from(path)).map(Some);
            }
            if action.as_deref() == Some(OsStr::new("verify")) {
                verify_profile(PathBuf::from(path))?;
                return Ok(Some(String::from(
                    "profile verified (no session import)\n",
                )));
            }
        }
        if action.as_deref() == Some(OsStr::new("repack")) {
            let replace = source.as_deref() == Some(OsStr::new("--replace"));
            let extra_target = if replace { arguments.next() } else { None };
            let (candidate_source, candidate_target) = if replace {
                (destination.as_deref(), extra_target.as_deref())
            } else {
                (source.as_deref(), destination.as_deref())
            };
            // A bare dash-prefixed token is a CLI option, not a selected
            // path. Explicitly prefix `./` to select an option-named file.
            if let (Some(repack_source), Some(repack_destination)) =
                (candidate_source, candidate_target)
                && !repack_source.as_encoded_bytes().starts_with(b"-")
                && !repack_destination.as_encoded_bytes().starts_with(b"-")
                && arguments.next().is_none()
            {
                repack_profile(
                    PathBuf::from(repack_source),
                    PathBuf::from(repack_destination),
                    if replace {
                        ExplicitOverwriteIntent::ReplaceExisting
                    } else {
                        ExplicitOverwriteIntent::PreserveExisting
                    },
                )?;
                return Ok(Some(String::from(
                    "profile repacked (verified; no session import)\n",
                )));
            }
        }
    }
    if command.is_none() {
        return Ok(None);
    }
    Err(io::Error::new(
        io::ErrorKind::InvalidInput,
        concat!(
            "usage: atrament [profile verify PATH | profile inspect PATH | ",
            "profile repack [--replace] SOURCE TARGET]",
        ),
    ))
}

fn main() -> io::Result<()> {
    if let Some(inspected) = inspect_cli_args(env::args_os().skip(1))? {
        let stdout = io::stdout();
        let mut output = stdout.lock();
        output.write_all(inspected.as_bytes())?;
        return output.flush();
    }
    publish_startup("starting", None)?;
    let mut application = SessionApplication::default();
    let handshake = HandshakeService;
    let secret = new_session_secret()?;
    let runtime = bind_runtime()?;
    publish_startup("listening", Some(runtime.origin()))?;
    let initial_browser_url = launch_url(runtime.origin(), &secret);
    browser_launch::launch(&initial_browser_url)
        .map_err(|error| browser_launch_failure(&error))?;
    publish_startup("ready", Some(runtime.origin()))?;
    runtime.serve(secret.encoded(), &handshake, &mut application);
    Ok(())
}
