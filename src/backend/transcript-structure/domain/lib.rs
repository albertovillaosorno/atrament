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
    EngineIdentity,
    JobIdentity,
    MediaIdentity,
    Word,
    UnresolvedFragment,
> {
    /// Ordered caller-reviewed spans.
    pub spans:
        Vec<ReviewedTranscriptSpan<'transcript, Word, UnresolvedFragment>>,
    /// Complete source transcript, including its provenance and uncertainty.
    pub transcript: &'transcript TranscriptEvidence<
        EngineIdentity,
        JobIdentity,
        MediaIdentity,
        Word,
        UnresolvedFragment,
    >,
}

/// Fail-closed reviewed-structure validation error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TranscriptStructureError {
    /// An explicit unresolved source fragment was assigned a confident role.
    UnresolvedFragmentPromotion {
        /// Zero-based reviewed-span index that attempted the promotion.
        span_index: usize,
    },
}

/// Validate reviewed roles while preserving the complete source transcript.
///
/// # Errors
///
/// Returns [`TranscriptStructureError::UnresolvedFragmentPromotion`] when an
/// explicit unresolved source fragment is assigned any role other than
/// [`ReviewedTranscriptRole::Unresolved`].
pub fn review_transcript_structure<
    'transcript,
    EngineIdentity,
    JobIdentity,
    MediaIdentity,
    Word,
    UnresolvedFragment,
>(
    transcript: &'transcript TranscriptEvidence<
        EngineIdentity,
        JobIdentity,
        MediaIdentity,
        Word,
        UnresolvedFragment,
    >,
    spans: Vec<ReviewedTranscriptSpan<'transcript, Word, UnresolvedFragment>>,
) -> Result<
    ReviewedTranscriptStructure<
        'transcript,
        EngineIdentity,
        JobIdentity,
        MediaIdentity,
        Word,
        UnresolvedFragment,
    >,
    TranscriptStructureError,
> {
    for (span_index, span) in spans.iter().enumerate() {
        if matches!(
            span.source,
            ReviewedTranscriptSource::UnresolvedFragment(_)
        ) && span.role != ReviewedTranscriptRole::Unresolved
        {
            return Err(
                TranscriptStructureError::UnresolvedFragmentPromotion {
                    span_index,
                },
            );
        }
    }

    Ok(ReviewedTranscriptStructure { spans, transcript })
}
