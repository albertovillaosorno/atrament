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
//   - Process startup, browser launch and explicit profile inspection CLI.
// - Must-Not:
//   - Implement HTTP admission, browser process mechanics, session state
//     or persistent profile Export.
// - Allows:
//   - Inputs: Runtime startup results or exact local profile CLI arguments.
//   - Outputs: Secret-free startup records or verified profile inspection.
//   - Side effects: Stdout/stderr, runtime coordination and read-only Import.
// - Split-When:
//   - Inspection command requires its own process composition owner.
// - Merge-When:
//   - Atrament no longer composes separate runtime and launcher components.
// - Summary:
//   - Wires disposable runtime and explicit local profile inspection.
// - Description:
//   - Keeps cross-adapter orchestration in the process composition root.
// - Usage:
//   - Run atrament normally, or atrament profile inspect PATH.
// - Defaults:
//   - Reject unexpected command arguments before opening a browser.
//   - Never publishes the session credential for manual recovery.
//

//! Atrament process composition for session startup or profile inspection.

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
use atrament_profile_import_preflight::ProfileImportBudget;
use atrament_profile_json_syntax::ProfileJsonSyntaxLimits;
use atrament_profile_manifest_json_codec::ProfileManifestJsonLimits;
use atrament_profile_manifest_json_writer::{
    ProfileManifestJsonWriteLimits, write_canonical_profile_manifest_json,
};
use atrament_profile_open_file_import::{
    ProfileOpenFileImportLimits, inspect_selected_profile_import,
};
use atrament_profile_zip_reader::ProfileZipReadLimits;

#[cfg(test)]
use atrament_profile_entry_sha256 as _;
#[cfg(test)]
use atrament_profile_stored_zip_writer as _;
use atrament_session_application::SessionApplication;
use atrament_session_handshake::{
    HandshakeService, PRODUCT_VERSION, PROTOCOL_VERSION,
};
use atrament_session_runtime::Runtime;
use atrament_session_secret::SessionSecret;

// This is a bounded, read-only inspection policy, not a general profile
// ingestion or handwriting-section schema. No opaque asset data is emitted.
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

/// Verify an explicitly selected local profile before exposing readable JSON.
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
            let manifest = write_canonical_profile_manifest_json(
                verified.manifest(),
                &[],
                ProfileManifestJsonWriteLimits {
                    maximum_bytes: PROFILE_INSPECT_MAX_ROOT_BYTES,
                    maximum_entries: PROFILE_INSPECT_MAX_ENTRIES - 1,
                    maximum_features: 32,
                    maximum_string_bytes: 1024,
                },
            )
            .map_err(|error| io::Error::other(format!(
                "cannot encode admitted manifest: {error:?}",
            )))?;
            let mut readable = String::from("manifest.json (verified):\n");
            readable.push_str(from_utf8(&manifest)
                .map_err(io::Error::other)?);
            readable.push('\n');
            // The manifest can declare entries in any order. Inspection
            // should still be stable for the same admitted profile contents.
            let mut sections = verified.manifest().entries.iter()
                .filter(|entry| entry.path.starts_with("sections/"))
                .collect::<Vec<_>>();
            sections.sort_unstable_by(|left, right| left.path.cmp(&right.path));
            for entry in sections {
                let bytes = verified.verified_entry_bytes(&entry.path)
                    .ok_or_else(|| {
                        io::Error::other("missing verified section")
                    })?;
                readable.push_str(&entry.path);
                readable.push_str(" (syntax-checked; untyped):\n");
                readable.push_str(from_utf8(bytes)
                    .map_err(io::Error::other)?);
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

/// Parse command-line arguments before startup has any side effects.
///
/// `None` means the normal disposable browser session; `Some` holds a
/// completed, fully verified inspection to publish without starting a session.
///
/// # Errors
///
/// Rejects unknown or incomplete commands before session startup. It reads
/// at most four argument values without allocating an unbounded argv list.
pub(crate) fn inspect_cli_args(
    args: impl IntoIterator<Item = OsString>,
) -> io::Result<Option<String>> {
    let mut arguments = args.into_iter();
    let Some(command) = arguments.next() else {
        return Ok(None);
    };
    let action = arguments.next();
    let path = arguments.next();
    let additional = arguments.next();
    if command == "profile"
        && action.as_deref() == Some(OsStr::new("inspect"))
        && let Some(selected_path) = path
        && additional.is_none()
    {
        return inspect_profile(PathBuf::from(selected_path)).map(Some);
    }
    Err(io::Error::new(
        io::ErrorKind::InvalidInput,
        "usage: atrament [profile inspect PATH]",
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
