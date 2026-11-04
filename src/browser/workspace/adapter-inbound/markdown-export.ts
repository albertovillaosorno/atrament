// Copyright:
//   - Copyright © 2026 Alberto Villa Osorno.
// SPDX-License-Identifier:
//   - LicenseRef-Proprietary
// Confidential:
//   - true
// License-File:
//   - LICENSE
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
    | { kind: "empty" | "too-large" | "invalid-unicode"
        | "invalid-controls" }
    | { kind: "ready"; text: string; bytes: number };

// This is an early transport guard, not an admission of Markdown syntax.
// Rust --flow-markdown-file independently validates all file content.
export function prepareMarkdownHandoff(text: string): MarkdownHandoff {
    // Bound pasted source before scanning for content. The original UTF-8
    // byte cap includes the BOM, even for structural-only whitespace.
    if (text.length > 4_096) return { kind: "too-large" };
    const utf8 = new TextEncoder().encode(text);
    if (utf8.length > 4_096) return { kind: "too-large" };
    // Match Rust's structural blank check, not JS trim(): non-ASCII
    // spacing such as NBSP/thin space can be exact authored ink glyphs.
    // A leading UTF-8 BOM is transport metadata, never visible text.
    const admitted = text.startsWith("\uFEFF") ? text.slice(1) : text;
    if (!/[^ \t\n\r\u2028\u2029]/u.test(admitted)) {
        return { kind: "empty" };
    }
    // Rust flow accepts LF/CRLF, tabs and the pasted Unicode separators;
    // nonstructural C0/C1 controls and lone CR cannot be rendered. Catch
    // those before offering a downloaded file, without parsing Markdown.
    if (/[\u0000-\u0008\u000B\u000C\u000E-\u001F\u007F-\u009F]/u
        .test(admitted) || admitted.replace(/\r\n/g, "").includes("\r")) {
        return { kind: "invalid-controls" };
    }
    // Ignore the decoder's default BOM stripping for exact-source replay.
    if (new TextDecoder("utf-8", { ignoreBOM: true })
        .decode(utf8) !== text) {
        return { kind: "invalid-unicode" };
    }
    return { kind: "ready", text, bytes: utf8.length };
}
