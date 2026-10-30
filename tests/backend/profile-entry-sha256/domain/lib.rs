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
    MAXIMUM_SHA256_MESSAGE_BYTES, ProfileChunkVerificationError,
    ProfileFallibleChunkVerificationError, ProfileSha256State,
    checked_sha256_message_bytes, sha256_profile_bytes,
    verify_profile_entry_bytes, verify_profile_entry_chunks,
    verify_profile_entry_chunks_fallible,
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

#[test]
fn incremental_sha256_matches_known_vectors_and_exact_padding_boundaries() {
    let patterns = [
        b"".as_slice(),
        b"abc".as_slice(),
        b"The quick brown fox jumps over the lazy dog".as_slice(),
    ];
    for pattern in patterns {
        let mut state = ProfileSha256State::new();
        assert_eq!(state.update(pattern), Ok(()));
        assert_eq!(state.finish(), sha256_profile_bytes(pattern));
    }
    let mut million = ProfileSha256State::default();
    let chunk = [b'a'; 1000];
    for _ in 0..1000 {
        assert_eq!(million.update(&chunk), Ok(()));
    }
    assert_eq!(
        digest_hex(million.finish()),
        "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
    );

    // Exhaust the full final-padding and first-block transitions around
    // 55/56 and 63/64. Different input splittings must be indistinguishable.
    for length in 0usize..=256usize {
        let bytes = (0..length)
            .map(|i| (i as u8).wrapping_mul(31))
            .collect::<Vec<_>>();
        let expected = sha256_profile_bytes(&bytes);
        for split in 0..=length {
            let (a, b) = bytes.split_at(split);
            let mut state = ProfileSha256State::new();
            assert_eq!(state.update(a), Ok(()));
            assert_eq!(state.update(&[]), Ok(()));
            assert_eq!(state.update(b), Ok(()));
            assert_eq!(
                state.finish(),
                expected,
                "length={length} split={split}"
            );
        }
    }
}

#[test]
fn incremental_sha256_preserves_empty_chunks_and_arbitrary_stride_splits() {
    for length in [
        0usize, 1, 2, 3, 7, 55, 56, 57, 63, 64, 65, 118, 119, 120, 121, 127,
        128, 129, 1024, 4097,
    ] {
        let bytes = (0..length)
            .map(|index| (index as u8).wrapping_mul(73).wrapping_add(17))
            .collect::<Vec<_>>();
        let expected = sha256_profile_bytes(&bytes);
        for stride in [
            1usize, 2, 3, 7, 31, 55, 56, 57, 63, 64, 65, 127, 128, 129, 256,
            1024,
        ] {
            let mut state = ProfileSha256State::new();
            for part in bytes.chunks(stride) {
                assert_eq!(state.update(&[]), Ok(()));
                assert_eq!(state.update(part), Ok(()));
            }
            assert_eq!(state.update(&[]), Ok(()));
            assert_eq!(
                state.finish(),
                expected,
                "length={length} stride={stride}"
            );
        }
    }
}

#[test]
fn incremental_sha256_cloned_prefixes_produce_independent_suffix_digests() {
    let prefix = vec![0xabu8; 47];
    let mut shared = ProfileSha256State::new();
    assert_eq!(shared.update(&prefix), Ok(()));
    let mut left = shared;
    let mut right = shared;
    assert_eq!(left.update(b"left"), Ok(()));
    assert_eq!(right.update(b"right"), Ok(()));
    let expected_left = [prefix.as_slice(), b"left"].concat();
    let expected_right = [prefix.as_slice(), b"right"].concat();
    assert_eq!(left.finish(), sha256_profile_bytes(&expected_left));
    assert_eq!(right.finish(), sha256_profile_bytes(&expected_right));
    assert_eq!(shared.finish(), sha256_profile_bytes(&prefix));
}

