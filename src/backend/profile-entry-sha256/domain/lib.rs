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
//   - Dependency-free FIPS 180-4 SHA-256 over exact bounded entry bytes.
//   - Independent byte-length and cryptographic digest verification.
//   - Fixed-memory incremental hashing of caller-bounded byte chunks.
// - Must-Not:
//   - Trust archive CRC-32, read paths, parse ZIP or JSON, allocate a copy of
//     entry contents, or authorize application-level profile acceptance.
// - Allows:
//   - Inputs: Bounded in-memory byte slices and one typed declaration.
//   - Outputs: Exact SHA-256 evidence or length/digest mismatch.
//   - Side effects: None.
// - Split-When:
//   - Streaming file transport requires an independent bounded adapter.
// - Merge-When:
//   - Complete portable-profile byte admission subsumes this digest primitive.
// - Summary:
//   - Independently verifies entry bytes against the manifest's SHA-256 value.
// - Description:
//   - Uses fixed working memory; ZIP CRC-32 never proves digest equality.
// - Usage:
//   - Call after explicit path, ZIP framing, inventory and resource admission.
// - Defaults:
//   - A size mismatch rejects before digest computation.
//

//! SHA-256 verification of exact, bounded portable-profile member bytes.

use std::fmt;

use atrament_handwriting_profile_container::{
    ProfileEntryEvidence, ProfileEntryVerificationError, ProfileManifestEntry,
    Sha256Digest, verify_profile_entry,
};

const INITIAL: [u32; 8] = [
    0x6a09_e667,
    0xbb67_ae85,
    0x3c6e_f372,
    0xa54f_f53a,
    0x510e_527f,
    0x9b05_688c,
    0x1f83_d9ab,
    0x5be0_cd19,
];

const ROUND_CONSTANTS: [u32; 64] = [
    0x428a_2f98,
    0x7137_4491,
    0xb5c0_fbcf,
    0xe9b5_dba5,
    0x3956_c25b,
    0x59f1_11f1,
    0x923f_82a4,
    0xab1c_5ed5,
    0xd807_aa98,
    0x1283_5b01,
    0x2431_85be,
    0x550c_7dc3,
    0x72be_5d74,
    0x80de_b1fe,
    0x9bdc_06a7,
    0xc19b_f174,
    0xe49b_69c1,
    0xefbe_4786,
    0x0fc1_9dc6,
    0x240c_a1cc,
    0x2de9_2c6f,
    0x4a74_84aa,
    0x5cb0_a9dc,
    0x76f9_88da,
    0x983e_5152,
    0xa831_c66d,
    0xb003_27c8,
    0xbf59_7fc7,
    0xc6e0_0bf3,
    0xd5a7_9147,
    0x06ca_6351,
    0x1429_2967,
    0x27b7_0a85,
    0x2e1b_2138,
    0x4d2c_6dfc,
    0x5338_0d13,
    0x650a_7354,
    0x766a_0abb,
    0x81c2_c92e,
    0x9272_2c85,
    0xa2bf_e8a1,
    0xa81a_664b,
    0xc24b_8b70,
    0xc76c_51a3,
    0xd192_e819,
    0xd699_0624,
    0xf40e_3585,
    0x106a_a070,
    0x19a4_c116,
    0x1e37_6c08,
    0x2748_774c,
    0x34b0_bcb5,
    0x391c_0cb3,
    0x4ed8_aa4a,
    0x5b9c_ca4f,
    0x682e_6ff3,
    0x748f_82ee,
    0x78a5_636f,
    0x84c8_7814,
    0x8cc7_0208,
    0x90be_fffa,
    0xa450_6ceb,
    0xbef9_a3f7,
    0xc671_78f2,
];

/// Maximum SHA-256 message size in bytes with a 64-bit bit-length field.
///
/// The importing application must enforce much smaller resource budgets.
pub const MAXIMUM_SHA256_MESSAGE_BYTES: u64 = u64::MAX >> 3u32;

