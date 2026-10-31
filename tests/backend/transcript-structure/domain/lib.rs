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

//! Transcript structure domain regression tests.

use atrament_transcript_evidence::{
    TranscriptEvidence, TranscriptMediaKind, TranscriptOrigin, TranscriptWord,
    UnresolvedTranscriptFragment,
};
use atrament_transcript_structure::{
    ReviewedTranscriptRole, ReviewedTranscriptSource,
    ReviewedTranscriptSourceLocation, ReviewedTranscriptSpan,
    TranscriptStructureError, review_transcript_structure,
};

// Explicitly assert all fixed fixture spans rather than indexing unchecked.
macro_rules! require_some {
    ($expression:expr) => {{
        let selected = $expression;
        assert!(selected.is_some(), "expected fixture element or span");
        let Some(value) = selected else {
            return;
        };
        value
    }};
}

macro_rules! require_ok {
    ($expression:expr) => {{
        let outcome = $expression;
        assert!(
            outcome.is_ok(),
            "expected admitted transcript review: {:?}",
            outcome.as_ref().err(),
        );
        let Ok(value) = outcome else {
            return;
        };
        value
    }};
}

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
            require_some!(transcript.words.get(0..1)),
        ),
    }];
    let reviewed = require_ok!(review_transcript_structure(&transcript, spans));
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
                require_some!(transcript.words.get(0..2)),
            ),
        },
        ReviewedTranscriptSpan {
            role: ReviewedTranscriptRole::Example,
            source: ReviewedTranscriptSource::ResolvedWords(
                require_some!(transcript.words.get(1..2)),
            ),
        },
        ReviewedTranscriptSpan {
            role: ReviewedTranscriptRole::Formula,
            source: ReviewedTranscriptSource::ResolvedWords(
                require_some!(transcript.words.get(2..3)),
            ),
        },
    ];
    let reviewed = require_ok!(review_transcript_structure(&transcript, spans));
    let definition_source = &require_some!(reviewed.spans().first()).source;
    assert!(
        matches!(definition_source, ReviewedTranscriptSource::ResolvedWords(_)),
        "definition source must remain resolved words",
    );
    let ReviewedTranscriptSource::ResolvedWords(definition) = definition_source
    else {
        return;
    };
    assert_eq!(require_some!(definition.first()).confidence, Some(96));
    assert_eq!(require_some!(definition.first()).speaker, Some("teacher"));
    assert_eq!(
        require_some!(definition.first()).time_range,
        Some((1_000, 1_220)),
    );
    assert_eq!(require_some!(definition.first()).text, "Definition");
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
    let formula_source = &require_some!(reviewed.spans().get(2)).source;
    assert!(
        matches!(formula_source, ReviewedTranscriptSource::ResolvedWords(_)),
        "formula source must remain resolved words",
    );
    let ReviewedTranscriptSource::ResolvedWords(formula) = formula_source else {
        return;
    };
    assert_eq!(require_some!(formula.first()).confidence, None);
    assert_eq!(require_some!(formula.first()).speaker, None);
    assert_eq!(require_some!(formula.first()).text, "v=dx/dt");
}