#[test]
fn chunked_entry_verification_matches_exact_slice_evidence() {
    for length in [
        0usize, 1, 3, 55, 56, 57, 63, 64, 65, 119, 120, 127, 128, 129, 1_024,
        4_097,
    ] {
        let bytes = (0..length)
            .map(|index| (index as u8).wrapping_mul(47).wrapping_add(3))
            .collect::<Vec<_>>();
        let entry = declaration(&bytes);
        let expected = verify_profile_entry_bytes(&entry, &bytes)
            .expect("complete in-memory bytes match declared digest");
        for stride in [1usize, 2, 3, 7, 55, 56, 63, 64, 65, 127, 128, 1_023] {
            let parts = bytes.chunks(stride);
            assert_eq!(
                verify_profile_entry_chunks(&entry, parts),
                Ok(expected),
                "length={length} stride={stride}"
            );
        }
        let parts = [b"".as_slice(), bytes.as_slice(), b"".as_slice()];
        assert_eq!(verify_profile_entry_chunks(&entry, parts), Ok(expected));
    }
}

#[test]
fn chunked_entry_verification_rejects_length_before_bad_sha_digest() {
    let bytes = b"critical member bytes";
    let mut entry = declaration(bytes);
    entry.digest = sha256_profile_bytes(b"unrelated");
    let short = [bytes.get(..7).expect("short prefix")];
    let observed_short = 7u64;
    assert_eq!(
        verify_profile_entry_chunks(&entry, short),
        Err(ProfileChunkVerificationError::Entry(
            ProfileEntryVerificationError::ByteLengthMismatch {
                declared: bytes.len() as u64,
                observed: observed_short,
            }
        ))
    );

    let length = bytes.len() as u64;
    let oversized = [bytes.as_slice(), b"extra".as_slice()];
    assert_eq!(
        verify_profile_entry_chunks(&entry, oversized),
        Err(ProfileChunkVerificationError::Entry(
            ProfileEntryVerificationError::ByteLengthMismatch {
                declared: length,
                observed: length + 5,
            }
        ))
    );
    assert_eq!(
        verify_profile_entry_chunks(&entry, [bytes.as_slice()]),
        Err(ProfileChunkVerificationError::Entry(
            ProfileEntryVerificationError::DigestMismatch,
        ))
    );
}

#[test]
fn chunked_entry_verification_stops_before_consuming_more_after_overrun() {
    use std::cell::Cell;
    let calls = Cell::new(0usize);
    let data = b"abc";
    let mut entry = declaration(data);
    entry.byte_length = 2;
    let chunks = [b"ab".as_slice(), b"c".as_slice(), b"surplus".as_slice()];
    let instrumented = chunks.into_iter().inspect(|_chunk| {
        calls.set(calls.get().saturating_add(1));
    });
    assert_eq!(
        verify_profile_entry_chunks(&entry, instrumented),
        Err(ProfileChunkVerificationError::Entry(
            ProfileEntryVerificationError::ByteLengthMismatch {
                declared: 2,
                observed: 3,
            }
        ))
    );
    assert_eq!(calls.get(), 2, "unseen third chunk is not consumed");
}

#[test]
fn incremental_hasher_tracks_admitted_byte_length_including_pending_prefix() {
    let mut state = ProfileSha256State::new();
    assert_eq!(state.bytes_hashed(), 0);
    for size in [0usize, 1, 2, 50, 0, 10, 1, 64, 5, 129] {
        let previous = state.bytes_hashed();
        let data = vec![0xabu8; size];
        assert_eq!(state.update(&data), Ok(()));
        assert_eq!(state.bytes_hashed(), previous + size as u64);
    }
}