/// Rejected incremental SHA-256 input beyond the FIPS message-length bound.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileSha256UpdateError {
    /// The total message would exceed the maximum 64-bit bit-length field.
    MessageTooLong,
}

/// Failed verification of one typed entry across admitted byte chunks.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileChunkVerificationError {
    /// Byte count or digest differs from the supplied typed declaration.
    Entry(ProfileEntryVerificationError),
    /// The cumulative source length exceeds the SHA-256 bit-length field.
    Hasher(ProfileSha256UpdateError),
}

/// Verification failure for a source that can separately report read errors.
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum ProfileFallibleChunkVerificationError<E> {
    /// The upstream source failed, not a declared length or digest mismatch.
    Input(E),
    /// The admitted chunk data failed its separate SHA-256 or length check.
    Verification(ProfileChunkVerificationError),
}

// The upstream error type can contain private selected paths and arbitrary
// messages. Never format that payload implicitly: typed callers retain E.
impl<E> fmt::Debug for ProfileFallibleChunkVerificationError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Input(_) => f.debug_tuple("Input").finish_non_exhaustive(),
            Self::Verification(reason) => f
                .debug_tuple("Verification")
                .field(reason)
                .finish(),
        }
    }
}

/// Fixed-memory incremental SHA-256 state for already bounded input chunks.
///
/// The caller still owns byte-source admission and resource ceilings. This
/// primitive performs no I/O, does not accept paths, and never authorizes
/// a manifest, profile or section. `finish` consumes one state value, ending
/// its update sequence before returning the digest.
#[derive(Clone, Copy)]
pub struct ProfileSha256State {
    block: [u8; 64],
    buffered: usize,
    length_bytes: u64,
    state: [u32; 8],
}

// Shared verifier state prevents ordinary and fallible byte iterators from
// disagreeing about length-overrun precedence or digest acceptance.
#[derive(Clone, Copy)]
struct EntryChunkAccumulator<'manifest> {
    declaration: &'manifest ProfileManifestEntry,
    hasher: ProfileSha256State,
}

impl EntryChunkAccumulator<'_> {
    fn finish(
        self,
    ) -> Result<ProfileEntryEvidence, ProfileChunkVerificationError> {
        let observed = self.hasher.bytes_hashed();
        if observed != self.declaration.byte_length {
            return Err(ProfileChunkVerificationError::Entry(
                ProfileEntryVerificationError::ByteLengthMismatch {
                    declared: self.declaration.byte_length,
                    observed,
                },
            ));
        }
        let evidence = ProfileEntryEvidence {
            byte_length: observed,
            digest: self.hasher.finish(),
        };
        verify_profile_entry(self.declaration, evidence)
            .map_err(ProfileChunkVerificationError::Entry)?;
        Ok(evidence)
    }

    const fn new(
        declaration: &ProfileManifestEntry,
    ) -> EntryChunkAccumulator<'_> {
        EntryChunkAccumulator {
            declaration,
            hasher: ProfileSha256State::new(),
        }
    }

    fn update(
        &mut self,
        chunk: &[u8],
    ) -> Result<(), ProfileChunkVerificationError> {
        let observed = checked_sha256_message_bytes(
            self.hasher.bytes_hashed(),
            chunk.len(),
        )
        .map_err(ProfileChunkVerificationError::Hasher)?;
        if observed > self.declaration.byte_length {
            return Err(ProfileChunkVerificationError::Entry(
                ProfileEntryVerificationError::ByteLengthMismatch {
                    declared: self.declaration.byte_length,
                    observed,
                },
            ));
        }
        self.hasher
            .update(chunk)
            .map_err(ProfileChunkVerificationError::Hasher)
    }
}

// Buffered blocks contain untrusted profile bytes, including partial JSON
// or opaque assets. Internal chaining words must not escape through logging.
impl fmt::Debug for ProfileSha256State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProfileSha256State")
            .finish_non_exhaustive()
    }
}

