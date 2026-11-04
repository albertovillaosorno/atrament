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
//   - Focused exact-text/browser CLI Markdown handoff regressions.
// - Must-Not:
//   - Invent a browser Markdown parser or claim full notebook admission.
// - Allows:
//   - Inputs: Local source Unicode and the Rust CLI's byte budget.
//   - Outputs: Byte-identity, safe refusal and generated UI wiring checks.
//   - Side effects: Read-only repository source inspection.
// - Split-When:
//   - A full real localhost Markdown document integration is available.
// - Merge-When:
//   - Native browser-to-renderer transfer supersedes file handoff.
// - Summary:
//   - Verifies explicit, bounded source download without normalization.
// - Description:
//   - Only Rust may validate the content and render real notebook pages.
// - Usage:
//   - node --test tests/browser/workspace/adapter-inbound/markdown-export.mjs
// - Defaults:
//   - Read-only tests; no local files, implicit writes or network access.
//
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const ROOT = "../../../../src/browser/workspace/adapter-inbound/";
const { prepareMarkdownHandoff } =
    await import(`${ROOT}generated/markdown-export.js`);

test("source Markdown handoff retains exact UTF-8 input without parsing",
    () => {
    const inputs = [
        "# Título\n\n- ñ\n1. café\n",
        "# n\u0303\r\ntexto  \r\nénfasis",
        "\uFEFF# title\n- line\n",
        " a  a  \n",
        "a\n# This is just source\n",
    ];
    for (const input of inputs) {
        const candidate = prepareMarkdownHandoff(input);
        assert.equal(candidate.kind, "ready", input);
        assert.equal(candidate.text, input);
        assert.equal(candidate.bytes,
            new TextEncoder().encode(input).byteLength);
        assert.equal(new TextDecoder("utf-8", { ignoreBOM: true })
            .decode(new TextEncoder().encode(candidate.text)), input);
    }
});

test("CLI file transport accepts exactly 4096 bytes", () => {
    const accepted = prepareMarkdownHandoff("á".repeat(2048));
    assert.deepEqual(accepted, {
        kind: "ready", text: "á".repeat(2048), bytes: 4096,
    });
    assert.equal(prepareMarkdownHandoff("á".repeat(2049)).kind,
        "too-large");
    assert.equal(prepareMarkdownHandoff("a".repeat(4096)).kind,
        "ready");
    assert.equal(prepareMarkdownHandoff("a".repeat(4097)).kind,
        "too-large");
    assert.equal(prepareMarkdownHandoff("n\u0303".repeat(1365)).kind,
        "ready");
    assert.equal(prepareMarkdownHandoff("n\u0303".repeat(1366)).kind,
        "too-large");
});

test("blank or lossy Unicode handoff refuses without fallback", () => {
    for (const source of ["", "\n \t", "\uFEFF",
        "\uFEFF \t\n\r\u2028\u2029"]) {
        assert.deepEqual(prepareMarkdownHandoff(source), { kind: "empty" });
    }
    for (const source of ["a\ud800b", "\udc00", "\ud800\ud800"]) {
        assert.deepEqual(prepareMarkdownHandoff(source), {
            kind: "invalid-unicode",
        });
    }
});

test("browser refuses Markdown source controls native Rust cannot render",
    () => {
    for (const source of ["a\u0007b", "a\u0085b", "a\u000Bb",
        "a\u000Cb", "a\rb", "\ra", "a\r", "a\r\r\nb",
        "\uFEFFa\u0085b", "\u0000"]) {
        assert.deepEqual(prepareMarkdownHandoff(source), {
            kind: "invalid-controls",
        }, `source must be refused: ${JSON.stringify(source)}`);
    }
    for (const source of ["a\r\nb", "a\nb\tc", "a\u2028b",
        "a\u2029b"]) {
        assert.equal(prepareMarkdownHandoff(source).kind, "ready",
            `structural separator must remain accepted: ${source}`);
    }
});

test("raw UTF-8 byte limit takes precedence over structural blank scan",
    () => {
    for (const source of [" ".repeat(4097),
        "\uFEFF" + " ".repeat(4094),
        "\u2028".repeat(1366)]) {
        assert.deepEqual(prepareMarkdownHandoff(source), {
            kind: "too-large",
        });
    }
    for (const source of [" ".repeat(4096),
        "\uFEFF" + " ".repeat(4093),
        "\u2028".repeat(1365)]) {
        assert.deepEqual(prepareMarkdownHandoff(source), { kind: "empty" });
    }
});

test("Unicode spacing authored as ink remains eligible for native CLI",
    () => {
    // Rust only treats ASCII spacing and pasted line boundaries as
    // structure. Other exact space graphemes require authored ink.
    for (const source of ["\u00A0", "\u2003", "\u2009", "\u202F",
        "\uFEFF\u00A0", " \u2009\n"]) {
        const candidate = prepareMarkdownHandoff(source);
        assert.deepEqual(candidate, {
            kind: "ready", text: source,
            bytes: new TextEncoder().encode(source).byteLength,
        }, `preserve exact Unicode spacing ${JSON.stringify(source)}`);
    }
});

test("workspace handoff is explicit and disabled before authentication", () => {
    const html = readFileSync(new URL(`${ROOT}index.html`, import.meta.url),
        "utf8");
    const script = readFileSync(new URL(`${ROOT}generated/main.js`,
        import.meta.url), "utf8");
    assert.match(html,
        /id="source-markdown-export"[^>]*\bdisabled\b/s);
    assert.match(html, /Download source as Markdown/);
    assert.match(script, /sourceMarkdownExport.addEventListener\("click"/);
    assert.match(script, /sessionSecret === null/);
    assert.match(script, /revokeSourceMarkdownUrl\(\)/);
    assert.match(script, /Source contains unsupported controls or line breaks/);
    assert.match(script, /link.download = "atrament-source.md"/);
    assert.doesNotMatch(script, /\.\/api\/notebook\/markdown/);
});
