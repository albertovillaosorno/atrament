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
//   - Regression evidence for reviewed transcript structure and uncertainty.
// - Must-Not:
//   - Segment transcripts, choose roles, resolve uncertainty, mutate notebooks,
//     invoke transcription engines, or interpret confidence/speaker evidence.
// - Allows:
//   - Inputs: Deterministic transcript evidence and caller-reviewed spans.
//   - Outputs: Assertions over retained provenance, span roles, and
//     uncertainty.
//   - Side effects: Process-local test allocation only.
// - Split-When:
//   - Transcript correction or notebook projection gains independent fixtures.
// - Merge-When:
//   - Reviewed transcript structure is tested by semantic ingestion directly.
// - Summary:
//   - Proves structuring does not hide source evidence or uncertain fragments.
// - Description:
//   - Covers section/definition/example/formula roles and unresolved retention.
// - Usage:
//   - Compile directly against transcript evidence and transcript structure.
// - Defaults:
//   - Explicit unresolved fragments remain unresolved without correction.
//
use atrament_transcript_evidence::{
    TranscriptEvidence, TranscriptMediaKind, TranscriptOrigin, TranscriptWord,
    UnresolvedTranscriptFragment,
};
use atrament_transcript_structure::{
    ReviewedTranscriptRole, ReviewedTranscriptSource,
    ReviewedTranscriptSourceLocation, ReviewedTranscriptSpan,
    TranscriptStructureError, review_transcript_structure,
};

type Word = TranscriptWord<u8, &'static str, &'static str, (u32, u32)>;
type Unresolved = UnresolvedTranscriptFragment<u8, &'static str, (u32, u32)>;
type Origin = TranscriptOrigin<&'static str, &'static str, &'static str>;
type Transcript = TranscriptEvidence<Origin, Word, Unresolved>;

fn transcript_fixture() -> Transcript {
    TranscriptEvidence {
        origin: TranscriptOrigin {
            engine_identity: "engine/model-a",
            job_identity: "job-17",
            media_identity: "lecture-video-3",
            media_kind: TranscriptMediaKind::Video,
        },
        unresolved_fragments: vec![UnresolvedTranscriptFragment {
            confidence: Some(21),
            text: "[inaudible denominator]",
            time_range: Some((3_400, 3_900)),
        }],
        words: vec![
            TranscriptWord {
                confidence: Some(96),
                speaker: Some("teacher"),
                text: "Definition",
                time_range: Some((1_000, 1_220)),
            },
            TranscriptWord {
                confidence: Some(94),
                speaker: Some("teacher"),
                text: "velocity",
                time_range: Some((1_230, 1_520)),
            },
            TranscriptWord {
                confidence: None,
                speaker: None,
                text: "v=dx/dt",
                time_range: Some((2_000, 2_600)),
            },
        ],
    }
}

#[test]
fn reviewed_structure_keeps_complete_transcript_provenance() {
    let transcript = transcript_fixture();
    let spans = vec![ReviewedTranscriptSpan {
        role: ReviewedTranscriptRole::Section,
        source: ReviewedTranscriptSource::ResolvedWords(
            &transcript.words[0..1],
        ),
    }];
    let reviewed = review_transcript_structure(&transcript, spans)
        .expect("resolved section review is admitted");
    assert_eq!(
        reviewed.transcript().origin.media_identity,
        "lecture-video-3",
    );
    assert_eq!(reviewed.transcript().origin.job_identity, "job-17");
    assert_eq!(reviewed.transcript().origin.engine_identity, "engine/model-a");
    assert_eq!(
        reviewed.transcript().origin.media_kind,
        TranscriptMediaKind::Video,
    );
}

