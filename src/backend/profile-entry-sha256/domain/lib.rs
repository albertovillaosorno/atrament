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
// - Must-Not:
//   - Trust archive CRC-32, read paths, parse ZIP or JSON, allocate a copy of
//     entry contents, or authorize application-level profile acceptance.
// - Allows:
//   - Inputs: A bounded in-memory byte slice and one typed declaration.
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
