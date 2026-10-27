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
//   - Source-backed bounded SHA-256 verifier stress and failure precedence.
// - Must-Not:
//   - Open real files, admit a profile, infer paths or mutate session state.
// - Allows:
//   - Inputs: In-memory reader mocks and deliberate declared entry metadata.
//   - Outputs: Exact source-error, stream length and hash assertions.
//   - Side effects: Local in-memory allocations only.
// - Split-When:
//   - Real selected filesystem adapters need an independent trust boundary.
// - Merge-When:
//   - Profile Import acceptance tests own complete source and manifest proof.
// - Summary:
//   - Proves a fixed-memory reader cannot confuse prefixes with full entries.
// - Description:
//   - Requires an actual EOF probe, even for declared zero-byte members.
// - Usage:
//   - Run with Cargo workspace offline tests and strict Clippy.
// - Defaults:
//   - Incomplete, oversized and failing readers never produce digest evidence.
//

//! In-memory adversarial readers for bounded stream-integrity checks.

use std::io::{self, Cursor, ErrorKind, Read};

use atrament_handwriting_profile_container::{
    ProfileEntryEvidence, ProfileEntryVerificationError, ProfileManifestEntry,
};
use atrament_profile_entry_sha256::{
    ProfileChunkVerificationError, ProfileSha256UpdateError,
    sha256_profile_bytes,
};
use atrament_profile_entry_stream_reader::{
    PROFILE_STREAM_READ_BUFFER_BYTES, ProfileEntryStreamReadError,
    verify_profile_entry_from_reader,
};

fn declaration(bytes: &[u8]) -> ProfileManifestEntry {
    ProfileManifestEntry {
        byte_length: u64::try_from(bytes.len()).expect("test length fits u64"),
        digest: sha256_profile_bytes(bytes),
        media_type: "application/json".into(),
        path: "sections/sample.json".into(),
    }
}

struct InstrumentedReader<R> {
    source: R,
    limit_per_call: usize,
    read_calls: usize,
    requested_max: usize,
    returned_bytes: usize,
}

impl<R: Read> Read for InstrumentedReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.read_calls = self.read_calls.saturating_add(1);
        self.requested_max = self.requested_max.max(buf.len());
        let capacity = self.limit_per_call.min(buf.len());
        let bytes = self
            .source
            .read(buf.get_mut(..capacity).unwrap_or_default())?;
        self.returned_bytes = self.returned_bytes.saturating_add(bytes);
        Ok(bytes)
    }
}

fn traced<'a>(
    input: &'a [u8],
    per_call: usize,
) -> InstrumentedReader<Cursor<&'a [u8]>> {
    InstrumentedReader {
        source: Cursor::new(input),
        limit_per_call: per_call,
        read_calls: 0,
        requested_max: 0,
        returned_bytes: 0,
    }
}

#[test]
fn full_sources_match_exact_digest_across_chunk_and_buffer_boundaries() {
    for len in [
        0usize, 1, 2, 7, 55, 56, 63, 64, 65, 119, 120, 127, 128, 129, 1023,
        8191, 8192, 8193, 16_385, 65_537,
    ] {
        let bytes = (0..len)
            .map(|index| (index as u8).wrapping_mul(29))
            .collect::<Vec<_>>();
        let entry = declaration(&bytes);
        let expected = ProfileEntryEvidence {
            byte_length: len as u64,
            digest: entry.digest,
        };
        for per_call in [1usize, 2, 3, 7, 31, 64, 511, 4096, 8192, 12_000] {
            let mut source = traced(&bytes, per_call);
            let observed = verify_profile_entry_from_reader(
                &mut source,
                &entry,
                len as u64,
            )
            .expect("exact bounded source verifies");
            assert_eq!(observed, expected, "length={len}, per_call={per_call}");
            assert!(source.read_calls > 0);
            assert_eq!(source.returned_bytes, len);
            assert!(source.requested_max <= PROFILE_STREAM_READ_BUFFER_BYTES);
            assert_eq!(source.source.position(), len as u64);
        }
    }
}

#[test]
fn declared_limit_denies_without_any_source_read_or_digest() {
    let bytes = b"one small input";
    let entry = declaration(bytes);
    let mut source = traced(bytes, 1);
    match verify_profile_entry_from_reader(&mut source, &entry, 3) {
        Err(ProfileEntryStreamReadError::DeclaredLimitExceeded {
            declared,
            limit,
        }) => {
            assert_eq!(declared, bytes.len() as u64);
            assert_eq!(limit, 3);
        }
        other => panic!("expected pre-read limit refusal: {other:?}"),
    }
    assert_eq!(source.read_calls, 0);
    assert_eq!(source.returned_bytes, 0);
    assert_eq!(source.source.position(), 0);
}

