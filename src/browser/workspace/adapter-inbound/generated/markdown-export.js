// This is an early transport guard, not an admission of Markdown syntax.
// Rust --flow-markdown-file independently validates all file content.
export function prepareMarkdownHandoff(text) {
    // Bound pasted source before scanning for content. The original UTF-8
    // byte cap includes the BOM, even for structural-only whitespace.
    if (text.length > 4_096)
        return { kind: "too-large" };
    const utf8 = new TextEncoder().encode(text);
    if (utf8.length > 4_096)
        return { kind: "too-large" };
    // Match Rust's structural blank check, not JS trim(): non-ASCII
    // spacing such as NBSP/thin space can be exact authored ink glyphs.
    // A leading UTF-8 BOM is transport metadata, never visible text.
    const admitted = text.startsWith("\uFEFF") ? text.slice(1) : text;
    if (!/[^ \t\n\r\u2028\u2029]/u.test(admitted)) {
        return { kind: "empty" };
    }
    // Ignore the decoder's default BOM stripping for exact-source replay.
    if (new TextDecoder("utf-8", { ignoreBOM: true })
        .decode(utf8) !== text) {
        return { kind: "invalid-unicode" };
    }
    return { kind: "ready", text, bytes: utf8.length };
}
