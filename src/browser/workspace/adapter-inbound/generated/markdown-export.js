// This is an early transport guard, not an admission of Markdown syntax.
// Rust --flow-markdown-file independently validates all file content.
export function prepareMarkdownHandoff(text) {
    if (!text.trim())
        return { kind: "empty" };
    if (text.length > 4_096)
        return { kind: "too-large" };
    const utf8 = new TextEncoder().encode(text);
    if (utf8.length > 4_096)
        return { kind: "too-large" };
    // Ignore the decoder's default BOM stripping for exact-source replay.
    if (new TextDecoder("utf-8", { ignoreBOM: true })
        .decode(utf8) !== text) {
        return { kind: "invalid-unicode" };
    }
    return { kind: "ready", text, bytes: utf8.length };
}