impl Default for ProfileSha256State {
    fn default() -> Self {
        Self::new()
    }
}

impl ProfileSha256State {
    /// Number of complete and pending input bytes admitted so far.
    ///
    /// This count includes the pending partial compression block and never
    /// wraps the SHA-256 64-bit message-length field.
    #[must_use]
    pub const fn bytes_hashed(&self) -> u64 {
        self.length_bytes
    }

    /// Finish all admitted chunks and return their combined SHA-256 digest.
    ///
    /// A final partial block, including an empty final block, is padded
    /// according to FIPS 180-4 with the exact total 64-bit bit length.
    #[must_use]
    pub fn finish(mut self) -> Sha256Digest {
        if let Some(delimiter) = self.block.get_mut(self.buffered) {
            *delimiter = 0x80;
        }
        if let Some(remaining) =
            self.block.get_mut(self.buffered.saturating_add(1)..)
        {
            remaining.fill(0);
        }
        if self.buffered >= 56 {
            compress(&mut self.state, &self.block);
            self.block.fill(0);
        }
        if let Some(length) = self.block.get_mut(56..64) {
            let bit_length = self.length_bytes.wrapping_mul(8);
            length.copy_from_slice(&bit_length.to_be_bytes());
        }
        compress(&mut self.state, &self.block);
        let mut digest = [0u8; 32];
        for (word, destination) in self
            .state
            .into_iter()
            .zip(digest.as_chunks_mut::<4>().0.iter_mut())
        {
            *destination = word.to_be_bytes();
        }
        Sha256Digest::from_bytes(digest)
    }

    /// Create the SHA-256 initial state using fixed-size stack storage.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            block: [0u8; 64],
            buffered: 0,
            length_bytes: 0,
            state: INITIAL,
        }
    }

    /// Add an admitted chunk, preserving exact byte order across updates.
    ///
    /// A length overflow is rejected before mutating state. Chunks can be
    /// empty, split at any byte, or span multiple compression blocks; the
    /// decoder never keeps a heap-backed copy of the complete message.
    ///
    /// # Errors
    ///
    /// Returns `MessageTooLong` if the cumulative bit count would exceed
    /// the SHA-256 64-bit message-length field, without consuming input.
    pub fn update(
        &mut self,
        bytes: &[u8],
    ) -> Result<(), ProfileSha256UpdateError> {
        let total =
            checked_sha256_message_bytes(self.length_bytes, bytes.len())?;
        self.length_bytes = total;
        let mut remaining = bytes;
        if self.buffered != 0 {
            let available = 64usize.saturating_sub(self.buffered);
            let consumed = available.min(remaining.len());
            let end = self.buffered.saturating_add(consumed);
            if let Some(slot) = self.block.get_mut(self.buffered..end)
                && let Some(source) = remaining.get(..consumed)
            {
                slot.copy_from_slice(source);
            }
            self.buffered = end;
            remaining = remaining.get(consumed..).unwrap_or_default();
            if self.buffered != 64 {
                // No complete compression block yet: retain the partial
                // prefix across arbitrary subsequent update calls.
                return Ok(());
            }
            compress(&mut self.state, &self.block);
            self.buffered = 0;
        }
        let (full, rest) = remaining.as_chunks::<64>();
        for block in full {
            compress(&mut self.state, block);
        }
        if let Some(slot) = self.block.get_mut(..rest.len()) {
            slot.copy_from_slice(rest);
        }
        self.buffered = rest.len();
        Ok(())
    }
}

/// Check cumulative SHA-256 message length without allocating any bytes.
///
/// This proves arithmetic range only, not source, file, or Import authority.
/// The calling application must separately bound allowed entry sizes.
///
/// # Errors
///
/// Refuses a total size exceeding the SHA-256 64-bit bit-length field.
pub fn checked_sha256_message_bytes(
    already: u64,
    additional: usize,
) -> Result<u64, ProfileSha256UpdateError> {
    let additional_bytes = u64::try_from(additional)
        .map_err(|_overflow| ProfileSha256UpdateError::MessageTooLong)?;
    already
        .checked_add(additional_bytes)
        .filter(|total| *total <= MAXIMUM_SHA256_MESSAGE_BYTES)
        .ok_or(ProfileSha256UpdateError::MessageTooLong)
}

