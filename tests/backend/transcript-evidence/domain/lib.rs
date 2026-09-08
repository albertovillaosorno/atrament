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
//   - Regression evidence for transcript provenance and uncertainty retention.
// - Must-Not:
//   - Decode media, call engines, choose thresholds, infer speakers, normalize
//     text, create temporary files, or mutate semantic notebook state.
// - Allows:
//   - Inputs: Deterministic caller-owned transcript fixtures.
//   - Outputs: Assertions over media/job provenance and optional evidence.
//   - Side effects: Process-local test allocation only.
// - Split-When:
//   - Media-job lifecycle or semantic transcript structuring gains fixtures.
// - Merge-When:
//   - Transcript evidence moves into another pure ingestion harness.
// - Summary:
//   - Proves uncertainty stays explicit and original media remains distinct.
// - Description:
//   - Covers audio/video provenance, word evidence, and unresolved fragments.
// - Usage:
//   - Compile directly against the transcript-evidence domain.
// - Defaults:
//   - Missing timing/confidence/speaker evidence remains absent rather than
//     fake.
//
use atrament_transcript_evidence::{
    TranscriptEvidence, TranscriptMediaKind, TranscriptWord,
    UnresolvedTranscriptFragment,
};

#[test]
fn transcript_keeps_original_media_job_and_replaceable_engine_identity() {
    let transcript = TranscriptEvidence {
        engine_identity: "whisper-adapter/model-a",
        job_identity: "job-17",
        media_identity: "original-video-4",
        media_kind: TranscriptMediaKind::Video,
        unresolved_fragments:
            Vec::<UnresolvedTranscriptFragment<u8, &str, (u32, u32)>>::new(),
        words: vec![TranscriptWord {
            confidence: Some(93_u8),
            speaker: Some("speaker-a"),
            text: "hola",
            time_range: Some((1_200_u32, 1_430_u32)),
        }],
    };
    assert_eq!(transcript.media_identity, "original-video-4");
    assert_eq!(transcript.job_identity, "job-17");
    assert_eq!(transcript.engine_identity, "whisper-adapter/model-a");
    assert_eq!(transcript.media_kind, TranscriptMediaKind::Video);
}

#[test]
fn word_timing_confidence_and_speaker_remain_optional() {
    let word: TranscriptWord<u8, &str, &str, (u32, u32)> = TranscriptWord {
        confidence: None,
        speaker: None,
        text: "uncertain-metadata",
        time_range: None,
    };
    assert_eq!(word.confidence, None);
    assert_eq!(word.speaker, None);
    assert_eq!(word.time_range, None);
    assert_eq!(word.text, "uncertain-metadata");
}

#[test]
fn unresolved_fragment_is_not_promoted_to_resolved_word() {
    let unresolved = UnresolvedTranscriptFragment {
        confidence: Some(18_u8),
        text: "[inaudible syllables]",
        time_range: Some((4_000_u32, 4_600_u32)),
    };
    let transcript = TranscriptEvidence {
        engine_identity: "engine-b",
        job_identity: "job-3",
        media_identity: "audio-9",
        media_kind: TranscriptMediaKind::Audio,
        unresolved_fragments: vec![unresolved.clone()],
        words: Vec::<TranscriptWord<u8, &str, &str, (u32, u32)>>::new(),
    };
    assert_eq!(transcript.words.len(), 0);
    assert_eq!(transcript.unresolved_fragments, [unresolved]);
}

#[test]
fn resolved_word_order_is_preserved_without_adapter_rewriting() {
    let words = [
        TranscriptWord::<u8, &str, _, (u32, u32)> {
            confidence: Some(99),
            speaker: None,
            text: "primero",
            time_range: Some((100, 200)),
        },
        TranscriptWord {
            confidence: Some(97),
            speaker: None,
            text: "segundo",
            time_range: Some((210, 330)),
        },
    ];
    assert_eq!(words[0].text, "primero");
    assert_eq!(words[1].text, "segundo");
}
