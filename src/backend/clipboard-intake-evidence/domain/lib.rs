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
//   - Typed rich-clipboard intake evidence before semantic acceptance.
// - Must-Not:
//   - Read the system clipboard, parse MIME, decode images, choose metadata
//     vocabularies, mutate notebooks, or persist source bytes.
// - Allows:
//   - Inputs: Caller-owned typed content, source provenance, unresolved
//     fragments, and exact retention-issue details.
//   - Outputs: One inspectable intake value with first-release content kind.
//   - Side effects: None.
// - Split-When:
//   - Clipboard parsing, image decoding, or semantic candidate construction
//     gains executable authority.
// - Merge-When:
//   - Intake evidence becomes inseparable from one ingestion adapter.
// - Summary:
//   - Keeps clipboard source fidelity explicit before notebook mutation.
// - Description:
//   - Freezes first-release kinds and typed metadata/structure loss evidence.
// - Usage:
//   - Carry one adapter-decoded clipboard fragment into later review.
// - Defaults:
//   - No unreported metadata or structure loss is inferred as acceptable.
//

//! Rich clipboard intake evidence independent of clipboard and media adapters.

/// First-release rich clipboard content families.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ClipboardContentKind {
    /// Structured mathematical source or formula fragment.
    Formula,
    /// JPEG image content.
    JpegImage,
    /// PNG image content.
    PngImage,
    /// Structured table fragment.
    StructuredTable,
    /// Plain or authored Unicode text.
    Text,
    /// WebP image content.
    WebpImage,
}

/// Exact category of source information that cannot be retained.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ClipboardRetentionIssueKind {
    /// Source metadata cannot be retained exactly.
    Metadata,
    /// Source structure cannot be retained exactly.
    Structure,
}

/// One exact source-retention issue reported by the ingestion adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClipboardRetentionIssue<Detail> {
    /// Caller-owned exact description or typed identity of the lost detail.
    pub detail: Detail,
    /// Metadata or structure loss category.
    pub kind: ClipboardRetentionIssueKind,
}

/// One typed rich-clipboard fragment before semantic notebook acceptance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClipboardIntakeEvidence<
    Content, Provenance, RetentionDetail, Unresolved,
> {
    /// Caller-owned decoded or structured content.
    pub content: Content,
    /// First-release clipboard content family.
    pub kind: ClipboardContentKind,
    /// Caller-owned source provenance retained separately from content.
    pub provenance: Provenance,
    /// Exact metadata or structure details that could not be retained.
    pub retention_issues: Vec<ClipboardRetentionIssue<RetentionDetail>>,
    /// Caller-owned unresolved fragments preserved instead of guessed away.
    pub unresolved_fragments: Vec<Unresolved>,
}
