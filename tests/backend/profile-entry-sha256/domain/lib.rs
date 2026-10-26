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
//   - Independent SHA-256 known-answer and exact profile-entry checks.
// - Must-Not:
//   - Read actual files, trust ZIP CRC-32, or accept a profile into a session.
// - Allows:
//   - Inputs: Static reference digests and deterministic byte sequences.
//   - Outputs: Digest equality and typed verification failures.
//   - Side effects: In-memory test allocation only.
// - Split-When:
//   - Streaming or ZIP/JSON adapter tests need their own fixtures.
// - Merge-When:
//   - End-to-end profile import can exercise exact member bytes.
// - Summary:
//   - Checks padding boundaries and size-before-digest precedence.
// - Description:
//   - Uses FIPS SHA-256 vectors and independent hashlib digests.
// - Usage:
//   - Run the Rust workspace integration suite.
// - Defaults:
//   - Mutated bytes fail closed without any session mutation.
//

//! Independent known-answer tests for portable-profile member SHA-256.

use atrament_handwriting_profile_container::{
    ProfileEntryEvidence, ProfileEntryVerificationError, ProfileManifestEntry,
    Sha256Digest,
};
use atrament_profile_entry_sha256::{
    sha256_profile_bytes, verify_profile_entry_bytes,
};
use std::fmt::Write as _;
use std::iter::successors;

fn digest_hex(digest: Sha256Digest) -> String {
    let mut hex = String::with_capacity(64);
    for byte in digest.bytes() {
        assert!(
            write!(&mut hex, "{byte:02x}").is_ok(),
            "String formatting works"
        );
    }
    hex
}

fn declaration(bytes: &[u8]) -> ProfileManifestEntry {
    ProfileManifestEntry {
        byte_length: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
        digest: sha256_profile_bytes(bytes),
        media_type: "application/json".into(),
        path: "sections/strokes.json".into(),
    }
}

#[test]
fn standard_empty_abc_and_million_a_vectors_match() {
    // Published SHA-256 examples, including the multi-block million-'a' case.
    assert_eq!(
        digest_hex(sha256_profile_bytes(b"")),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    );
    assert_eq!(
        digest_hex(sha256_profile_bytes(b"abc")),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
    );
    assert_eq!(
        digest_hex(sha256_profile_bytes(&vec![b'a'; 1_000_000])),
        "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0",
    );
}

#[test]
fn independently_hashed_padding_and_block_boundaries_match() {
    // Python hashlib reference values for bytes (index * 31 + 7) modulo 256.
    // Exercise both padding lengths and full-block carry at adjacent lengths.
    let lengths: [usize; 13] =
        [1, 3, 55, 56, 63, 64, 65, 119, 120, 127, 128, 129, 1024];
    let digests = [
        "ca358758f6d27e6cf45272937977a748fd88391db679ceda7dc7bf1f005ee879",
        "647674a296197442f518bcca323ec605dd8d098b2d4f22ee1fdcdd2bb753a189",
        "8aa994584139d128848eeebc4e815639ba5ab6e6e39574195a63ac4f14f7c43b",
        "ad574708f75c044c9b85de64cb568ee7711ff4f36448c6242f053ba8f6cc2b63",
        "280ed3e8ff1df845b2e7dfe6ac6cee817bef20e783cc65abc41b818b4d2fe076",
        "c6ab9724ade5b6a7a1edfffb12f3aa9181351355af8fd08c919952ad211339dd",
        "788367c73c7ddf4c53f65e68cc0d943e6227ab55b0e78ba63ace822b1c6301c0",
        "3d610547d68216dedf7435a4fb6260353911f6b3fd3f18805ddb8be285d726fe",
        "1f80156a804cb7862ad113e8200e9d74499723e7c7854d5f48776d3148e09656",
        "192409cd280e14b743642ad1343fbd3e82d9305de72c078117745a679210cc3d",
        "cc548ca2dec1f6fe4f58b2e27aa9c7521607df1130d140b55a4dad0665302356",
        "81e89a7b2911aaa7795f9e3d4910cb47d6cd2b00d83b8399481527261a1a7519",
        "8d7e566766f6bd1bb4cac87cadfde681197f9243f4d2692a0fd12674092212a7",
    ];
    for (length, expected) in lengths.into_iter().zip(digests) {
        let bytes: Vec<u8> =
            successors(Some(7u8), |value| Some(value.wrapping_add(31)))
                .take(length)
                .collect();
        assert_eq!(
            digest_hex(sha256_profile_bytes(&bytes)),
            expected,
            "length={length}",
        );
    }
}

#[test]
fn exact_bytes_match_declaration_and_mutations_fail_closed() {
    let bytes = b"{\"strokes\": [1, 2, 3]}";
    let entry = declaration(bytes);
    let expected_length = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    assert_eq!(
        verify_profile_entry_bytes(&entry, bytes),
        Ok(ProfileEntryEvidence {
            byte_length: expected_length,
            digest: entry.digest,
        }),
    );

    let mut changed = bytes.to_vec();
    if let Some(first) = changed.first_mut() {
        *first ^= 1;
    }
    assert_eq!(
        verify_profile_entry_bytes(&entry, &changed),
        Err(ProfileEntryVerificationError::DigestMismatch),
    );
    assert_eq!(
        verify_profile_entry_bytes(
            &entry,
            bytes
                .get(..bytes.len().saturating_sub(1))
                .unwrap_or_default()
        ),
        Err(ProfileEntryVerificationError::ByteLengthMismatch {
            declared: expected_length,
            observed: expected_length - 1,
        }),
    );
}

#[test]
fn length_mismatch_takes_precedence_over_digest_mismatch() {
    let mut entry = declaration(b"abc");
    entry.byte_length += 1;
    entry.digest = sha256_profile_bytes(b"");
    assert_eq!(
        verify_profile_entry_bytes(&entry, b"abc"),
        Err(ProfileEntryVerificationError::ByteLengthMismatch {
            declared: 4,
            observed: 3,
        }),
    );

    entry.byte_length = 3;
    assert_eq!(
        verify_profile_entry_bytes(&entry, b"abc"),
        Err(ProfileEntryVerificationError::DigestMismatch),
    );
}
