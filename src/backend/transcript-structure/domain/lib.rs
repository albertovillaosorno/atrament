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
//   - Reviewed transcript-span roles that retain original transcript evidence.
// - Must-Not:
//   - Segment transcripts, choose roles, interpret confidence, infer speakers,
//     normalize text, resolve uncertain fragments, mutate notebook semantics,
//     or perform media/transcription work.
// - Allows:
//   - Inputs: One derived transcript plus caller-reviewed resolved or
//     unresolved span evidence and semantic role choices.
//   - Outputs: Ordered reviewed spans that remain linked to the full
//     transcript.
//   - Side effects: None.
// - Split-When:
//   - Transcript correction or semantic notebook construction gains executable
//     authority.
// - Merge-When:
//   - Reviewed transcript structure becomes inseparable from semantic
//     ingestion.
// - Summary:
//   - Adds reviewed structure without hiding transcription uncertainty.
// - Description:
//   - Retains source evidence while classifying reviewed transcript spans.
// - Usage:
//   - Review transcript evidence before a later semantic notebook projection.
// - Defaults:
//   - Explicit unresolved fragments cannot become confident roles implicitly.
//

//! Reviewed transcript structure without parsing or notebook mutation
//! authority.

use std::ptr::eq as ptr_eq;

use atrament_transcript_evidence::TranscriptEvidence;

/// Caller-reviewed role for one transcript span.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ReviewedTranscriptRole {
    /// A reviewed definition span.
    Definition,
    /// A reviewed explanatory or worked-example span.
    Example,
    /// A reviewed mathematical formula span.
    Formula,
    /// A reviewed section or section-heading span.
    Section,
    /// Content that deliberately remains unresolved after review.
    Unresolved,
}

/// Original transcript evidence used by one reviewed span.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReviewedTranscriptSource<'transcript, Word, UnresolvedFragment> {
    /// One caller-selected contiguous slice of resolved transcript words.
    ResolvedWords(&'transcript [Word]),
    /// One explicit unresolved fragment from the source transcript.
    UnresolvedFragment(&'transcript UnresolvedFragment),
}

/// Address of reviewed evidence within the exact supplied transcript.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReviewedTranscriptSourceLocation {
    /// Half-open resolved-word range in source transcript order.
    ResolvedWords {
        /// Exclusive source-word index after the reviewed span.
        end_word: usize,
        /// Inclusive source-word index where the reviewed span begins.
        start_word: usize,
    },
    /// Exact unresolved-fragment index in source transcript order.
    UnresolvedFragment {
        /// Zero-based source unresolved-fragment index.
        fragment_index: usize,
    },
}

/// One reviewed transcript span and its retained source evidence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReviewedTranscriptSpan<'transcript, Word, UnresolvedFragment> {
    /// Caller-reviewed semantic role; this does not construct a notebook block.
    pub role: ReviewedTranscriptRole,
    /// Borrowed source evidence retained without rewriting.
    pub source: ReviewedTranscriptSource<'transcript, Word, UnresolvedFragment>,
}

/// Complete reviewed structure linked to the original derived transcript.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReviewedTranscriptStructure<
    'transcript,
    Origin,
    Word,
    UnresolvedFragment,
> {
    /// Exact source location aligned one-to-one with each reviewed span.
    locations: Vec<ReviewedTranscriptSourceLocation>,
    /// Ordered caller-reviewed spans.
    spans: Vec<ReviewedTranscriptSpan<'transcript, Word, UnresolvedFragment>>,
    /// Complete source transcript, including its provenance and uncertainty.
    transcript:
        &'transcript TranscriptEvidence<Origin, Word, UnresolvedFragment>,
}

impl<'transcript, Origin, Word, UnresolvedFragment>
    ReviewedTranscriptStructure<'transcript, Origin, Word, UnresolvedFragment>
{
    /// Return exact source locations aligned one-to-one with reviewed spans.
    #[must_use]
    pub fn locations(&self) -> &[ReviewedTranscriptSourceLocation] {
        &self.locations
    }

    /// Return caller-reviewed spans in admitted review order.
    #[must_use]
    pub fn spans(
        &self,
    ) -> &[ReviewedTranscriptSpan<'transcript, Word, UnresolvedFragment>] {
        &self.spans
    }

    /// Return the complete source transcript retained by this review.
    #[must_use]
    pub const fn transcript(
        &self,
    ) -> &'transcript TranscriptEvidence<Origin, Word, UnresolvedFragment> {
        self.transcript
    }
}

