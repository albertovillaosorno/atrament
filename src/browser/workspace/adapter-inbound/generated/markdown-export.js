// Native flow expands tabs to physical four-grapheme columns after
// normalizing source line separators. This read-only estimate only blocks
// known size overflow; unsupported segmenters defer to the Rust CLI.
function expandsBeyondNativeFlowLimit(text, encoder) {
    if (!text.includes("\t") || typeof Intl.Segmenter !== "function") {
        return false;
    }
    const normalized = text.replace(/\r\n/g, "\n")
        .replace(/\u2028/g, "  \n").replace(/\u2029/g, "\n\n");
    let expandedBytes = encoder.encode(normalized).byteLength;
    let column = 0;
    const segments = new Intl.Segmenter(undefined, {
        granularity: "grapheme",
    });
    for (const { segment } of segments.segment(normalized)) {
        if (segment === "\n") {
            column = 0;
        }
        else if (segment === "\t") {
            const spaces = 4 - column % 4;
            expandedBytes += spaces - 1;
            column += spaces;
        }
        else {
            column += 1;
        }
        if (expandedBytes > 4_096)
            return true;
    }
    return false;
}
// This is an early transport guard, not an admission of Markdown syntax.
// Rust --flow-markdown-file independently validates all file content.
export function prepareMarkdownHandoff(text) {
    // Bound pasted source before scanning for content. The original UTF-8
    // byte cap includes the BOM, even for structural-only whitespace.
    if (text.length > 4_096)
        return { kind: "too-large" };
    const encoder = new TextEncoder();
    const utf8 = encoder.encode(text);
    if (utf8.length > 4_096)
        return { kind: "too-large" };
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
    if (expandsBeyondNativeFlowLimit(admitted, encoder)) {
        return { kind: "too-large" };
    }
    return { kind: "ready", text, bytes: utf8.length };
}