#[test]
fn fallible_chunked_sha256_preserves_source_error_identity() {
    use std::io::ErrorKind;
    let bytes = b"deliberate profile payload";
    let entry = declaration(bytes);
    let partial: [Result<&[u8], ErrorKind>; 3] = [
        Ok(b"deliberate"),
        Err(ErrorKind::PermissionDenied),
        Ok(b" profile payload"),
    ];
    assert_eq!(
        verify_profile_entry_chunks_fallible(&entry, partial),
        Err(ProfileFallibleChunkVerificationError::Input(
            ErrorKind::PermissionDenied,
        ))
    );
    // Even a perfect complete prefix does not make a later source error
    // look like successful completion or a missing-byte length error.
    let complete_then_fail: [Result<&[u8], ErrorKind>; 2] =
        [Ok(bytes), Err(ErrorKind::Interrupted)];
    assert_eq!(
        verify_profile_entry_chunks_fallible(&entry, complete_then_fail),
        Err(ProfileFallibleChunkVerificationError::Input(
            ErrorKind::Interrupted,
        ))
    );

    let empty = declaration(b"");
    let failed_empty: [Result<&[u8], ErrorKind>; 1] =
        [Err(ErrorKind::UnexpectedEof)];
    assert_eq!(
        verify_profile_entry_chunks_fallible(&empty, failed_empty),
        Err(ProfileFallibleChunkVerificationError::Input(
            ErrorKind::UnexpectedEof,
        ))
    );
}

#[test]
fn fallible_chunked_sha256_success_matches_infallible_and_slice_paths() {
    use std::io::ErrorKind;
    for length in [0usize, 1, 55, 56, 63, 64, 65, 127, 128, 129, 1023] {
        let bytes = (0..length)
            .map(|index| (index as u8).wrapping_mul(37).wrapping_add(11))
            .collect::<Vec<_>>();
        let entry = declaration(&bytes);
        let expected = verify_profile_entry_bytes(&entry, &bytes)
            .expect("known declaration for complete bytes");
        for stride in [1usize, 2, 7, 63, 64, 65, 128, 256] {
            let input = bytes.chunks(stride).map(Ok::<&[u8], ErrorKind>);
            assert_eq!(
                verify_profile_entry_chunks_fallible(&entry, input),
                Ok(expected),
                "length={length} stride={stride}"
            );
        }
    }
}

#[test]
fn fallible_chunked_sha256_preserves_length_and_digest_error_precedence() {
    use std::io::ErrorKind;
    let bytes = b"abc";
    let mut entry = declaration(bytes);
    entry.digest = sha256_profile_bytes(b"wrong");
    let underflow: [Result<&[u8], ErrorKind>; 1] = [Ok(b"ab")];
    assert_eq!(
        verify_profile_entry_chunks_fallible(&entry, underflow),
        Err(ProfileFallibleChunkVerificationError::Verification(
            ProfileChunkVerificationError::Entry(
                ProfileEntryVerificationError::ByteLengthMismatch {
                    declared: 3,
                    observed: 2,
                },
            ),
        ))
    );
    let complete: [Result<&[u8], ErrorKind>; 1] = [Ok(bytes)];
    assert_eq!(
        verify_profile_entry_chunks_fallible(&entry, complete),
        Err(ProfileFallibleChunkVerificationError::Verification(
            ProfileChunkVerificationError::Entry(
                ProfileEntryVerificationError::DigestMismatch,
            ),
        ))
    );
    let extra: [Result<&[u8], ErrorKind>; 2] = [Ok(bytes), Ok(b"x")];
    assert_eq!(
        verify_profile_entry_chunks_fallible(&entry, extra),
        Err(ProfileFallibleChunkVerificationError::Verification(
            ProfileChunkVerificationError::Entry(
                ProfileEntryVerificationError::ByteLengthMismatch {
                    declared: 3,
                    observed: 4,
                },
            ),
        ))
    );
}

