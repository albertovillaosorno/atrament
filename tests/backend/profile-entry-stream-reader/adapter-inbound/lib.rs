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

// Test guards require the typed stream outcome before inspecting its evidence.
macro_rules! require_ok {
    ($expression:expr) => {{
        let result = $expression;
        assert!(
            result.is_ok(),
            "expected verified entry: {:?}",
            result.as_ref().err()
        );
        let Ok(value) = result else {
            return;
        };
        value
    }};
}

macro_rules! require_err {
    ($expression:expr) => {{
        let result = $expression;
        assert!(
            result.is_err(),
            "expected rejection, got: {:?}",
            result.as_ref().ok()
        );
        let Err(failure) = result else {
            return;
        };
        failure
    }};
}

struct FailAfterComplete {
    after: ErrorKind,
    content: Cursor<&'static [u8]>,
}

#[expect(clippy::missing_trait_methods, reason = "mock uses Read defaults")]
impl Read for FailAfterComplete {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.content.position() >= byte_count(self.content.get_ref().len()) {
            return Err(io::Error::new(
                self.after,
                "injected terminal read failure",
            ));
        }
        self.content.read(buf)
    }
}

struct FailAtOffset<'source> {
    after_bytes: usize,
    calls: usize,
    kind: ErrorKind,
    source: Cursor<&'source [u8]>,
}

#[expect(clippy::missing_trait_methods, reason = "mock uses Read defaults")]
impl Read for FailAtOffset<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.calls = self.calls.saturating_add(1);
        let consumed =
            usize::try_from(self.source.position()).unwrap_or(usize::MAX);
        if consumed >= self.after_bytes {
            return Err(io::Error::new(
                self.kind,
                "source read failure marker",
            ));
        }
        let admitted = self.after_bytes.saturating_sub(consumed).min(buf.len());
        self.source
            .read(buf.get_mut(..admitted).unwrap_or_default())
    }
}

struct InstrumentedReader<R> {
    limit_per_call: usize,
    read_calls: usize,
    requested_max: usize,
    returned_bytes: usize,
    source: R,
}

#[expect(clippy::missing_trait_methods, reason = "mock uses Read defaults")]
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

struct PrivateIoFailure;

#[expect(clippy::missing_trait_methods, reason = "mock uses Read defaults")]
impl Read for PrivateIoFailure {
    fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::new(
            ErrorKind::PermissionDenied,
            "selected-private-path-7391: source payload inaccessible",
        ))
    }
}

fn byte_count(length: usize) -> u64 {
    u64::try_from(length).unwrap_or(u64::MAX)
}

const fn traced(
    input: &[u8],
    per_call: usize,
) -> InstrumentedReader<Cursor<&[u8]>> {
    InstrumentedReader {
        limit_per_call: per_call,
        read_calls: 0,
        requested_max: 0,
        returned_bytes: 0,
        source: Cursor::new(input),
    }
}

fn declaration(bytes: &[u8]) -> ProfileManifestEntry {
    ProfileManifestEntry {
        byte_length: byte_count(bytes.len()),
        digest: sha256_profile_bytes(bytes),
        media_type: "application/json".into(),
        path: "sections/sample.json".into(),
    }
}

#[test]
fn full_sources_match_exact_digest_across_chunk_and_buffer_boundaries() {
    for len in [
        0usize, 1, 2, 7, 55, 56, 63, 64, 65, 119, 120, 127, 128, 129, 1023,
        8191, 8192, 8193, 16_385, 65_537,
    ] {
        let bytes = (0..len)
            .map(|index| {
                u8::try_from(index.rem_euclid(256))
                    .unwrap_or_default()
                    .wrapping_mul(29)
            })
            .collect::<Vec<_>>();
        let entry = declaration(&bytes);
        let expected = ProfileEntryEvidence {
            byte_length: byte_count(len),
            digest: entry.digest,
        };
        for per_call in [1usize, 2, 3, 7, 31, 64, 511, 4096, 8192, 12_000] {
            let mut source = traced(&bytes, per_call);
            let observed = require_ok!(verify_profile_entry_from_reader(
                &mut source,
                &entry,
                byte_count(len),
            ));
            assert_eq!(observed, expected, "length={len}, per_call={per_call}");
            assert!(source.read_calls > 0);
            assert_eq!(source.returned_bytes, len);
            assert!(source.requested_max <= PROFILE_STREAM_READ_BUFFER_BYTES);
            assert_eq!(source.source.position(), byte_count(len));
        }
    }
}

