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
//   - Exact SHA-256 and byte-length verification from one preopened reader
//     under an independent per-entry input-byte budget.
// - Must-Not:
//   - Open paths, establish file authority, parse ZIP/JSON, allocate full
//     input payloads, decode sections, or accept a handwriting profile.
// - Allows:
//   - Inputs: A caller-controlled Read source, an admitted typed entry and
//     an independently selected maximum entry-byte bound.
//   - Outputs: Typed verified byte/digest evidence or exact source failure.
//   - Side effects: Consumes only the supplied reader; no owned file effects.
// - Split-When:
//   - Runtime path authority and cancellation require a host-specific adapter.
// - Merge-When:
//   - A complete, admitted explicit Import transport replaces this boundary.
// - Summary:
//   - Hashes bounded read chunks without promoting byte text into Import.
// - Description:
//   - Probes EOF to reject trailing bytes after an exact declared length.
// - Usage:
//   - Use only after separate source selection and manifest admission.
// - Defaults:
//   - Read failures, incomplete streams, extra bytes and hashes fail closed.
//

//! Bounded SHA-256 verification of bytes from an already opened reader.

use std::io::{self, Read};

use atrament_handwriting_profile_container::{
    ProfileEntryEvidence, ProfileEntryVerificationError, ProfileManifestEntry,
    verify_profile_entry,
};
use atrament_profile_entry_sha256::{
    ProfileChunkVerificationError, ProfileSha256State,
    checked_sha256_message_bytes,
};

/// Fixed, stack-backed read buffer, independent of untrusted byte lengths.
pub const PROFILE_STREAM_READ_BUFFER_BYTES: usize = 8192;

/// Source or integrity failure while verifying one preopened reader.
#[derive(Debug)]
pub enum ProfileEntryStreamReadError {
    /// The typed declaration exceeds a separately chosen byte budget.
    DeclaredLimitExceeded {
        /// Exact declared entry byte count.
        declared: u64,
        /// Maximum admitted entry byte count.
        limit: u64,
    },
    /// A read operation failed, including after an otherwise complete prefix.
    Input(io::Error),
    /// A complete or partial stream failed the typed length/digest check.
    Verification(ProfileChunkVerificationError),
}

/// Verify one already opened, individually bounded entry byte stream.
///
/// The caller must independently establish the reader's provenance, its path
/// and intent authority, and the typed manifest's connection to ZIP root
/// bytes. The function reads in fixed stack storage, never buffers a complete
/// entry, and retains no bytes after returning. It checks declared and SHA
/// length bounds before the first read, then consumes at most the declared
/// count plus one EOF-probe byte. Even after receiving the declared length,
/// the EOF probe must succeed and return zero before any digest is accepted.
///
/// A partial or early EOF yields the observed length, not a guessed digest.
/// A non-EOF `io::Error` is preserved instead of being treated as EOF.
/// EOF refers **only to the supplied `Read` view**: wrappers such as
/// `Read::take` can manufacture an EOF while an underlying source still has
/// more bytes. The caller must independently prove that its reader exposes
/// the complete intended source before interpreting verified evidence.
/// Host deadlines, blocking, cancellation and stream closure remain the
/// responsibility of the caller and are not enforced by this function.
///
/// # Errors
///
/// Rejects oversized declarations without consuming the reader; returns
/// precise upstream I/O errors or a typed length/digest/hash-range failure.
pub fn verify_profile_entry_from_reader<R>(
    reader: &mut R,
    declaration: &ProfileManifestEntry,
    maximum_entry_bytes: u64,
) -> Result<ProfileEntryEvidence, ProfileEntryStreamReadError>
where
    R: Read,
{
    if declaration.byte_length > maximum_entry_bytes {
        return Err(ProfileEntryStreamReadError::DeclaredLimitExceeded {
            declared: declaration.byte_length,
            limit: maximum_entry_bytes,
        });
    }
    let _maximum_checked =
        checked_sha256_message_bytes(declaration.byte_length, 0).map_err(
            |failure| {
                ProfileEntryStreamReadError::Verification(
                    ProfileChunkVerificationError::Hasher(failure),
                )
            },
        )?;
    let mut hasher = ProfileSha256State::new();
    let mut buffer = [0u8; PROFILE_STREAM_READ_BUFFER_BYTES];
    while hasher.bytes_hashed() < declaration.byte_length {
        let remaining = declaration
            .byte_length
            .saturating_sub(hasher.bytes_hashed());
        let requested = usize::try_from(remaining)
            .unwrap_or(PROFILE_STREAM_READ_BUFFER_BYTES)
            .min(PROFILE_STREAM_READ_BUFFER_BYTES);
        let slot = buffer.get_mut(..requested).unwrap_or_default();
        let received = reader
            .read(slot)
            .map_err(ProfileEntryStreamReadError::Input)?;
        if received == 0 {
            break;
        }
        let bytes = slot.get(..received).ok_or_else(|| {
            ProfileEntryStreamReadError::Input(io::Error::new(
                io::ErrorKind::InvalidData,
                "reader returned more bytes than requested",
            ))
        })?;
        hasher.update(bytes).map_err(|failure| {
            ProfileEntryStreamReadError::Verification(
                ProfileChunkVerificationError::Hasher(failure),
            )
        })?;
    }
    let observed = hasher.bytes_hashed();
    if observed < declaration.byte_length {
        return Err(ProfileEntryStreamReadError::Verification(
            ProfileChunkVerificationError::Entry(
                ProfileEntryVerificationError::ByteLengthMismatch {
                    declared: declaration.byte_length,
                    observed,
                },
            ),
        ));
    }
    // A full prefix is not EOF. Reading one more byte is required to prove
    // the exact-length claim; errors at this point must still be visible.
    let mut probe = [0u8; 1];
    if reader
        .read(&mut probe)
        .map_err(ProfileEntryStreamReadError::Input)?
        != 0
    {
        return Err(ProfileEntryStreamReadError::Verification(
            ProfileChunkVerificationError::Entry(
                ProfileEntryVerificationError::ByteLengthMismatch {
                    declared: declaration.byte_length,
                    observed: observed.saturating_add(1),
                },
            ),
        ));
    }
    let evidence = ProfileEntryEvidence {
        byte_length: observed,
        digest: hasher.finish(),
    };
    verify_profile_entry(declaration, evidence).map_err(|failure| {
        ProfileEntryStreamReadError::Verification(
            ProfileChunkVerificationError::Entry(failure),
        )
    })?;
    Ok(evidence)
}