#[test]
fn reviewed_roles_preserve_resolved_word_evidence_exactly() {
    let transcript = transcript_fixture();
    let spans = vec![
        ReviewedTranscriptSpan {
            role: ReviewedTranscriptRole::Definition,
            source: ReviewedTranscriptSource::ResolvedWords(
                &transcript.words[0..2],
            ),
        },
        ReviewedTranscriptSpan {
            role: ReviewedTranscriptRole::Example,
            source: ReviewedTranscriptSource::ResolvedWords(
                &transcript.words[1..2],
            ),
        },
        ReviewedTranscriptSpan {
            role: ReviewedTranscriptRole::Formula,
            source: ReviewedTranscriptSource::ResolvedWords(
                &transcript.words[2..3],
            ),
        },
    ];
    let reviewed = review_transcript_structure(&transcript, spans)
        .expect("resolved semantic roles are admitted");
    let ReviewedTranscriptSource::ResolvedWords(definition) =
        reviewed.spans()[0].source
    else {
        panic!("definition source must remain resolved words");
    };
    assert_eq!(definition[0].confidence, Some(96));
    assert_eq!(definition[0].speaker, Some("teacher"));
    assert_eq!(definition[0].time_range, Some((1_000, 1_220)));
    assert_eq!(definition[0].text, "Definition");
    assert_eq!(
        reviewed.locations(),
        [
            ReviewedTranscriptSourceLocation::ResolvedWords {
                end_word: 2,
                start_word: 0,
            },
            ReviewedTranscriptSourceLocation::ResolvedWords {
                end_word: 2,
                start_word: 1,
            },
            ReviewedTranscriptSourceLocation::ResolvedWords {
                end_word: 3,
                start_word: 2,
            },
        ],
    );
    let ReviewedTranscriptSource::ResolvedWords(formula) =
        reviewed.spans()[2].source
    else {
        panic!("formula source must remain resolved words");
    };
    assert_eq!(formula[0].confidence, None);
    assert_eq!(formula[0].speaker, None);
    assert_eq!(formula[0].text, "v=dx/dt");
}

#[test]
fn unresolved_fragment_keeps_text_timing_and_confidence() {
    let transcript = transcript_fixture();
    let spans = vec![ReviewedTranscriptSpan {
        role: ReviewedTranscriptRole::Unresolved,
        source: ReviewedTranscriptSource::UnresolvedFragment(
            &transcript.unresolved_fragments[0],
        ),
    }];
    let reviewed = review_transcript_structure(&transcript, spans)
        .expect("unresolved review is admitted");
    let ReviewedTranscriptSource::UnresolvedFragment(fragment) =
        reviewed.spans()[0].source
    else {
        panic!("unresolved source must remain an unresolved fragment");
    };
    assert_eq!(fragment.text, "[inaudible denominator]");
    assert_eq!(fragment.confidence, Some(21));
    assert_eq!(fragment.time_range, Some((3_400, 3_900)));
    assert_eq!(
        reviewed.locations(),
        [ReviewedTranscriptSourceLocation::UnresolvedFragment {
            fragment_index: 0,
        }],
    );
}

#[test]
fn overlapping_and_repeated_review_spans_keep_exact_source_ranges() {
    let transcript = transcript_fixture();
    let spans = vec![
        ReviewedTranscriptSpan {
            role: ReviewedTranscriptRole::Definition,
            source: ReviewedTranscriptSource::ResolvedWords(
                &transcript.words[0..2],
            ),
        },
        ReviewedTranscriptSpan {
            role: ReviewedTranscriptRole::Example,
            source: ReviewedTranscriptSource::ResolvedWords(
                &transcript.words[1..3],
            ),
        },
        ReviewedTranscriptSpan {
            role: ReviewedTranscriptRole::Section,
            source: ReviewedTranscriptSource::ResolvedWords(
                &transcript.words[0..2],
            ),
        },
    ];
    let reviewed = review_transcript_structure(&transcript, spans)
        .expect("overlap and repeated review remain caller-owned");
    assert_eq!(
        reviewed.locations(),
        [
            ReviewedTranscriptSourceLocation::ResolvedWords {
                end_word: 2,
                start_word: 0,
            },
            ReviewedTranscriptSourceLocation::ResolvedWords {
                end_word: 3,
                start_word: 1,
            },
            ReviewedTranscriptSourceLocation::ResolvedWords {
                end_word: 2,
                start_word: 0,
            },
        ],
    );
}

#[test]
fn reviewed_sources_must_borrow_from_the_supplied_transcript() {
    let transcript = transcript_fixture();
    let foreign = transcript_fixture();
    let foreign_words = vec![ReviewedTranscriptSpan {
        role: ReviewedTranscriptRole::Section,
        source: ReviewedTranscriptSource::ResolvedWords(&foreign.words[0..1]),
    }];
    assert_eq!(
        review_transcript_structure(&transcript, foreign_words),
        Err(TranscriptStructureError::ForeignResolvedWords { span_index: 0 }),
    );

    let foreign_unresolved = vec![ReviewedTranscriptSpan {
        role: ReviewedTranscriptRole::Unresolved,
        source: ReviewedTranscriptSource::UnresolvedFragment(
            &foreign.unresolved_fragments[0],
        ),
    }];
    assert_eq!(
        review_transcript_structure(&transcript, foreign_unresolved),
        Err(TranscriptStructureError::ForeignUnresolvedFragment {
            span_index: 0,
        }),
    );
}

