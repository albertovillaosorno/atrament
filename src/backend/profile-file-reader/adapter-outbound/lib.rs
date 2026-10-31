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
//   - Bounded read-only bytes from an already opened regular file.
// - Must-Not:
//   - Choose paths, infer host selection, decode ZIP, or write data.
// - Allows:
//   - Inputs: An owned host-opened File and a caller-selected byte ceiling.
//   - Outputs: Exact owned file bytes or a typed source failure.
//   - Side effects: Reads and closes one file handle; bounded allocation.
// - Split-When:
//   - Trusted host file-opening and selection policy need separate adapters.
// - Merge-When:
//   - Full file Import ownership includes host paths and section decoding.
// - Summary:
//   - Exact EOF and descriptor metadata rechecks reject observed file drift.
// - Description:
//   - Source bytes cannot escape on size, I/O or metadata drift failure.
// - Usage:
//   - Only a trusted host may bind the open handle to its selected path.
// - Defaults:
//   - Nonregular, oversized, short, growing or observed rewritten files fail.
//

//! File transport snapshot with exact length/EOF checks, no path authority.

use std::fmt;
use std::fs::{File, Metadata};
use std::io;
#[cfg(not(unix))]
use std::io::{Read as _, Seek as _, SeekFrom};
#[cfg(unix)]
use std::os::unix::fs::FileExt as _;

const READ_BUFFER_BYTES: usize = 8 * 1024;

/// One exact-length owned snapshot, not a stable disk snapshot or trusted path.
/// Debug output excludes the complete private source bytes.
pub struct ProfileOpenedFileBytes {
    bytes: Vec<u8>,
}

impl fmt::Debug for ProfileOpenedFileBytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProfileOpenedFileBytes")
            .finish_non_exhaustive()
    }
}

impl ProfileOpenedFileBytes {
    /// Borrow exactly the bytes read from the owned file handle.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Refusal before any source bytes become accessible to a decoder.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileFileReadError {
    /// An initial exact-size reservation could not be completed.
    AllocationFailed,
    /// The source metadata, seek or read failed.
    FileIo(io::ErrorKind),
    /// Source length differed from metadata at EOF or during reading.
    FileLengthChanged,
    /// Source metadata changed across the bounded read, even at equal length.
    FileMetadataChanged,
    /// The file's reported size exceeds the host's specified ceiling.
    FileTooLarge,
    /// Directories, pipes and devices are not admitted profile sources.
    NotRegularFile,
}

// Positional reads avoid changing a duplicated descriptor's shared offset.
// Another holder's seek cannot redirect this reader into the wrong chunks.
#[cfg(unix)]
fn read_retry(
    source: &File,
    destination: &mut [u8],
    offset: u64,
) -> Result<usize, io::ErrorKind> {
    loop {
        match source.read_at(destination, offset) {
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error.kind()),
            Ok(count) => return Ok(count),
        }
    }
}

// Preserve portable seek/read behavior until other platforms have a secure
// position-independent file read primitive with equivalent validation.
#[cfg(not(unix))]
fn read_retry(
    source: &File,
    destination: &mut [u8],
    _offset: u64,
) -> Result<usize, io::ErrorKind> {
    let mut reader = source;
    loop {
        match reader.read(destination) {
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error.kind()),
            Ok(count) => return Ok(count),
        }
    }
}

// Metadata is an additional observed-mutation gate, not a filesystem lock.
// The already-open descriptor remains bound to the same inode. On Unix,
// checking ctime as well as mtime rejects an observed rewrite even if another
// writer restores the previous mtime. Other platforms require a readable
// modification timestamp and exact size; they do not gain Unix ctime proof.
#[must_use]
pub(crate) fn source_metadata_unchanged(
    before: &Metadata,
    after: &Metadata,
) -> bool {
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

/// Consume a host-opened regular file with an actual exact EOF probe.
///
/// A trusted host must establish the original selection and open policy.
/// This reader never opens paths and does not prove that a simultaneous
/// writer left the disk file stable. It accepts only a complete byte read
/// matching the original file length and rechecks descriptor metadata after
/// the exact EOF probe. Observed even-length rewrites refuse, but unchanged
/// metadata cannot prove a snapshot against all simultaneous writers.
/// Reading begins at byte zero regardless of the caller's file cursor. On
/// Unix, positional reads also leave duplicated descriptors' shared offset
/// untouched; another holder's seek cannot redirect the source read.
/// The caller must still check ZIP framing, typed metadata and SHA-256.
///
/// # Errors
///
/// Rejects wrong file types, OS errors, metadata-size excess, short reads,
/// unexpected bytes after the declared end (including zero-length files),
/// and changes to the opened file's metadata across the read.
pub fn read_bounded_profile_file(
    source: File,
    maximum_bytes: usize,
) -> Result<ProfileOpenedFileBytes, ProfileFileReadError> {
    let metadata = source
        .metadata()
        .map_err(|error| ProfileFileReadError::FileIo(error.kind()))?;
    if !metadata.is_file() {
        return Err(ProfileFileReadError::NotRegularFile);
    }
    let expected = usize::try_from(metadata.len())
        .map_err(|_error| ProfileFileReadError::FileTooLarge)?;
    if expected > maximum_bytes {
        return Err(ProfileFileReadError::FileTooLarge);
    }
    #[cfg(not(unix))]
    let _position = (&source)
        .seek(SeekFrom::Start(0))
        .map_err(|error| ProfileFileReadError::FileIo(error.kind()))?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(expected)
        .map_err(|_error| ProfileFileReadError::AllocationFailed)?;
    let mut buffer = [0u8; READ_BUFFER_BYTES];
    while bytes.len() < expected {
        let remaining = expected.saturating_sub(bytes.len());
        let offset = u64::try_from(bytes.len())
            .map_err(|_error| ProfileFileReadError::FileTooLarge)?;
        let count = read_retry(
            &source,
            buffer
                .get_mut(..remaining.min(READ_BUFFER_BYTES))
                .ok_or(ProfileFileReadError::AllocationFailed)?,
            offset,
        )
        .map_err(ProfileFileReadError::FileIo)?;
        if count == 0 {
            return Err(ProfileFileReadError::FileLengthChanged);
        }
        bytes.extend_from_slice(
            buffer
                .get(..count)
                .ok_or(ProfileFileReadError::AllocationFailed)?,
        );
    }
    // A truncated view can manufacture EOF; probe the underlying handle.
    if read_retry(
        &source,
        buffer
            .get_mut(..1)
            .ok_or(ProfileFileReadError::AllocationFailed)?,
        metadata.len(),
    )
    .map_err(ProfileFileReadError::FileIo)?
        != 0
    {
        return Err(ProfileFileReadError::FileLengthChanged);
    }
    // An in-place rewrite can preserve both the original length and EOF.
    // Refuse observed descriptor drift before any owned bytes are returned.
    let after = source
        .metadata()
        .map_err(|error| ProfileFileReadError::FileIo(error.kind()))?;
    if !source_metadata_unchanged(&metadata, &after) {
        return Err(ProfileFileReadError::FileMetadataChanged);
    }
    // Release the owned descriptor before exposing the verified byte buffer.
    drop(source);
    Ok(ProfileOpenedFileBytes { bytes })
}
