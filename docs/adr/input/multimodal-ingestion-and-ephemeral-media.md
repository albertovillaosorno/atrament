# Multimodal ingestion and ephemeral media

## Status

Accepted.

## Decision ID

`atrament.input.multimodal-ingestion-and-ephemeral-media`

## Context

Notebook material can arrive as typed text, clipboard fragments, photographs,
audio, or video. Speech alignment tools may require a normalized audio stream,
but intermediate media should not become an unexplained permanent copy of a
personal recording.

## Decision

Every ingestion adapter produces typed content plus source provenance,
diagnostics, and unresolved fragments. Audio and video ingestion may create a
bounded waveform intermediate, invoke WhisperX through a replaceable adapter,
and retain word-level timing and confidence where available.

Intermediate waveform files are created below owned temporary storage with a
unique job identity. They are removed after success, cancellation, or handled
failure; cleanup failure is surfaced and retried rather than hidden.

## Consequences

- Original media remains distinct from derived transcripts.
- Transcription engines can change without changing notebook semantics.
- Long operations need progress, cancellation, and cleanup recovery.
- The transcript records uncertainty instead of presenting every token as fact.

## Rejected Alternatives

- Permanent waveform conversion by default was rejected because it duplicates
  sensitive media without product value.
- Feeding transcripts directly into page drawing was rejected because users
  need an editable semantic review boundary.
- Binding the document model to WhisperX objects was rejected because model
  versions and providers evolve independently.

## Verification

Tests must cover audio, video, cancellation, decoder failure, transcription
failure, process interruption, and cleanup failure. No completed or failed job
may leave an unregistered waveform intermediate behind.