fn compress(state: &mut [u32; 8], block: &[u8; 64]) {
    // Indices are bounded by the fixed 64-word SHA-256 message schedule.
    let mut words = [0u32; 64];
    for (destination, chunk) in
        words.iter_mut().zip(block.as_chunks::<4>().0.iter())
    {
        *destination = u32::from_be_bytes(*chunk);
    }
    for index in 16usize..64usize {
        let w15 = words
            .get(index.saturating_sub(15))
            .copied()
            .unwrap_or_default();
        let w2 = words
            .get(index.saturating_sub(2))
            .copied()
            .unwrap_or_default();
        let w16 = words
            .get(index.saturating_sub(16))
            .copied()
            .unwrap_or_default();
        let w7 = words
            .get(index.saturating_sub(7))
            .copied()
            .unwrap_or_default();
        let small0 = w15.rotate_right(7) ^ w15.rotate_right(18) ^ (w15 >> 3u32);
        let small1 = w2.rotate_right(17) ^ w2.rotate_right(19) ^ (w2 >> 10u32);
        if let Some(word) = words.get_mut(index) {
            *word = w16
                .wrapping_add(small0)
                .wrapping_add(w7)
                .wrapping_add(small1);
        }
    }
    let [
        mut work0,
        mut work1,
        mut work2,
        mut work3,
        mut work4,
        mut work5,
        mut work6,
        mut work7,
    ] = *state;
    for (word, constant) in words.into_iter().zip(ROUND_CONSTANTS) {
        let sigma1 = work4.rotate_right(6)
            ^ work4.rotate_right(11)
            ^ work4.rotate_right(25);
        let choice = (work4 & work5) ^ ((!work4) & work6);
        let temp1 = work7
            .wrapping_add(sigma1)
            .wrapping_add(choice)
            .wrapping_add(constant)
            .wrapping_add(word);
        let sigma0 = work0.rotate_right(2)
            ^ work0.rotate_right(13)
            ^ work0.rotate_right(22);
        let majority = (work0 & work1) ^ (work0 & work2) ^ (work1 & work2);
        let temp2 = sigma0.wrapping_add(majority);
        work7 = work6;
        work6 = work5;
        work5 = work4;
        work4 = work3.wrapping_add(temp1);
        work3 = work2;
        work2 = work1;
        work1 = work0;
        work0 = temp1.wrapping_add(temp2);
    }
    for (slot, calculated) in state
        .iter_mut()
        .zip([work0, work1, work2, work3, work4, work5, work6, work7])
    {
        *slot = slot.wrapping_add(calculated);
    }
}

/// Compute SHA-256 of an exact byte slice without I/O or heap allocations.
///
/// The caller must already enforce archive and entry memory limits. The
/// algorithm uses a 64-byte block and a fixed 128-byte final padding buffer.
/// This does not confer trust in the byte slice's source or its declared path.
#[must_use]
pub fn sha256_profile_bytes(bytes: &[u8]) -> Sha256Digest {
    let mut state = INITIAL;
    let (blocks, remainder) = bytes.as_chunks::<64>();
    for block in blocks {
        compress(&mut state, block);
    }
    let mut tail = [0u8; 128];
    if let Some(destination) = tail.get_mut(..remainder.len()) {
        destination.copy_from_slice(remainder);
    }
    if let Some(delimiter) = tail.get_mut(remainder.len()) {
        *delimiter = 0x80;
    }
    let bit_length = u64::try_from(bytes.len())
        .unwrap_or(u64::MAX)
        .wrapping_mul(8);
    let padding_length = if remainder.len() < 56 {
        64usize
    } else {
        128usize
    };
    if let Some(length) =
        tail.get_mut(padding_length.saturating_sub(8)..padding_length)
    {
        length.copy_from_slice(&bit_length.to_be_bytes());
    }
    if let Some(padding) = tail.get(..padding_length) {
        for block in padding.as_chunks::<64>().0 {
            compress(&mut state, block);
        }
    }
    let mut digest = [0u8; 32];
    for (word, destination) in state
        .into_iter()
        .zip(digest.as_chunks_mut::<4>().0.iter_mut())
    {
        *destination = word.to_be_bytes();
    }
    Sha256Digest::from_bytes(digest)
}

