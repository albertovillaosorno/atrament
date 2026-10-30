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
//   - Real EOF probes prevent truncated views from proving full file content.
// - Description:
//   - Source bytes cannot escape on a size or I/O failure.
// - Usage:
//   - Only a trusted host may bind the open handle to its selected path.
// - Defaults:
//   - Nonregular, oversized, short and growing files fail closed.
//

//! File transport snapshot with exact length/EOF checks, no path authority.

use std::fmt;
use std::fs::File;
use std::io::{self, Read as _, Seek as _, SeekFrom};

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
    /// The file's reported size exceeds the host's specified ceiling.
    FileTooLarge,
    /// Directories, pipes and devices are not admitted profile sources.
    NotRegularFile,
}

fn read_retry(
    source: &mut File,
    destination: &mut [u8],
) -> Result<usize, io::ErrorKind> {
    loop {
        match source.read(destination) {
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error.kind()),
            Ok(count) => return Ok(count),
        }
    }
}

/// Consume a host-opened regular file with an actual exact EOF probe.
///
/// A trusted host must establish the original selection and open policy.
/// This reader never opens paths and does not prove that a simultaneous
/// writer left the disk file stable. It accepts only a complete byte read
/// matching the original file length, then returns an owned bounded slice.
/// Rewind begins at byte zero regardless of the caller's initial file cursor.
/// The caller must still check ZIP framing, typed metadata and SHA-256.
///
/// # Errors
///
/// Rejects wrong file types, OS errors, metadata-size excess, short reads
/// and unexpected bytes after the declared end, including zero-length files.
pub fn read_bounded_profile_file(
    mut source: File,
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
    let _position = source
        .seek(SeekFrom::Start(0))
        .map_err(|error| ProfileFileReadError::FileIo(error.kind()))?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(expected)
        .map_err(|_error| ProfileFileReadError::AllocationFailed)?;
    let mut buffer = [0u8; READ_BUFFER_BYTES];
    while bytes.len() < expected {
        let remaining = expected.saturating_sub(bytes.len());
        let count = read_retry(
            &mut source,
            buffer
                .get_mut(..remaining.min(READ_BUFFER_BYTES))
                .ok_or(ProfileFileReadError::AllocationFailed)?,
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
        &mut source,
        buffer
            .get_mut(..1)
            .ok_or(ProfileFileReadError::AllocationFailed)?,
    )
    .map_err(ProfileFileReadError::FileIo)?
        != 0
    {
        return Err(ProfileFileReadError::FileLengthChanged);
    }
    Ok(ProfileOpenedFileBytes { bytes })
}