#[test]
fn zero_length_still_probes_eof_and_rejects_unexpected_payload() {
    let entry = declaration(b"");
    let mut empty = traced(b"", 100);
    assert!(verify_profile_entry_from_reader(&mut empty, &entry, 0).is_ok());
    assert_eq!(empty.read_calls, 1, "empty source still proved EOF");
    let mut nonempty = traced(b"unexpected", 100);
    assert!(matches!(
        verify_profile_entry_from_reader(&mut nonempty, &entry, 0),
        Err(ProfileEntryStreamReadError::Verification(
            ProfileChunkVerificationError::Entry(
                ProfileEntryVerificationError::ByteLengthMismatch {
                    declared: 0,
                    observed: 1,
                },
            ),
        )),
    ));
    assert_eq!(nonempty.read_calls, 1);
    assert_eq!(nonempty.source.position(), 1);
}

#[test]
fn stream_underflow_precedes_unrelated_digest_mismatch() {
    let content = b"0123456789";
    let mut entry = declaration(content);
    entry.digest = sha256_profile_bytes(b"unrelated");
    let mut short = traced(&content[..8], 3);
    assert!(matches!(
        verify_profile_entry_from_reader(
            &mut short,
            &entry,
            content.len() as u64
        ),
        Err(ProfileEntryStreamReadError::Verification(
            ProfileChunkVerificationError::Entry(
                ProfileEntryVerificationError::ByteLengthMismatch {
                    declared: 10,
                    observed: 8,
                },
            ),
        )),
    ));
    let mut complete = traced(content, 3);
    assert!(matches!(
        verify_profile_entry_from_reader(
            &mut complete,
            &entry,
            content.len() as u64
        ),
        Err(ProfileEntryStreamReadError::Verification(
            ProfileChunkVerificationError::Entry(
                ProfileEntryVerificationError::DigestMismatch,
            ),
        )),
    ));
}

#[test]
fn oversized_member_detected_by_single_extra_byte_not_bulk_read() {
    let content = b"0123456789EXTRA AND MORE";
    let entry = declaration(b"0123456789");
    let mut source = traced(content, 20);
    assert!(matches!(
        verify_profile_entry_from_reader(&mut source, &entry, 10),
        Err(ProfileEntryStreamReadError::Verification(
            ProfileChunkVerificationError::Entry(
                ProfileEntryVerificationError::ByteLengthMismatch {
                    declared: 10,
                    observed: 11,
                },
            ),
        )),
    ));
    assert_eq!(source.source.position(), 11);
    assert_eq!(source.returned_bytes, 11);
    assert_eq!(source.read_calls, 2);
}

struct FailAfterComplete {
    content: Cursor<&'static [u8]>,
    after: ErrorKind,
}

impl Read for FailAfterComplete {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        if self.content.position() >= self.content.get_ref().len() as u64 {
            return Err(io::Error::new(
                self.after,
                "injected terminal read failure",
            ));
        }
        self.content.read(bytes)
    }
}

#[test]
fn read_error_after_complete_declared_prefix_is_not_eof() {
    let bytes = b"complete";
    let entry = declaration(bytes);
    let mut source = FailAfterComplete {
        content: Cursor::new(bytes),
        after: ErrorKind::PermissionDenied,
    };
    match verify_profile_entry_from_reader(&mut source, &entry, 100) {
        Err(ProfileEntryStreamReadError::Input(error)) => {
            assert_eq!(error.kind(), ErrorKind::PermissionDenied);
            assert!(error.to_string().contains("injected"));
        }
        other => panic!("expected source error after full prefix: {other:?}"),
    }
}

#[test]
fn sha256_message_length_refuses_huge_declaration_without_read() {
    let mut entry = declaration(b"");
    entry.byte_length = (u64::MAX >> 3u32).saturating_add(1);
    let mut source = traced(b"", 1);
    assert!(matches!(
        verify_profile_entry_from_reader(&mut source, &entry, u64::MAX),
        Err(ProfileEntryStreamReadError::Verification(
            ProfileChunkVerificationError::Hasher(
                ProfileSha256UpdateError::MessageTooLong,
            ),
        )),
    ));
    assert_eq!(source.read_calls, 0);
}

struct FailAtOffset<'a> {
    source: Cursor<&'a [u8]>,
    kind: ErrorKind,
    after_bytes: usize,
    calls: usize,
}

impl Read for FailAtOffset<'_> {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        self.calls = self.calls.saturating_add(1);
        let consumed =
            usize::try_from(self.source.position()).unwrap_or(usize::MAX);
        if consumed >= self.after_bytes {
            return Err(io::Error::new(
                self.kind,
                "source read failure marker",
            ));
        }
        let admitted =
            self.after_bytes.saturating_sub(consumed).min(output.len());
        self.source
            .read(output.get_mut(..admitted).unwrap_or_default())
    }
}