#[test]
fn fallible_sha256_rejects_known_overrun_before_reading_later_error() {
    use std::cell::Cell;
    let entry = declaration(b"ab");
    let reads = Cell::new(0usize);
    let chunks: [Result<&[u8], &'static str>; 3] =
        [Ok(b"ab"), Ok(b"z"), Err("unread transport failure")];
    let observed = chunks.into_iter().inspect(|_chunk| {
        reads.set(reads.get().saturating_add(1));
    });
    assert_eq!(
        verify_profile_entry_chunks_fallible(&entry, observed),
        Err(ProfileFallibleChunkVerificationError::Verification(
            ProfileChunkVerificationError::Entry(
                ProfileEntryVerificationError::ByteLengthMismatch {
                    declared: 2,
                    observed: 3,
                },
            ),
        ))
    );
    assert_eq!(reads.get(), 2, "third source error was never read");
}

#[test]
fn sha256_message_length_arithmetic_rejects_exact_overflow_without_allocating()
{
    let maximum = MAXIMUM_SHA256_MESSAGE_BYTES;
    assert_eq!(maximum, u64::MAX >> 3u32);
    assert_eq!(checked_sha256_message_bytes(0, 0), Ok(0));
    assert_eq!(checked_sha256_message_bytes(123, 0), Ok(123));
    assert_eq!(checked_sha256_message_bytes(maximum, 0), Ok(maximum));
    assert_eq!(checked_sha256_message_bytes(maximum - 1, 1), Ok(maximum));
    assert_eq!(checked_sha256_message_bytes(maximum, 1),
        Err(atrament_profile_entry_sha256::
            ProfileSha256UpdateError::MessageTooLong));
    assert_eq!(checked_sha256_message_bytes(maximum - 1, 2),
        Err(atrament_profile_entry_sha256::
            ProfileSha256UpdateError::MessageTooLong));
    assert_eq!(checked_sha256_message_bytes(u64::MAX, 0),
        Err(atrament_profile_entry_sha256::
            ProfileSha256UpdateError::MessageTooLong));
    if let Ok(capacity) = usize::try_from(maximum) {
        assert_eq!(checked_sha256_message_bytes(0, capacity), Ok(maximum));
        if let Some(too_big) = capacity.checked_add(1) {
            assert_eq!(checked_sha256_message_bytes(0, too_big),
                Err(atrament_profile_entry_sha256::
                    ProfileSha256UpdateError::MessageTooLong));
        }
    }
}

#[test]
fn incremental_hash_state_debug_never_exposes_buffered_profile_bytes() {
    let secret = b"private-stroke-vector-5168";
    let mut hasher = ProfileSha256State::new();
    hasher.update(secret).expect("first partial chunk");
    let first = hasher;
    hasher
        .update(&[b'x'; 64])
        .expect("cross one compression boundary");
    let second = hasher;
    for state in [first, second] {
        for rendered in [format!("{state:?}"), format!("{state:#?}")] {
            assert!(rendered.contains("ProfileSha256State"));
            assert!(!rendered.contains("private-stroke-vector-5168"));
            assert!(!rendered.contains("112, 114, 105, 118, 97, 116, 101"));
            assert!(!rendered.contains("state:"));
            assert!(!rendered.contains("block:"));
        }
    }
    let mut expected = secret.to_vec();
    expected.extend_from_slice(&[b'x'; 64]);
    assert_eq!(second.finish(), sha256_profile_bytes(&expected));
    assert_eq!(first.bytes_hashed(), secret.len() as u64);
}

#[test]
fn fallible_chunk_debug_redacts_arbitrary_transport_errors() {
    struct PrivateTransportFailure {
        selected_path: &'static str,
    }

    let entry = declaration(b"private member bytes");
    let source = [Err::<&[u8], _>(PrivateTransportFailure {
        selected_path: "path/private-reader-7591",
    })];
    let outcome = verify_profile_entry_chunks_fallible(&entry, source);
    for printed in [format!("{outcome:?}"), format!("{outcome:#?}")] {
        assert!(printed.contains("Input"));
        assert!(!printed.contains("private-reader-7591"));
    }
    match outcome {
        Err(ProfileFallibleChunkVerificationError::Input(reason)) => {
            assert_eq!(reason.selected_path, "path/private-reader-7591");
        }
        other => panic!("expected typed source failure: {other:?}"),
    }
    type Failure =
        ProfileFallibleChunkVerificationError<PrivateTransportFailure>;
    let verification = Failure::Verification(
        ProfileChunkVerificationError::Entry(
            ProfileEntryVerificationError::DigestMismatch,
        ),
    );
    let printed = format!("{verification:?}");
    assert!(printed.contains("Verification"));
    assert!(printed.contains("DigestMismatch"));
}