#[test]
fn unresolved_fragment_keeps_text_timing_and_confidence() {
    let transcript = transcript_fixture();
    let spans = vec![ReviewedTranscriptSpan {
        role: ReviewedTranscriptRole::Unresolved,
        source: ReviewedTranscriptSource::UnresolvedFragment(
            require_some!(transcript.unresolved_fragments.first()),
        ),
    }];
    let reviewed = require_ok!(review_transcript_structure(&transcript, spans));
    let unresolved_source = &require_some!(reviewed.spans().first()).source;
    assert!(
        matches!(
            unresolved_source,
            ReviewedTranscriptSource::UnresolvedFragment(_),
        ),
        "unresolved source must remain an unresolved fragment",
    );
    let ReviewedTranscriptSource::UnresolvedFragment(fragment) =
        unresolved_source
    else {
        return;
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
                require_some!(transcript.words.get(0..2)),
            ),
        },
        ReviewedTranscriptSpan {
            role: ReviewedTranscriptRole::Example,
            source: ReviewedTranscriptSource::ResolvedWords(
                require_some!(transcript.words.get(1..3)),
            ),
        },
        ReviewedTranscriptSpan {
            role: ReviewedTranscriptRole::Section,
            source: ReviewedTranscriptSource::ResolvedWords(
                require_some!(transcript.words.get(0..2)),
            ),
        },
    ];
    let reviewed = require_ok!(review_transcript_structure(&transcript, spans));
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
        source: ReviewedTranscriptSource::ResolvedWords(
            require_some!(foreign.words.get(0..1)),
        ),
    }];
    assert_eq!(
        review_transcript_structure(&transcript, foreign_words),
        Err(TranscriptStructureError::ForeignResolvedWords { span_index: 0 }),
    );

    let foreign_unresolved = vec![ReviewedTranscriptSpan {
        role: ReviewedTranscriptRole::Unresolved,
        source: ReviewedTranscriptSource::UnresolvedFragment(
            require_some!(foreign.unresolved_fragments.first()),
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
            require_some!(transcript.words.get(1..1)),
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
        source: ReviewedTranscriptSource::ResolvedWords(empty.words.as_slice()),
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
        source: ReviewedTranscriptSource::ResolvedWords(
            transcript.words.as_slice(),
        ),
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
            require_some!(transcript.unresolved_fragments.first()),
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
                require_some!(transcript.words.get(0..1)),
            ),
        },
        ReviewedTranscriptSpan {
            role: ReviewedTranscriptRole::Formula,
            source: ReviewedTranscriptSource::UnresolvedFragment(
                require_some!(transcript.unresolved_fragments.first()),
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
            require_some!(transcript.words.get(2..3)),
        ),
    }];
    let reviewed = require_ok!(review_transcript_structure(&transcript, spans));
    assert_eq!(
        require_some!(reviewed.spans().first()).role,
        ReviewedTranscriptRole::Unresolved,
    );
}

#[test]
fn every_review_role_matches_resolved_and_unresolved_source_oracle() {
    let roles = [
        ReviewedTranscriptRole::Definition,
        ReviewedTranscriptRole::Example,
        ReviewedTranscriptRole::Formula,
        ReviewedTranscriptRole::Section,
        ReviewedTranscriptRole::Unresolved,
    ];
    let transcript = transcript_fixture();
    let mut cases = 0u8;
    for role in roles {
        let resolved = vec![ReviewedTranscriptSpan {
            role,
            source: ReviewedTranscriptSource::ResolvedWords(
                require_some!(transcript.words.get(0..1)),
            ),
        }];
        let reviewed = require_ok!(review_transcript_structure(
            &transcript, resolved
        ));
        assert_eq!(require_some!(reviewed.spans().first()).role, role);
        assert_eq!(
            reviewed.locations(),
            [ReviewedTranscriptSourceLocation::ResolvedWords {
                end_word: 1,
                start_word: 0,
            }],
        );
        cases = cases.saturating_add(1);

        let unresolved = vec![ReviewedTranscriptSpan {
            role,
            source: ReviewedTranscriptSource::UnresolvedFragment(
                require_some!(transcript.unresolved_fragments.first()),
            ),
        }];
        if role == ReviewedTranscriptRole::Unresolved {
            let reviewed_unresolved = require_ok!(review_transcript_structure(
                &transcript, unresolved
            ));
            assert_eq!(
                require_some!(reviewed_unresolved.spans().first()).role, role
            );
            assert_eq!(
                reviewed_unresolved.locations(),
                [ReviewedTranscriptSourceLocation::UnresolvedFragment {
                    fragment_index: 0,
                }],
            );
        } else {
            assert_eq!(
                review_transcript_structure(&transcript, unresolved),
                Err(TranscriptStructureError::UnresolvedFragmentPromotion {
                    span_index: 0,
                }),
                "role {role:?} must not promote unresolved source evidence",
            );
        }
        cases = cases.saturating_add(1);
    }
    assert_eq!(cases, 10);
}