#[test]
fn interrupted_and_wouldblock_reads_are_not_retried_or_reinterpreted_as_eof() {
    let bytes = b"some relevant source bytes";
    let entry = declaration(bytes);
    for (kind, offset) in [
        (ErrorKind::Interrupted, 0usize),
        (ErrorKind::WouldBlock, 1),
        (ErrorKind::TimedOut, bytes.len() - 1),
        (ErrorKind::BrokenPipe, bytes.len()),
    ] {
        let mut source = FailAtOffset {
            source: Cursor::new(bytes.as_slice()),
            kind,
            after_bytes: offset,
            calls: 0,
        };
        let observed = verify_profile_entry_from_reader(
            &mut source,
            &entry,
            bytes.len() as u64,
        );
        match observed {
            Err(ProfileEntryStreamReadError::Input(error)) => {
                assert_eq!(error.kind(), kind);
                assert!(
                    error.to_string().contains("source read failure marker")
                );
            }
            other => panic!("source error was not retained: {other:?}"),
        }
        assert_eq!(source.source.position(), offset as u64);
    }
}

#[test]
fn invalid_digest_does_not_hide_error_at_final_eof_probe() {
    let bytes = b"a complete prefix";
    let mut entry = declaration(bytes);
    entry.digest = sha256_profile_bytes(b"different content");
    let mut source = FailAtOffset {
        source: Cursor::new(bytes.as_slice()),
        kind: ErrorKind::ConnectionReset,
        after_bytes: bytes.len(),
        calls: 0,
    };
    let observed = verify_profile_entry_from_reader(
        &mut source,
        &entry,
        bytes.len() as u64,
    );
    match observed {
        Err(ProfileEntryStreamReadError::Input(error)) => {
            assert_eq!(error.kind(), ErrorKind::ConnectionReset);
        }
        other => panic!("probe source error must beat digest: {other:?}"),
    }
    assert_eq!(source.source.position(), bytes.len() as u64);
}

#[test]
fn oversized_source_with_large_declared_prefix_reads_only_one_extra_byte() {
    let expected = vec![0xa5u8; PROFILE_STREAM_READ_BUFFER_BYTES + 37];
    let mut supplied = expected.clone();
    supplied.extend_from_slice(&vec![0xfe; 500_000]);
    let entry = declaration(&expected);
    let mut source = traced(&supplied, usize::MAX);
    let observed = verify_profile_entry_from_reader(
        &mut source,
        &entry,
        expected.len() as u64,
    );
    assert!(matches!(
        observed,
        Err(ProfileEntryStreamReadError::Verification(
            ProfileChunkVerificationError::Entry(
                ProfileEntryVerificationError::ByteLengthMismatch { .. },
            ),
        )),
    ));
    assert_eq!(source.returned_bytes, expected.len() + 1);
    assert_eq!(source.source.position(), (expected.len() + 1) as u64);
    assert!(source.requested_max <= PROFILE_STREAM_READ_BUFFER_BYTES);
}

#[test]
fn eof_is_proven_only_for_supplied_reader_not_hidden_upstream_sources() {
    let expected = b"first nine";
    let actual_source = b"first nine + additional bytes";
    let entry = declaration(expected);
    let mut physically_unbounded = Cursor::new(actual_source.as_slice());
    assert!(matches!(
        verify_profile_entry_from_reader(
            &mut physically_unbounded,
            &entry,
            expected.len() as u64,
        ),
        Err(ProfileEntryStreamReadError::Verification(
            ProfileChunkVerificationError::Entry(
                ProfileEntryVerificationError::ByteLengthMismatch { .. },
            ),
        ))
    ));
    assert_eq!(physically_unbounded.position(), (expected.len() + 1) as u64);

    // std::io::Take is allowed to manufacture EOF at its own limit.
    // SHA success here is restricted to the caller-supplied Take view and
    // cannot attest the hidden original source is exactly this long.
    let cursor = Cursor::new(actual_source.as_slice());
    let mut restricted = cursor.take(expected.len() as u64);
    let accepted = verify_profile_entry_from_reader(
        &mut restricted,
        &entry,
        expected.len() as u64,
    )
    .expect("exact supplied Take view hashes correctly");
    assert_eq!(accepted.digest, entry.digest);
    assert_eq!(restricted.get_ref().position(), expected.len() as u64);
    assert!(restricted.get_ref().get_ref().len() > expected.len());
}

#[test]
fn read_error_before_length_fails_even_when_declared_sha_matches_prefix() {
    let complete = b"abcd";
    let entry = declaration(complete);
    let mut source = FailAtOffset {
        source: Cursor::new(complete.as_slice()),
        kind: ErrorKind::UnexpectedEof,
        after_bytes: 3,
        calls: 0,
    };
    // The upstream I/O error is not the same as observing a clean EOF at
    // byte three, and it must not be reclassified as length mismatch.
    let observed = verify_profile_entry_from_reader(&mut source, &entry, 4);
    match observed {
        Err(ProfileEntryStreamReadError::Input(error)) => {
            assert_eq!(error.kind(), ErrorKind::UnexpectedEof);
        }
        other => panic!("upstream failure lost: {other:?}"),
    }
    assert_eq!(source.source.position(), 3);
}