/// Verify one declared entry from ordered, already admitted byte chunks.
///
/// The caller must independently enforce source and aggregate resource limits
/// and must not infer profile or path authority from this digest. Every chunk
/// is processed once without collecting the full message on the heap. A
/// byte-count overrun rejects at the earliest known prefix before its hash
/// is computed; a short stream rejects after the final chunk. Digest equality
/// is considered only for an exact declared byte length.
///
/// # Errors
///
/// Distinguishes the existing typed entry length/digest failures from a
/// SHA-256 message-length overflow. A length-overrun error's `observed` count
/// is the first already-supplied prefix larger than the declaration, not a
/// claim about unseen later chunks.
pub fn verify_profile_entry_chunks<'bytes, I>(
    declaration: &ProfileManifestEntry,
    chunks: I,
) -> Result<ProfileEntryEvidence, ProfileChunkVerificationError>
where
    I: IntoIterator<Item = &'bytes [u8]>,
{
    let mut verifier = EntryChunkAccumulator::new(declaration);
    for chunk in chunks {
        verifier.update(chunk)?;
    }
    verifier.finish()
}

/// Verify a declared entry from ordered chunks that can fail independently.
///
/// Upstream errors propagate without being mislabeled as missing bytes,
/// malformed digests, or successful end-of-stream. This function never reads
/// from files: the caller still owns source authentication, path policy,
/// I/O, aggregate budget enforcement and cleanup. It shares the exact
/// length-before-digest policy of `verify_profile_entry_chunks`.
///
/// # Errors
///
/// Returns the original input error unchanged or the typed entry verifier
/// failure; an input failure never yields partially accepted digest evidence.
pub fn verify_profile_entry_chunks_fallible<'bytes, I, E>(
    declaration: &ProfileManifestEntry,
    chunks: I,
) -> Result<ProfileEntryEvidence, ProfileFallibleChunkVerificationError<E>>
where
    I: IntoIterator<Item = Result<&'bytes [u8], E>>,
{
    let mut verifier = EntryChunkAccumulator::new(declaration);
    for chunk in chunks {
        let bytes =
            chunk.map_err(ProfileFallibleChunkVerificationError::Input)?;
        verifier
            .update(bytes)
            .map_err(ProfileFallibleChunkVerificationError::Verification)?;
    }
    verifier
        .finish()
        .map_err(ProfileFallibleChunkVerificationError::Verification)
}

/// Verify one actual member byte slice against a typed manifest declaration.
///
/// A length mismatch rejects before hashing, and a digest mismatch rejects
/// before any downstream typed-section parsing. No file or session is mutated.
///
/// # Errors
///
/// Returns the existing typed byte-length or SHA-256 digest mismatch.
pub fn verify_profile_entry_bytes(
    declaration: &ProfileManifestEntry,
    bytes: &[u8],
) -> Result<ProfileEntryEvidence, ProfileEntryVerificationError> {
    let observed_length = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    if observed_length != declaration.byte_length {
        return Err(ProfileEntryVerificationError::ByteLengthMismatch {
            declared: declaration.byte_length,
            observed: observed_length,
        });
    }
    let evidence = ProfileEntryEvidence {
        byte_length: observed_length,
        digest: sha256_profile_bytes(bytes),
    };
    verify_profile_entry(declaration, evidence)?;
    Ok(evidence)
}