#[test]
fn empty_resolved_words_cannot_claim_source_provenance() {
    let transcript = transcript_fixture();
    let spans = vec![ReviewedTranscriptSpan {
        role: ReviewedTranscriptRole::Section,
        source: ReviewedTranscriptSource::ResolvedWords(
            &transcript.words[1..1],
        ),
    }];
    assert_eq!(
        review_transcript_structure(&transcript, spans),
        Err(TranscriptStructureError::UnverifiableSourceEvidence {
            span_index: 0,
        }),
    );

    let empty: Transcript = TranscriptEvidence {
        origin: TranscriptOrigin {
            engine_identity: "engine/model-a",
            job_identity: "job-empty",
            media_identity: "lecture-video-empty",
            media_kind: TranscriptMediaKind::Video,
        },
        unresolved_fragments: vec![],
        words: vec![],
    };
    let empty_span = vec![ReviewedTranscriptSpan {
        role: ReviewedTranscriptRole::Section,
        source: ReviewedTranscriptSource::ResolvedWords(&empty.words[..]),
    }];
    assert_eq!(
        review_transcript_structure(&empty, empty_span),
        Err(TranscriptStructureError::UnverifiableSourceEvidence {
            span_index: 0,
        }),
    );
}

#[test]
fn zero_sized_evidence_cannot_claim_source_provenance() {
    let transcript = TranscriptEvidence {
        origin: TranscriptOrigin {
            engine_identity: "engine",
            job_identity: "job",
            media_identity: "media",
            media_kind: TranscriptMediaKind::Audio,
        },
        unresolved_fragments: vec![()],
        words: vec![()],
    };
    let spans = vec![ReviewedTranscriptSpan {
        role: ReviewedTranscriptRole::Section,
        source: ReviewedTranscriptSource::ResolvedWords(&transcript.words[..]),
    }];
    assert_eq!(
        review_transcript_structure(&transcript, spans),
        Err(TranscriptStructureError::UnverifiableSourceEvidence {
            span_index: 0,
        }),
    );
    let unresolved = vec![ReviewedTranscriptSpan {
        role: ReviewedTranscriptRole::Unresolved,
        source: ReviewedTranscriptSource::UnresolvedFragment(
            &transcript.unresolved_fragments[0],
        ),
    }];
    assert_eq!(
        review_transcript_structure(&transcript, unresolved),
        Err(TranscriptStructureError::UnverifiableSourceEvidence {
            span_index: 0,
        }),
    );
}

#[test]
fn explicit_unresolved_fragment_cannot_be_promoted_silently() {
    let transcript = transcript_fixture();
    let spans = vec![
        ReviewedTranscriptSpan {
            role: ReviewedTranscriptRole::Section,
            source: ReviewedTranscriptSource::ResolvedWords(
                &transcript.words[0..1],
            ),
        },
        ReviewedTranscriptSpan {
            role: ReviewedTranscriptRole::Formula,
            source: ReviewedTranscriptSource::UnresolvedFragment(
                &transcript.unresolved_fragments[0],
            ),
        },
    ];
    assert_eq!(
        review_transcript_structure(&transcript, spans),
        Err(TranscriptStructureError::UnresolvedFragmentPromotion {
            span_index: 1,
        }),
    );
}

#[test]
fn resolved_words_may_remain_semantically_unresolved_after_review() {
    let transcript = transcript_fixture();
    let spans = vec![ReviewedTranscriptSpan {
        role: ReviewedTranscriptRole::Unresolved,
        source: ReviewedTranscriptSource::ResolvedWords(
            &transcript.words[2..3],
        ),
    }];
    let reviewed = review_transcript_structure(&transcript, spans)
        .expect("review may retain semantic uncertainty");
    assert_eq!(reviewed.spans()[0].role, ReviewedTranscriptRole::Unresolved);
}