#[test]
fn declared_limit_denies_without_any_source_read_or_digest() {
    let bytes = b"one small input";
    let entry = declaration(bytes);
    let mut source = traced(bytes, 1);
    let outcome = verify_profile_entry_from_reader(&mut source, &entry, 3);
    assert!(matches!(
        outcome,
        Err(ProfileEntryStreamReadError::DeclaredLimitExceeded {
            declared,
            limit: 3,
        }) if declared == byte_count(bytes.len()),
    ));
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
            byte_count(content.len())
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
            byte_count(content.len())
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

#[test]
fn read_error_after_complete_declared_prefix_is_not_eof() {
    let bytes = b"complete";
    let entry = declaration(bytes);
    let mut source = FailAfterComplete {
        after: ErrorKind::PermissionDenied,
        content: Cursor::new(bytes),
    };
    let outcome = verify_profile_entry_from_reader(&mut source, &entry, 100);
    assert!(
        matches!(
            &outcome,
            Err(ProfileEntryStreamReadError::Input(error))
                if error.kind() == ErrorKind::PermissionDenied
                    && error.to_string().contains("injected"),
        ),
        "expected terminal source failure: {outcome:?}"
    );
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

#[test]
fn interrupted_and_wouldblock_reads_are_not_retried_or_reinterpreted_as_eof() {
    let bytes = b"some relevant source bytes";
    let entry = declaration(bytes);
    let source_marker = "source read failure marker";
    for (kind, offset) in [
        (ErrorKind::Interrupted, 0usize),
        (ErrorKind::WouldBlock, 1),
        (ErrorKind::TimedOut, bytes.len() - 1),
        (ErrorKind::BrokenPipe, bytes.len()),
    ] {
        let mut source = FailAtOffset {
            after_bytes: offset,
            calls: 0,
            kind,
            source: Cursor::new(bytes.as_slice()),
        };
        let observed = verify_profile_entry_from_reader(
            &mut source,
            &entry,
            byte_count(bytes.len()),
        );
        assert!(
            matches!(
                &observed,
                Err(ProfileEntryStreamReadError::Input(error))
                    if error.kind() == kind
                        && error.to_string().contains(source_marker),
            ),
            "source error was not retained: {observed:?}"
        );
        assert_eq!(source.source.position(), byte_count(offset));
    }
}

#[test]
fn errors_across_all_read_buffer_seams_remain_source_failures() {
    let bytes = vec![b'v'; PROFILE_STREAM_READ_BUFFER_BYTES * 2 + 17];
    let entry = declaration(&bytes);
    let seam = PROFILE_STREAM_READ_BUFFER_BYTES;
    let offsets = [
        0,
        1,
        31,
        55,
        56,
        63,
        64,
        65,
        seam - 1,
        seam,
        seam + 1,
        seam * 2 - 1,
        seam * 2,
        seam * 2 + 1,
        bytes.len() - 1,
        bytes.len(),
    ];
    for kind in [
        ErrorKind::Interrupted,
        ErrorKind::TimedOut,
        ErrorKind::PermissionDenied,
    ] {
        for offset in offsets {
            let mut source = FailAtOffset {
                after_bytes: offset,
                calls: 0,
                kind,
                source: Cursor::new(bytes.as_slice()),
            };
            let result = verify_profile_entry_from_reader(
                &mut source,
                &entry,
                entry.byte_length,
            );
            assert!(
                matches!(
                    &result,
                    Err(ProfileEntryStreamReadError::Input(error))
                        if error.kind() == kind,
                ),
                "error at {offset} must not become digest evidence: {result:?}"
            );
            assert_eq!(source.source.position(), byte_count(offset));
            assert!(source.calls > 0);
        }
    }
}

#[test]
fn invalid_digest_does_not_hide_error_at_final_eof_probe() {
    let bytes = b"a complete prefix";
    let mut entry = declaration(bytes);
    entry.digest = sha256_profile_bytes(b"different content");
    let mut source = FailAtOffset {
        after_bytes: bytes.len(),
        calls: 0,
        kind: ErrorKind::ConnectionReset,
        source: Cursor::new(bytes.as_slice()),
    };
    let observed = verify_profile_entry_from_reader(
        &mut source,
        &entry,
        byte_count(bytes.len()),
    );
    assert!(
        matches!(
            &observed,
            Err(ProfileEntryStreamReadError::Input(error))
                if error.kind() == ErrorKind::ConnectionReset,
        ),
        "probe source failure must precede digest: {observed:?}"
    );
    assert_eq!(source.source.position(), byte_count(bytes.len()));
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
        byte_count(expected.len()),
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
    assert_eq!(source.source.position(), byte_count(expected.len() + 1));
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
            byte_count(expected.len()),
        ),
        Err(ProfileEntryStreamReadError::Verification(
            ProfileChunkVerificationError::Entry(
                ProfileEntryVerificationError::ByteLengthMismatch { .. },
            ),
        ))
    ));
    assert_eq!(
        physically_unbounded.position(),
        byte_count(expected.len() + 1)
    );

    // std::io::Take is allowed to manufacture EOF at its own limit.
    // SHA success here is restricted to the caller-supplied Take view and
    // cannot attest the hidden original source is exactly this long.
    let cursor = Cursor::new(actual_source.as_slice());
    let mut restricted = cursor.take(byte_count(expected.len()));
    let accepted = require_ok!(verify_profile_entry_from_reader(
        &mut restricted,
        &entry,
        byte_count(expected.len()),
    ));
    assert_eq!(accepted.digest, entry.digest);
    assert_eq!(restricted.get_ref().position(), byte_count(expected.len()));
    assert!(restricted.get_ref().get_ref().len() > expected.len());
}

