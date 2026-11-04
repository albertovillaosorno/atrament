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
//   - Read-only browser eligibility for an explicit Markdown CLI handoff.
// - Must-Not:
//   - Parse Markdown, validate rendered content or persist notebook state.
// - Allows:
//   - Inputs: Current editable source text.
//   - Outputs: UTF-8 byte length or a bounded handoff refusal.
//   - Side effects: None.
// - Split-When:
//   - The Rust notebook API admits native Markdown source uploads.
// - Merge-When:
//   - Browser-to-Rust document intake replaces CLI file handoff.
// - Summary:
//   - Limits explicit .md download to the existing Rust CLI source budget.
// - Description:
//   - Rust remains the authority for glyph coverage, Markdown and layout.
// - Usage:
//   - Called only for the local source field's user-requested export.
// - Defaults:
//   - No silent source cleanup, normalization or file writes.
//
export type MarkdownHandoff =
    | { kind: "empty" | "too-large" | "invalid-unicode" }
    | { kind: "ready"; text: string; bytes: number };

// This is an early transport guard, not an admission of Markdown syntax.
// Rust --flow-markdown-file independently validates all file content.
export function prepareMarkdownHandoff(text: string): MarkdownHandoff {
    if (!text.trim()) return { kind: "empty" };
    if (text.length > 4_096) return { kind: "too-large" };
    const utf8 = new TextEncoder().encode(text);
    if (utf8.length > 4_096) return { kind: "too-large" };
    // Ignore the decoder's default BOM stripping for exact-source replay.
    if (new TextDecoder("utf-8", { ignoreBOM: true })
        .decode(utf8) !== text) {
        return { kind: "invalid-unicode" };
    }
    return { kind: "ready", text, bytes: utf8.length };
}
