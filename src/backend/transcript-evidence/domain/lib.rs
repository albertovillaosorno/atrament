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
//   - Transport-neutral derived transcript evidence and uncertainty retention.
// - Must-Not:
//   - Decode media, invoke transcription engines, choose confidence thresholds,
//     infer speakers, normalize text, write temporary files, or mutate notebook
//     semantics.
// - Allows:
//   - Inputs: Original media/job/engine identities plus caller-owned transcript
//     timing, confidence, speaker, and unresolved-fragment evidence.
//   - Outputs: One inspectable derived transcript distinct from original media.
//   - Side effects: None.
// - Split-When:
//   - Media-job lifecycle or transcript-to-semantic structuring gains
//     executable
//     authority.
// - Merge-When:
//   - Transcript evidence becomes inseparable from one ingestion adapter.
// - Summary:
//   - Preserves transcription uncertainty without binding semantics to
//     WhisperX.
// - Description:
//   - Retains optional word evidence and explicit unresolved fragments.
// - Usage:
//   - Carry reviewed transcription output across replaceable engine adapters.
// - Defaults:
//   - Missing timing, confidence, or speaker evidence remains explicitly
//     absent.
//

//! Transcript evidence independent of media decoding and transcription engines.

/// Original media family from which a transcript was derived.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum TranscriptMediaKind {
    /// Audio source media.
    Audio,
    /// Video source media whose audio was transcribed.
    Video,
}

/// One resolved transcript word with optional engine evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TranscriptWord<Confidence, Speaker, Text, TimeRange> {
    /// Optional caller-owned confidence evidence when the engine supplies it.
    pub confidence: Option<Confidence>,
    /// Optional speaker identity or label when available.
    pub speaker: Option<Speaker>,
    /// Resolved word text retained exactly as provided by the adapter boundary.
    pub text: Text,
    /// Optional word-level time range when available.
    pub time_range: Option<TimeRange>,
}

/// One unresolved transcript fragment retained for explicit review.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnresolvedTranscriptFragment<Confidence, Text, TimeRange> {
    /// Optional confidence evidence associated with the unresolved fragment.
    pub confidence: Option<Confidence>,
    /// Unresolved text or adapter-owned display evidence retained as data.
    pub text: Text,
    /// Optional source-media time range associated with the fragment.
    pub time_range: Option<TimeRange>,
}

/// Typed origin facts retained for one derived transcript.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TranscriptOrigin<EngineIdentity, JobIdentity, MediaIdentity> {
    /// Replaceable transcription engine/model identity or version.
    pub engine_identity: EngineIdentity,
    /// Unique ingestion/transcription job identity.
    pub job_identity: JobIdentity,
    /// Original source-media identity, never transcript text masquerading as
    /// it.
    pub media_identity: MediaIdentity,
    /// Whether the original source was audio or video.
    pub media_kind: TranscriptMediaKind,
}

/// Derived transcript evidence linked to original media and one ingestion job.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TranscriptEvidence<Origin, Word, UnresolvedFragment> {
    /// Complete typed media/job/engine origin retained without
    /// reinterpretation.
    pub origin: Origin,
    /// Explicit unresolved fragments requiring review.
    pub unresolved_fragments: Vec<UnresolvedFragment>,
    /// Resolved words in caller-owned transcript order.
    pub words: Vec<Word>,
}