#[test]
fn read_error_before_length_fails_even_when_declared_sha_matches_prefix() {
    let complete = b"abcd";
    let entry = declaration(complete);
    let mut source = FailAtOffset {
        after_bytes: 3,
        calls: 0,
        kind: ErrorKind::UnexpectedEof,
        source: Cursor::new(complete.as_slice()),
    };
    // The upstream I/O error is not the same as observing a clean EOF at
    // byte three, and it must not be reclassified as length mismatch.
    let observed = verify_profile_entry_from_reader(&mut source, &entry, 4);
    assert!(
        matches!(
            &observed,
            Err(ProfileEntryStreamReadError::Input(error))
                if error.kind() == ErrorKind::UnexpectedEof,
        ),
        "upstream failure lost: {observed:?}"
    );
    assert_eq!(source.source.position(), 3);
}

#[test]
fn invalid_reader_count_cannot_expand_slice_or_panic() {
    struct InvalidLengthReader {
        calls: usize,
    }
    #[expect(clippy::missing_trait_methods, reason = "mock uses Read defaults")]
    impl Read for InvalidLengthReader {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            self.calls = self.calls.saturating_add(1);
            Ok(buf.len().saturating_add(1))
        }
    }
    let entry = declaration(b"abc");
    let mut source = InvalidLengthReader { calls: 0 };
    let outcome = verify_profile_entry_from_reader(&mut source, &entry, 3);
    assert!(
        matches!(
            &outcome,
            Err(ProfileEntryStreamReadError::Input(error))
                if error.kind() == ErrorKind::InvalidData,
        ),
        "invalid Read count was not refused: {outcome:?}"
    );
    assert_eq!(source.calls, 1);
}

#[test]
fn premature_zero_count_is_not_retried_into_hidden_later_bytes() {
    struct PrematureEof {
        calls: usize,
    }
    #[expect(clippy::missing_trait_methods, reason = "mock uses Read defaults")]
    impl Read for PrematureEof {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            self.calls = self.calls.saturating_add(1);
            if self.calls == 1 {
                return Ok(0);
            }
            if let Some(slot) = buf.get_mut(0) {
                *slot = b'a';
            }
            Ok(1)
        }
    }
    let entry = declaration(b"a");
    let mut source = PrematureEof { calls: 0 };
    let result = verify_profile_entry_from_reader(&mut source, &entry, 1);
    assert!(matches!(
        result,
        Err(ProfileEntryStreamReadError::Verification(
            ProfileChunkVerificationError::Entry(
                ProfileEntryVerificationError::ByteLengthMismatch {
                    declared: 1,
                    observed: 0,
                },
            ),
        )),
    ));
    assert_eq!(source.calls, 1, "no later bytes were used to bypass EOF");
}

#[test]
fn source_error_debug_retains_kind_without_leaking_private_message() {
    let entry = declaration(b"private section payload");
    let mut source = PrivateIoFailure;
    let failure = require_err!(verify_profile_entry_from_reader(
        &mut source,
        &entry,
        entry.byte_length,
    ));
    for rendered in [format!("{failure:?}"), format!("{failure:#?}")] {
        assert!(rendered.contains("Input"));
        assert!(rendered.contains("PermissionDenied"));
        assert!(!rendered.contains("selected-private-path-7391"));
        assert!(!rendered.contains("source payload inaccessible"));
    }
    let private_marker = "selected-private-path-7391";
    assert!(
        matches!(
            &failure,
            ProfileEntryStreamReadError::Input(original)
                if original.kind() == ErrorKind::PermissionDenied
                    && original.to_string().contains(private_marker),
        ),
        "expected source error: {failure:?}"
    );
    // Safe numeric diagnostics remain visible for non-I/O errors.
    let mut refused_source = PrivateIoFailure;
    let oversized = require_err!(verify_profile_entry_from_reader(
        &mut refused_source,
        &entry,
        entry.byte_length - 1,
    ));
    let rendered = format!("{oversized:?}");
    assert!(rendered.contains("DeclaredLimitExceeded"));
    assert!(rendered.contains("declared"));
    assert!(rendered.contains("limit"));
}