/// Fail-closed reviewed-structure validation error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TranscriptStructureError {
    /// A resolved-word slice does not borrow from the supplied transcript.
    ForeignResolvedWords {
        /// Zero-based reviewed-span index carrying foreign word evidence.
        span_index: usize,
    },
    /// An unresolved fragment does not borrow from the supplied transcript.
    ForeignUnresolvedFragment {
        /// Zero-based reviewed-span index carrying foreign fragment evidence.
        span_index: usize,
    },
    /// An explicit unresolved source fragment was assigned a confident role.
    UnresolvedFragmentPromotion {
        /// Zero-based reviewed-span index that attempted the promotion.
        span_index: usize,
    },
    /// Source provenance is indistinguishable for zero-sized evidence.
    UnverifiableSourceEvidence {
        /// Zero-based reviewed-span index carrying unverifiable evidence.
        span_index: usize,
    },
}

/// Result of validating one reviewed transcript structure.
pub type TranscriptStructureResult<
    'transcript,
    Origin,
    Word,
    UnresolvedFragment,
> =
    Result<
        ReviewedTranscriptStructure<
            'transcript,
            Origin,
            Word,
            UnresolvedFragment,
        >,
        TranscriptStructureError,
    >;

/// Validate reviewed roles while preserving the complete source transcript.
///
/// # Errors
///
/// Returns a typed source-membership failure when one reviewed span borrows
/// evidence from another transcript. Explicit unresolved source fragments
/// also reject when assigned any role other than
/// [`ReviewedTranscriptRole::Unresolved`].
pub fn review_transcript_structure<
    'transcript,
    Origin,
    Word,
    UnresolvedFragment,
>(
    transcript:
        &'transcript TranscriptEvidence<Origin, Word, UnresolvedFragment>,
    spans: Vec<ReviewedTranscriptSpan<'transcript, Word, UnresolvedFragment>>,
) -> TranscriptStructureResult<
    'transcript,
    Origin,
    Word,
    UnresolvedFragment,
> {
    let mut locations = Vec::with_capacity(spans.len());
    for (span_index, span) in spans.iter().enumerate() {
        let location = match span.source {
            ReviewedTranscriptSource::ResolvedWords(words) => {
                if words.is_empty() || size_of::<Word>() == 0 {
                    return Err(
                        TranscriptStructureError::UnverifiableSourceEvidence {
                            span_index,
                        },
                    );
                }
                let Some((start_word, end_word)) = resolved_word_range(
                    &transcript.words,
                    words,
                ) else {
                    return Err(TranscriptStructureError::ForeignResolvedWords {
                        span_index,
                    });
                };
                ReviewedTranscriptSourceLocation::ResolvedWords {
                    end_word,
                    start_word,
                }
            },
            ReviewedTranscriptSource::UnresolvedFragment(fragment) => {
                if size_of::<UnresolvedFragment>() == 0 {
                    return Err(
                        TranscriptStructureError::UnverifiableSourceEvidence {
                            span_index,
                        },
                    );
                }
                let Some(fragment_index) = transcript
                    .unresolved_fragments
                    .iter()
                    .position(|candidate| ptr_eq(candidate, fragment))
                else {
                    return Err(
                        TranscriptStructureError::ForeignUnresolvedFragment {
                            span_index,
                        },
                    );
                };
                if span.role != ReviewedTranscriptRole::Unresolved {
                    return Err(
                        TranscriptStructureError::UnresolvedFragmentPromotion {
                            span_index,
                        },
                    );
                }
                ReviewedTranscriptSourceLocation::UnresolvedFragment {
                    fragment_index,
                }
            },
        };
        locations.push(location);
    }

    Ok(ReviewedTranscriptStructure {
        locations,
        spans,
        transcript,
    })
}

fn resolved_word_range<Word>(
    transcript_words: &[Word],
    reviewed_words: &[Word],
) -> Option<(usize, usize)> {
    if reviewed_words.len() > transcript_words.len() {
        return None;
    }
    transcript_words
        .windows(reviewed_words.len())
        .position(|candidate| ptr_eq(candidate, reviewed_words))
        .map(|start_word| {
            (start_word, start_word.saturating_add(reviewed_words.len()))
        })
}
