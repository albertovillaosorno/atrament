// Copyright © 2026 Alberto Villa Osorno. SPDX-License-Identifier: MIT
// Browser authored Unicode stroke authoring and session-safe TSV contracts.
import assert from "node:assert/strict";
import test from "node:test";
const { parseGlyphLibrary, encodeGlyphLibrary, createGlyphStrokeEditor,
    geometricStarter, SELECTABLE_MATH_GLYPHS, italicizeGlyphStrokes,
    scaleGlyphHeight, scaleGlyphWidth,
    scaleGlyphInk, translateGlyphStrokes, rotateGlyphStrokes,
    reflectGlyphStrokes, subdivideGlyphStroke, splitGlyphStrokeExactly,
    mergeGlyphStrokes,
    composeAccentedGlyph, fillMissingBilingualBaseline,
    fillMissingConnectedLowercase,
    missingBilingualBaseline, varyGlyphStrokes,
    candidatePenPaths, copyCandidatePath, copyShiftedCandidatePath,
    reverseCandidatePath, moveCandidatePath,
    rotateClosedCandidatePath, closeCandidatePath, joinNextCandidatePath,
    joinNextCandidatePathCurved,
    translateCandidatePath, styleCandidatePath, scaleCandidatePath,
    reflectCandidatePath, rotateCandidatePath, shearCandidatePath,
    removeCandidatePath } =
    await import(new URL(
        "../../../../src/browser/workspace/adapter-inbound/generated/"
            + "glyph-strokes.js", import.meta.url,
    ).href);

const header = "atrament.glyph-strokes.v1\n";
const stroke = (grapheme = "ñ") => ({
    grapheme, x1: 100, y1: 200, x2: 3000, y2: 4200,
    width: 250, color: "#A1B2C3",
});

test("exact Unicode shape identity survives TSV", () => {
    const shapes = [stroke("ñ"), stroke("n\u{303}"), stroke("∞"), stroke("«")];
    const saved = encodeGlyphLibrary(shapes);
    assert.equal(saved.startsWith(header), true);
    assert.equal(saved.split("\n").length, 6);
    assert.deepEqual(parseGlyphLibrary(saved), shapes);
    assert.notEqual(shapes[0].grapheme, shapes[1].grapheme);
    const padded = header + "ñ\t0001\t2\t3000\t4200\t250\t#a1b2c3\n";
    assert.deepEqual(parseGlyphLibrary(padded), [{
        grapheme: "ñ", x1: 1, y1: 2, x2: 3000, y2: 4200,
        width: 250, color: "#A1B2C3",
    }]);
});

test("invalid UTF-16 glyphs cannot silently become replacement ink", () => {
    const validAstral = stroke("🖋");
    const bytes = new TextEncoder().encode(
        encodeGlyphLibrary([validAstral]),
    );
    assert.deepEqual(parseGlyphLibrary(new TextDecoder().decode(bytes)),
        [validAstral], "valid supplementary Unicode must roundtrip");
    for (const malformed of ["\uD800", "\uDC00", "\uD800a", "a\uDC00"]) {
        const bad = stroke(malformed);
        assert.throws(() => encodeGlyphLibrary([bad]),
            /Unicode|surrogate/, "invalid UTF-16 must refuse before download");
        const source = header
            + `${malformed}\t100\t200\t3000\t4200\t250\t#A1B2C3\n`;
        assert.throws(() => parseGlyphLibrary(source),
            /Unicode|surrogate/, "invalid UTF-16 cannot enter the draft");
    }
});

test("Unicode row separators cannot become authored browser TSV ink", () => {
    // Native --audit-text and flow treat these as row boundaries, not ink.
    for (const separator of ["\u2028", "\u2029"]) {
        assert.throws(() => encodeGlyphLibrary([stroke(separator)]),
            /visible Unicode grapheme/,
            "browser must refuse unrenderable separator before download");
        const raw = header
            + `${separator}\t100\t200\t3000\t4200\t250\t#A1B2C3\n`;
        assert.throws(() => parseGlyphLibrary(raw),
            /visible Unicode grapheme/,
            "browser must not admit a TSV rejected by Rust");
    }
    for (const authored of ["\u00A0", "\u2009", "\u202F"]) {
        assert.deepEqual(parseGlyphLibrary(
            encodeGlyphLibrary([stroke(authored)])), [stroke(authored)],
            "exact authored Unicode spaces remain independent glyphs");
    }
});

test("reserved Unicode and standalone nonmarking keys cannot be ink", () => {
    const invisible = ["\u00AD", "\u034F", "\u061C", "\u180E",
        "\u0600", "\u0605", "\u06DD", "\u070F",
        "\u0890", "\u0891", "\u08E2",
        "\u{110BD}", "\u{110CD}", "\u{13430}", "\u{1343F}",
        "\u{13440}", "\u{13441}", "\u{13442}",
        "\u{13447}", "\u{13455}",
        "\u200B", "\u200C", "\u200D", "\u200E", "\u200F",
        "\u202A", "\u202E", "\u2060", "\u2061", "\u2062",
        "\u2063", "\u2064", "\u2066", "\u2069", "\u206F",
        "\uFE00", "\uFE0E", "\uFE0F", "\uFEFF",
        "\u{E0100}", "\u{E01EF}", "\u115F", "\u1160",
        "\u17B4", "\u17B5", "\u180B", "\u180F", "\u3164",
        "\uFFA0", "\u2800", "\uFFF0", "\uFFF8", "\uFFF9",
        "\uFFFA", "\uFFFB", "\u{1BCA0}",
        "\u{1BCA3}", "\u{1D173}", "\u{1D17A}",
        "\u{E0001}", "\u{E0067}", "\u{E007F}",
        "\uFDD0", "\uFDEF", "\uFFFE", "\uFFFF",
        "\u{1FFFE}", "\u{1FFFF}", "\u{10FFFE}", "\u{10FFFF}",
        "\uFDD0\u0301", "\u{1FFFE}\u{E0100}",
        "\uFE0E\uFE0F", "\u200D\uFE0F", "\u2800\uFE0F",
        "\u0600\u200D", "\u034F\uFE0E",
        "\u{E0067}\u{E007F}"];
    for (const glyph of invisible) {
        assert.throws(() => encodeGlyphLibrary([stroke(glyph)]),
            /visible Unicode grapheme/,
            "nonmarking-only grapheme is not a visible glyph");
        const source = header
            + `${glyph}\t100\t200\t3000\t4200\t250\t#A1B2C3\n`;
        assert.throws(() => parseGlyphLibrary(source),
            /visible Unicode grapheme/,
            "native must refuse the same unrenderable exact key");
    }
    for (const visible of ["👩\u200D💻", "❤\uFE0F", "a\u034F",
        "\u0600\u0661", "\u06DD\u0661",
        "\u0890\u0661", "\u0891\u0661",
        "a\u{E0100}", "⠁", "\u2800\u0301",
        "\u{13000}\u{13440}", "\u{13000}\u{13447}",
        "\u{13443}", "\uE000", "\u{10FFFD}",
        "\uFFFD", "\uFDCF", "\uFDF0", "🏴\u{E0067}\u{E0062}\u{E0065}"
            + "\u{E006E}\u{E0067}\u{E007F}", "\u00A0"]) {
        const original = [stroke(visible)];
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(original)),
            original, "joiners and selectors inside real glyphs survive");
    }
});

test("every Unicode noncharacter is refused as browser TSV ink", () => {
    const reserved = Array.from({ length: 32 }, (_, n) =>
        String.fromCodePoint(0xFDD0 + n));
    for (let plane = 0; plane <= 16; plane += 1) {
        reserved.push(String.fromCodePoint(plane * 0x10000 + 0xFFFE));
        reserved.push(String.fromCodePoint(plane * 0x10000 + 0xFFFF));
    }
    assert.equal(reserved.length, 66);
    for (const key of reserved) {
        assert.throws(() => encodeGlyphLibrary([stroke(key)]),
            /visible Unicode grapheme/);
        const raw = header
            + `${key}\t100\t200\t3000\t4200\t250\t#A1B2C3\n`;
        assert.throws(() => parseGlyphLibrary(raw),
            /visible Unicode grapheme/);
    }
});

test("candidate pen paths respect directed endpoints and exact Unicode", () => {
    const nfd = "n\u{303}";
    const source = [
        { ...stroke(nfd), x1: 100, y1: 300, x2: 400, y2: 600 },
        { ...stroke("ñ"), x1: 100, y1: 300, x2: 400, y2: 600 },
        { ...stroke(nfd), x1: 400, y1: 600, x2: 800, y2: 900,
            color: "#a1b2c3" },
        { ...stroke(nfd), x1: 800, y1: 900, x2: 1200, y2: 1200,
            width: 350 },
        { ...stroke(nfd), x1: 1200, y1: 1200, x2: 1400, y2: 1600,
            width: 350 },
        { ...stroke(nfd), x1: 1400, y1: 1600, x2: 1200, y2: 1200,
            color: "#123456", width: 350 },
    ];
    const before = structuredClone(source);
    assert.deepEqual(candidatePenPaths(source, nfd), [
        { first: 1, last: 2, start: { x: 100, y: 300 },
            end: { x: 800, y: 900 }, width: 250, color: "#A1B2C3" },
        { first: 3, last: 4, start: { x: 800, y: 900 },
            end: { x: 1400, y: 1600 }, width: 350, color: "#A1B2C3" },
        { first: 5, last: 5, start: { x: 1400, y: 1600 },
            end: { x: 1200, y: 1200 }, width: 350, color: "#123456" },
    ]);
    assert.deepEqual(candidatePenPaths(source, "ñ"), [
        { first: 1, last: 1, start: { x: 100, y: 300 },
            end: { x: 400, y: 600 }, width: 250, color: "#A1B2C3" },
    ]);
    assert.deepEqual(candidatePenPaths(source, "∞"), []);
    assert.throws(() => candidatePenPaths(source, "nn"), /exact visible/);
    assert.throws(() => candidatePenPaths(source, " "), /exact visible/);
    assert.deepEqual(source, before, "inspection never edits a source");
    const reversed = source.slice();
    reversed[2] = { ...reversed[2], x1: 800, y1: 900,
        x2: 400, y2: 600 };
    assert.equal(candidatePenPaths(reversed, nfd).length, 4,
        "reversing a line breaks its previously directed joint");
});

test("candidate pen-path inspector handles full native stroke budget", () => {
    const source = Array.from({ length: 4096 }, (_, i) => ({
        ...stroke("ó"), x1: 100, y1: 100,
        x2: 100 + i % 100, y2: 200 + i % 100,
    }));
    assert.equal(candidatePenPaths(source, "ó").length, 4096);
    const chain = Array.from({ length: 4096 }, (_, i) => ({
        ...stroke("ó"), x1: 100, y1: i, x2: 100, y2: i + 1,
    }));
    assert.deepEqual(candidatePenPaths(chain, "ó"), [
        { first: 1, last: 4096, start: { x: 100, y: 0 },
            end: { x: 100, y: 4096 }, width: 250, color: "#A1B2C3" },
    ]);
});

test("path copy appends selected directed ink across exact Unicode", () => {
    const nfd = "n\u0303";
    const a = { ...stroke(nfd), x1: 500, y1: 600,
        x2: 1000, y2: 1300 };
    const b = { ...a, x1: 1000, y1: 1300, x2: 1400, y2: 1800 };
    const accent = { ...a, x1: 1800, y1: 3000,
        x2: 2300, y2: 3500, color: "#334455" };
    const other = stroke("ñ");
    const original = [a, other, b, stroke("∞"), accent];
    const bytes = encodeGlyphLibrary(original);
    const expected = [...original,
        { ...a, grapheme: "ñ" }, { ...b, grapheme: "ñ" }];
    const changed = copyCandidatePath(original, nfd, 2, "ñ");
    assert.deepEqual(changed, expected);
    assert.equal(encodeGlyphLibrary(changed),
        encodeGlyphLibrary(expected));
    assert.equal(encodeGlyphLibrary(original), bytes,
        "copy leaves every original row and source shape untouched");
    const second = copyCandidatePath(changed, nfd, 4, "á");
    assert.deepEqual(second, [...expected, { ...accent, grapheme: "á" }]);
    assert.deepEqual(candidatePenPaths(second, "á").map((path) =>
        [path.first, path.last]), [[1, 1]]);
    assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(second)), second);
    for (const [source, index, target] of [
        [nfd, 1, nfd], [nfd, 1, "ab"], [nfd, 1, " "],
        [nfd, 5, "á"], [nfd, -1, "á"], [nfd, 0.5, "á"],
        ["ñ", 2, "á"], ["n", 0, "á"], [nfd, 3, "á"],
    ]) {
        assert.throws(() => copyCandidatePath(original,
            source, index, target));
        assert.equal(encodeGlyphLibrary(original), bytes);
    }
    const full = Array.from({ length: 4096 }, (_, index) => ({
        ...stroke("ñ"), x1: 100, y1: 200,
        x2: 150 + index % 100, y2: 250 + index % 100,
    }));
    assert.throws(() => copyCandidatePath(full, "ñ", 0, "á"),
        /4096|stroke count/);
    const invalidUnused = [...original,
        { ...stroke("á"), x2: 100, y2: 200 }];
    assert.throws(() => copyCandidatePath(invalidUnused, nfd, 0, "ñ"),
        /Stroke endpoints must differ/);
});

test("shifted pen path copies preserve full round caps and source ink", () => {
    const nfd = "n\u0303";
    const a = { ...stroke(nfd), x1: 500, y1: 500,
        x2: 900, y2: 1000, width: 251 };
    const b = { ...a, x1: 900, y1: 1000, x2: 1600, y2: 1800 };
    const prior = { ...stroke("ñ"), x1: 500, y1: 200 };
    const library = [a, prior, b, stroke("∞")];
    const bytes = encodeGlyphLibrary(library);
    const moved = (target) => [{ ...a, grapheme: target,
        x1: 600, y1: 700, x2: 1000, y2: 1200 },
    { ...b, grapheme: target,
        x1: 1000, y1: 1200, x2: 1700, y2: 2000 }];
    const cross = copyShiftedCandidatePath(library, nfd, 2, "ñ", 100, 200);
    assert.deepEqual(cross, [...library, ...moved("ñ")]);
    const same = copyShiftedCandidatePath(library, nfd, 0, nfd, 100, 200);
    assert.deepEqual(same, [...library, ...moved(nfd)]);
    assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(same)), same);
    for (const [target, dx, dy] of [
        ["ñ", 0, 0], ["ñ", 3500, 1], ["ñ", -500, 0],
        ["ñ", 0, -500], ["ab", 100, 1], [" ", 100, 1],
        ["ñ", 0.5, 1], ["ñ", 3501, 1], ["ñ", 1, -4501],
    ]) {
        assert.throws(() => copyShiftedCandidatePath(
            library, nfd, 0, target, dx, dy));
        assert.equal(encodeGlyphLibrary(library), bytes);
    }
    for (const index of [-1, 1, 9, 1.5]) {
        assert.throws(() => copyShiftedCandidatePath(
            library, nfd, index, "ñ", 100, 100));
    }
    const full = Array.from({ length: 4096 }, (_, i) => ({
        ...a, x1: 500, y1: 500, x2: 600 + i % 500, y2: 800,
    }));
    assert.throws(() => copyShiftedCandidatePath(
        full, nfd, 0, "ñ", 100, 100), /4096|stroke count/);
    assert.equal(encodeGlyphLibrary(library), bytes);
});

test("reversing one candidate path preserves exact Unicode TSV slots", () => {
    const nfd = "n\u{303}";
    const first = { ...stroke(nfd), x1: 400, y1: 500,
        x2: 800, y2: 900 };
    const second = { ...first, x1: 800, y1: 900,
        x2: 1200, y2: 1500 };
    const third = { ...first, x1: 1200, y1: 1500,
        x2: 1800, y2: 2000 };
    const other = { ...stroke("ñ"), x1: 900, y1: 1000 };
    const math = { ...stroke("∞"), x1: 600, y1: 700 };
    const isolated = { ...stroke(nfd), x1: 2000, y1: 2500 };
    const original = [first, other, second, third, math, isolated];
    const before = encodeGlyphLibrary(original);
    const result = reverseCandidatePath(original, nfd, 0);
    const flipped = (line) => ({
        ...line, x1: line.x2, y1: line.y2, x2: line.x1, y2: line.y1,
    });
    const expected = [
        flipped(third), other, flipped(second), flipped(first), math, isolated,
    ];
    assert.equal(result.selectedIndex, 3,
        "the same selected segment moves to its mirrored authored slot");
    assert.deepEqual(result.strokes, expected);
    assert.deepEqual(candidatePenPaths(result.strokes, nfd), [
        { first: 1, last: 3, start: { x: 1800, y: 2000 },
            end: { x: 400, y: 500 }, width: 250, color: "#A1B2C3" },
        { first: 4, last: 4, start: { x: 2000, y: 2500 },
            end: { x: 3000, y: 4200 }, width: 250, color: "#A1B2C3" },
    ]);
    assert.equal(encodeGlyphLibrary(result.strokes),
        encodeGlyphLibrary(expected), "native TSV order stays exact");
    const again = reverseCandidatePath(
        result.strokes, nfd, result.selectedIndex,
    );
    assert.equal(again.selectedIndex, 0);
    assert.equal(encodeGlyphLibrary(again.strokes), before,
        "reversing a path twice restores every source byte");
    for (const [glyph, index] of [["ñ", 0], [nfd, 1], [nfd, 5],
        [nfd, -1], [nfd, 0.5], ["ab", 0]]) {
        assert.throws(() => reverseCandidatePath(original, glyph, index));
    }
    assert.equal(encodeGlyphLibrary(original), before,
        "refused reversal cannot mutate the original in-memory draft");
});

test("move adjacent complete paths without breaking stroke continuity", () => {
    const nfd = "n\u{303}";
    const line = (grapheme, x1, y1, x2, y2) => ({
        grapheme, x1, y1, x2, y2,
        color: "#A1B2C3", width: 250,
    });
    const a = line(nfd, 100, 200, 400, 600);
    const b = line(nfd, 400, 600, 700, 900);
    const c = line(nfd, 1500, 1600, 1800, 1900);
    const d = line(nfd, 800, 2500, 1000, 2700);
    const e = line(nfd, 1000, 2700, 1200, 2900);
    const other = line("ñ", 900, 300, 1000, 400);
    const math = line("∞", 400, 1200, 500, 1300);
    const original = [a, other, b, math, c, d, other, e];
    const saved = encodeGlyphLibrary(original);
    const later = moveCandidatePath(original, nfd, 2, 1);
    assert.deepEqual(later, {
        strokes: [c, other, a, math, b, d, other, e],
        selectedIndex: 4,
    });
    assert.deepEqual(candidatePenPaths(later.strokes, nfd)
        .map((path) => [path.first, path.last]),
    [[1, 1], [2, 3], [4, 5]]);
    const restored = moveCandidatePath(later.strokes, nfd,
        later.selectedIndex, -1);
    assert.equal(restored.selectedIndex, 2);
    assert.equal(encodeGlyphLibrary(restored.strokes), saved);
    const first = moveCandidatePath(original, nfd, 5, -1);
    assert.deepEqual(first.strokes,
        [a, other, b, math, d, e, other, c]);
    assert.equal(first.selectedIndex, 4);
    assert.deepEqual(candidatePenPaths(first.strokes, nfd)
        .map((path) => [path.first, path.last]),
    [[1, 2], [3, 4], [5, 5]]);
    const returnToOriginal = moveCandidatePath(first.strokes, nfd,
        first.selectedIndex, 1);
    assert.equal(encodeGlyphLibrary(returnToOriginal.strokes), saved);
    for (const [index, direction, glyph] of [
        [0, -1, nfd], [7, 1, nfd], [1, 1, nfd],
        [-1, 1, nfd], [0.5, 1, nfd], [0, 0, nfd],
        [0, 2, nfd], [0, 1, "n"],
    ]) {
        assert.throws(() => moveCandidatePath(
            original, glyph, index, direction));
    }
    assert.equal(encodeGlyphLibrary(original), saved,
        "no path movement can mutate the caller's source");
});

test("whole candidate path translation retains exact directed pen joins",
    () => {
    const nfd = "n\u{303}";
    const a = { ...stroke(nfd), x1: 500, y1: 600,
        x2: 900, y2: 1000, width: 300 };
    const b = { ...a, x1: 900, y1: 1000, x2: 1400, y2: 1500 };
    const c = { ...a, x1: 1700, y1: 2800,
        x2: 2000, y2: 3100, width: 350 };
    const other = { ...stroke("ñ"), x1: 300, y1: 400 };
    const math = { ...stroke("∞"), x1: 400, y1: 500 };
    const original = [a, other, b, math, c];
    const source = encodeGlyphLibrary(original);
    const shifted = translateCandidatePath(original, nfd, 2, 150, -200);
    assert.equal(shifted.selectedIndex, 2);
    const offset = (item, dx, dy) => ({
        ...item, x1: item.x1 + dx, x2: item.x2 + dx,
        y1: item.y1 + dy, y2: item.y2 + dy,
    });
    assert.deepEqual(shifted.strokes,
        [offset(a, 150, -200), other, offset(b, 150, -200),
            math, c]);
    assert.deepEqual(candidatePenPaths(shifted.strokes, nfd)
        .map((path) => [path.first, path.last]), [[1, 2], [3, 3]]);
    assert.deepEqual(translateCandidatePath(
        shifted.strokes, nfd, 2, -150, 200,
    ).strokes, original);
    const single = translateCandidatePath(original, nfd, 4, -100, -200);
    assert.deepEqual(single.strokes,
        [a, other, b, math, offset(c, -100, -200)]);
    assert.equal(encodeGlyphLibrary(original), source,
        "a translation never mutates the source bytes");
    for (const [glyph, index, dx, dy] of [
        [nfd, 0, 0, 0], [nfd, 0, 0.5, 100],
        [nfd, 0, Number.NaN, 100], [nfd, 0, 3501, 0],
        [nfd, 0, -500, 0], [nfd, 0, 0, -600],
        [nfd, 0, 3000, 0], [nfd, 0, 0, -4501],
        [nfd, -1, 100, 0], [nfd, 1, 100, 0],
        [nfd, 99, 100, 0], ["ñ", 0, 100, 0],
    ]) {
        assert.throws(() => translateCandidatePath(
            original, glyph, index, dx, dy,
        ), "invalid selection, shift or pen clearance must refuse");
        assert.equal(encodeGlyphLibrary(original), source);
    }
    const chain = Array.from({ length: 4096 }, (_, i) => ({
        ...stroke("ó"), x1: 600, x2: 600,
        y1: 200 + i, y2: 201 + i,
    }));
    const huge = translateCandidatePath(chain, "ó", 3000, 100, 0);
    assert.equal(huge.strokes.length, 4096);
    assert.equal(candidatePenPaths(huge.strokes, "ó").length, 1);
    assert.equal(chain[0].x1, 600);
    assert.throws(() => translateCandidatePath(
        chain, "ó", 3000, 0, 200,
    ), "the final pen segment would leave the paper cell");
});

test("untrusted glyph TSV and edited numeric fields fail closed", () => {
    const valid = encodeGlyphLibrary([stroke()]);
    const bad = [
        "", "unversioned\n", valid.replaceAll("\n", "\r\n"),
        valid.slice(0, -1), `${header}ñ\t1\t2\n`,
        `${header}ab\t1\t2\t3\t4\t80\t#123456\n`,
        `${header}\n`, `${header}ñ\t0\t0\t0\t0\t80\t#123456\n`,
        `${header}ñ\t0\t0\t3501\t4\t80\t#123456\n`,
        `${header}ñ\t0\t0\t1\t4501\t80\t#123456\n`,
        `${header}ñ\t0\t0\t1\t4\t79\t#123456\n`,
        `${header}ñ\t0\t0\t1\t4\t1501\t#123456\n`,
        `${header}ñ\t0\t0\t1\t4\t80\t#Q23456\n`,
        `${header}ñ\t-1\t0\t1\t4\t80\t#123456\n`,
        `${header}ñ\tNaN\t0\t1\t4\t80\t#123456\n`,
        `${header}ñ\t0\t0\t1\t4\t80\t#123456\tbogus\n`,
        `${header}ñ\t0\t0\t1\t4\t80\t#123456\n`.repeat(4097),
    ];
    for (const candidate of bad) {
        assert.throws(() => parseGlyphLibrary(candidate), undefined,
            `refuse ${candidate.slice(0, 70)}`);
    }
    assert.throws(() => encodeGlyphLibrary([]));
    assert.throws(() => encodeGlyphLibrary([stroke("a\n")]));
    assert.throws(() => encodeGlyphLibrary([stroke("ab")]));
    assert.throws(() => encodeGlyphLibrary(Array(4097).fill(stroke())));
    assert.throws(() => parseGlyphLibrary("ü".repeat(262145)));
    const expanded = Array.from({ length: 600 }, (_, index) => ({
        ...stroke(index % 2 === 0 ? "ñ" : "n\u{303}"),
        x1: 100 + index,
    }));
    assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(expanded)), expanded);
});

const variation = {
    width: 8, height: 8, thickness: 10,
    slant: 7, spacing: 200, regularity: 40,
};

test("seeded variation replays and preserves joins", () => {
    const source = [
        { ...stroke("á"), x1: 900, y1: 1200, x2: 1700, y2: 2400 },
        { ...stroke("á"), x1: 1700, y1: 2400, x2: 2400, y2: 3100 },
        { ...stroke("á"), x1: 1300, y1: 700, x2: 1600, y2: 400 },
        stroke("ñ"),
    ];
    const before = structuredClone(source);
    const repeat = varyGlyphStrokes(source, "á", 42, 0, variation);
    assert.deepEqual(repeat,
        varyGlyphStrokes(source, "á", 42, 0, variation));
    assert.notDeepEqual(repeat,
        varyGlyphStrokes(source, "á", 42, 1, variation));
    assert.deepEqual([repeat[0].x2, repeat[0].y2],
        [repeat[1].x1, repeat[1].y1],
        "authored shared nodes must remain connected after irregularity");
    assert.deepEqual(repeat.map((item) => item.color),
        source.slice(0, 3).map((item) => item.color));
    assert.deepEqual(repeat.map((item) => item.grapheme), ["á", "á", "á"]);
    assert.deepEqual(varyGlyphStrokes(source, "á", 0, 0, {
        width: 0, height: 0, thickness: 0, slant: 0,
        spacing: 0, regularity: 0,
    }), before.slice(0, 3), "zero amplitudes leave geometric ideal intact");
    const reversed = varyGlyphStrokes([...source].reverse(), "á", 42,
        0, variation);
    assert.deepEqual(reversed, [...repeat].reverse(),
        "reordering pen strokes must not change seeded geometry");
    assert.deepEqual(source, before, "variation never changes authored ink");
});

test("seeded variation refuses invalid bounds without changing ink", () => {
    const source = [
        { ...stroke("ó"), x1: 900, y1: 1200, x2: 2400, y2: 3100 },
    ];
    const before = structuredClone(source);
    for (const seed of [-1, 0.4, Number.NaN, 4294967296]) {
        assert.throws(() => varyGlyphStrokes(source, "ó", seed, 0,
            variation));
    }
    for (const occurrence of [-1, 1.5, 10001]) {
        assert.throws(() => varyGlyphStrokes(source, "ó", 1,
            occurrence, variation));
    }
    for (const [key, invalid] of [
        ["width", 21], ["height", 21], ["thickness", 21],
        ["slant", 13], ["regularity", 151], ["spacing", 501],
        ["width", 1.2], ["slant", -1],
    ]) {
        assert.throws(() => varyGlyphStrokes(source, "ó", 42, 0,
            { ...variation, [key]: invalid }), `invalid ${key}`);
    }
    assert.throws(() => varyGlyphStrokes(source, "o", 42, 0, variation));
    assert.throws(() => varyGlyphStrokes(Array(129).fill(source[0]), "ó",
        42, 0, variation));
    assert.throws(() => varyGlyphStrokes([{
        ...stroke("ó"), x1: 0, y1: 0, x2: 3500, y2: 4500,
    }], "ó", 42, 0, variation), /fit inside glyph cell/);
    assert.deepEqual(source, before);
});

class FakeElement {
    value = "";
    disabled = true;
    textContent = "";
    size = 0;
    files = null;
    attributes = new Map();
    children = [];
    listeners = new Map();
    addEventListener(type, callback) { this.listeners.set(type, callback); }
    dispatch(type, event = {}) { return this.listeners.get(type)?.(event); }
    setAttribute(name, value) { this.attributes.set(name, value); }
    replaceChildren(...children) { this.children = children; }
    getBoundingClientRect() {
        return { left: 0, top: 0, width: 350, height: 450 };
    }
    setPointerCapture(id) { this.captured = id; }
    hasPointerCapture(id) { return this.captured === id; }
    releasePointerCapture(id) {
        if (this.captured === id) this.captured = null;
    }
}

function mountEditor() {
    const original = {
        document: globalThis.document,
        window: globalThis.window,
        createObjectURL: URL.createObjectURL,
        revokeObjectURL: URL.revokeObjectURL,
    };
    const ids = ["glyph-editor-region", "glyph-character",
        "glyph-color", "glyph-load", "glyph-add", "glyph-copy-from",
        "glyph-copy", "glyph-compose", "glyph-selected",
        "glyph-form", "glyph-construct", "glyph-seed-lowercase",
        "glyph-seed-connected",
        "glyph-seed-baseline", "glyph-seed-connected-baseline",
        "glyph-focus-missing",
        "glyph-baseline-coverage",
        "glyph-seed-spanish", "glyph-seed-spanish-nfd",
        "glyph-seed-upper-spanish", "glyph-seed-upper-spanish-nfd",
        "glyph-seed-punctuation", "glyph-seed-quotes",
        "glyph-seed-math", "glyph-seed-digits", "glyph-seed-capitals",
        "glyph-seed-text-marks",
        "glyph-circle-x",
        "glyph-circle-y", "glyph-circle-radius", "glyph-accent-shift",
        "glyph-accent-lift", "glyph-italic-angle",
        "glyph-authored-angle", "glyph-authored-baseline",
        "glyph-slant-authored", "glyph-slant-undo", "glyph-slant-redo",
        "glyph-recovery-undo-select", "glyph-recovery-redo-select",
        "glyph-recovery-status",
        "glyph-slant-preview", "glyph-slant-preview-status",
        "glyph-height-percent", "glyph-height-baseline",
        "glyph-scale-height", "glyph-height-preview",
        "glyph-height-preview-status",
        "glyph-width-percent", "glyph-width-anchor",
        "glyph-scale-width", "glyph-width-preview",
        "glyph-width-preview-status",
        "glyph-ink-percent", "glyph-scale-ink", "glyph-ink-preview",
        "glyph-ink-preview-status",
        "glyph-rotate-angle", "glyph-rotate-pivot-x",
        "glyph-rotate-pivot-y", "glyph-rotate-all",
        "glyph-rotate-preview", "glyph-rotate-preview-status",
        "glyph-reflect-axis", "glyph-reflect-pivot",
        "glyph-reflect-all", "glyph-reflect-preview",
        "glyph-reflect-preview-status",
        "glyph-move-x", "glyph-move-y", "glyph-move-all",
        "glyph-move-preview", "glyph-move-preview-status",
        "glyph-ideal-preview", "glyph-ideal-geometry",
        "glyph-ideal-circle", "glyph-ideal-stem", "glyph-ideal-arch",
        "glyph-ideal-entry", "glyph-ideal-approach", "glyph-ideal-exit",
        "glyph-ideal-rounded", "glyph-ideal-rounded-left",
        "glyph-ideal-rounded-right", "glyph-ideal-c-loop",
        "glyph-ideal-e-bar", "glyph-ideal-i-stem", "glyph-ideal-i-dot",
        "glyph-ideal-accent", "glyph-ideal-diacritic",
        "glyph-update", "glyph-remove", "glyph-subdivide",
        "glyph-merge", "glyph-split-exact",
        "glyph-clear", "glyph-reverse",
        "glyph-reverse-path", "glyph-path-start", "glyph-path-close",
        "glyph-path-join-next", "glyph-path-join-curved",
        "glyph-path-bend-x", "glyph-path-bend-y",
        "glyph-path-earlier", "glyph-path-later",
        "glyph-path-dx", "glyph-path-dy", "glyph-path-translate",
        "glyph-path-style", "glyph-path-remove", "glyph-path-scale",
        "glyph-path-reflect", "glyph-path-reflect-axis",
        "glyph-path-rotate", "glyph-path-rotate-degrees",
        "glyph-path-rotate-pivot-x", "glyph-path-rotate-pivot-y",
        "glyph-path-shear", "glyph-path-shear-degrees",
        "glyph-path-shear-baseline",
        "glyph-path-reflect-pivot",
        "glyph-path-scale-x", "glyph-path-scale-y",
        "glyph-path-copy", "glyph-path-copy-target",
        "glyph-path-copy-shifted",
        "glyph-earlier", "glyph-later", "glyph-join",
        "glyph-paths", "glyph-variation-preview", "glyph-variation-status",
        "glyph-variation-seed", "glyph-variation-sample",
        "glyph-variation-download", ...["width", "height", "thickness",
            "slant", "spacing", "regularity"].map((key) =>
            `glyph-variation-${key}`),
        "glyph-save", "glyph-preview",
        "glyph-status", ...["x1", "y1", "x2", "y2", "width"]
            .flatMap((axis) => [`glyph-${axis}`, `glyph-${axis}-range`])];
    const elements = new Map(ids.map((id) => [id, new FakeElement()]));
    const get = (id) => elements.get(`glyph-${id}`);
    get("character").value = "ñ";
    get("color").value = "#243647";
    get("form").value = "o";
    get("circle-x").value = "1550";
    get("circle-y").value = "2450";
    get("circle-radius").value = "1100";
    get("accent-shift").value = "0";
    get("accent-lift").value = "0";
    get("italic-angle").value = "12";
    get("authored-angle").value = "12";
    get("authored-baseline").value = "3550";
    get("height-percent").value = "110";
    get("height-baseline").value = "2250";
    get("width-percent").value = "110";
    get("width-anchor").value = "1750";
    get("ink-percent").value = "110";
    get("rotate-angle").value = "0";
    get("rotate-pivot-x").value = "1750";
    get("rotate-pivot-y").value = "2250";
    get("reflect-axis").value = "x";
    get("reflect-pivot").value = "1750";
    get("move-x").value = "0";
    get("move-y").value = "0";
    get("path-dx").value = "0";
    get("path-dy").value = "0";
    get("path-scale-x").value = "100";
    get("path-scale-y").value = "100";
    get("path-rotate-degrees").value = "10";
    get("path-rotate-pivot-x").value = "1750";
    get("path-rotate-pivot-y").value = "2250";
    get("path-shear-degrees").value = "10";
    get("path-shear-baseline").value = "3000";
    get("path-reflect-axis").value = "x";
    get("path-reflect-pivot").value = "1750";
    get("variation-sample").value = "1";
    for (const [key, value] of Object.entries({
        seed: 42, width: 6, height: 6, thickness: 8, slant: 4,
        spacing: 160, regularity: 20,
    })) {
        get(`variation-${key}`).value = String(value);
    }
    for (const [axis, value] of Object.entries({
        x1: 100, y1: 200, x2: 3000, y2: 4200, width: 250,
    })) {
        get(axis).value = String(value);
        get(`${axis}-range`).value = String(value);
    }
    const downloads = [];
    const revoked = [];
    const links = [];
    globalThis.document = {
        getElementById: (id) => elements.get(id) ?? null,
        createElementNS: (_ns, tag) => {
            assert.equal(tag, "line");
            return new FakeElement();
        },
        createElement: (tag) => {
            assert.ok(tag === "a" || tag === "option");
            const element = new FakeElement();
            if (tag === "a") {
                element.click = () => links.push({
                    name: element.download, href: element.href,
                });
            }
            return element;
        },
    };
    globalThis.window = { setTimeout: (fn) => { fn(); } };
    URL.createObjectURL = (blob) => {
        downloads.push(blob);
        return "blob:local-ink";
    };
    URL.revokeObjectURL = (url) => revoked.push(url);
    const editor = createGlyphStrokeEditor();
    return {
        editor, get, downloads, links, revoked,
        restore: () => {
            globalThis.document = original.document;
            globalThis.window = original.window;
            URL.createObjectURL = original.createObjectURL;
            URL.revokeObjectURL = original.revokeObjectURL;
        },
    };
}

test("geometric glyph authoring refuses coerced decimal inputs", async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        editor.start();
        get("character").value = "o";
        get("form").value = "o";
        get("construct").dispatch("click");
        get("save").dispatch("click");
        const accepted = await downloads.at(-1).text();
        assert.equal(parseGlyphLibrary(accepted).length, 48);

        // Single-glyph previews and every bulk family must not reinterpret
        // hexadecimal, exponent, positive sign or surrounding whitespace.
        const invalid = [
            ["circle-x", "+1550", "construct"],
            ["circle-y", "2.45e3", "seed-lowercase"],
            ["circle-radius", "0x44c", "seed-connected"],
            ["width", "2.5e2", "seed-baseline"],
            ["circle-x", " 1550", "seed-punctuation"],
            ["circle-y", "2.45e3", "seed-quotes"],
            ["circle-radius", "0x44c", "seed-math"],
            ["width", "+250", "seed-digits"],
            ["circle-x", "1.55e3", "seed-capitals"],
            ["circle-y", "+2450", "seed-text-marks"],
            ["accent-shift", "+0", "seed-connected-baseline"],
            ["accent-lift", "0e0", "seed-baseline"],
        ];
        for (const [field, raw, action] of invalid) {
            const input = get(field);
            const previous = input.value;
            input.value = raw;
            input.dispatch("input");
            get(action).dispatch("click");
            assert.match(get("status").textContent, /whole decimal/,
                `${action} accepted ${field}=${raw}`);
            get("save").dispatch("click");
            assert.equal(await downloads.at(-1).text(), accepted,
                `${action} changed the previously authored TSV`);
            input.value = previous;
            input.dispatch("input");
        }

        get("form").value = "o-italic";
        get("italic-angle").value = "1.2e1";
        get("italic-angle").dispatch("input");
        get("construct").dispatch("click");
        assert.match(get("status").textContent, /whole decimal/);
        get("save").dispatch("click");
        assert.equal(await downloads.at(-1).text(), accepted);
    } finally {
        ui.restore();
    }
});

test("accent composition rejects coerced offsets without losing ink",
    async () => {
        const ui = mountEditor();
        try {
            const { editor, get, downloads } = ui;
            editor.start();
            get("seed-lowercase").dispatch("click");
            get("save").dispatch("click");
            const accepted = await downloads.at(-1).text();
            assert.ok(parseGlyphLibrary(accepted).length > 200);

            const invalid = [
                ["accent-shift", "+0", "seed-spanish"],
                ["accent-lift", "0e0", "seed-spanish-nfd"],
                ["accent-shift", "-5e1", "seed-spanish"],
                ["accent-lift", "0x0", "seed-spanish-nfd"],
            ];
            for (const [field, raw, action] of invalid) {
                const control = get(field);
                const previous = control.value;
                control.value = raw;
                get(action).dispatch("click");
                assert.match(get("status").textContent, /whole decimal/,
                    `${action} accepted ${field}=${raw}`);
                get("save").dispatch("click");
                assert.equal(await downloads.at(-1).text(), accepted);
                control.value = previous;
            }
            get("character").value = "ñ";
            get("copy-from").value = "n";
            get("accent-shift").value = "+0";
            get("compose").dispatch("click");
            assert.match(get("status").textContent, /whole decimal/);
            get("save").dispatch("click");
            assert.equal(await downloads.at(-1).text(), accepted);

            // The same controls still admit a negative decimal offset,
            // emitting independently editable exact Unicode accent ink.
            get("accent-shift").value = "-100";
            get("accent-lift").value = "50";
            get("compose").dispatch("click");
            assert.match(get("status").textContent, /[Cc]omposed/);
            get("save").dispatch("click");
            const composed = parseGlyphLibrary(await downloads.at(-1).text());
            assert.ok(composed.some((stroke) => stroke.grapheme === "ñ"));
        } finally {
            ui.restore();
        }
    });

test("authored glyph transforms reject numeric coercions", () => {
    const ui = mountEditor();
    try {
        const { get, editor } = ui;
        editor.start();
        get("x1").value = "300";
        get("add").dispatch("click");
        assert.equal(get("preview").children.length, 1);
        const original = get("preview").children.map((item) =>
            [...item.attributes]);
        const invalid = [
            ["authored-angle", "+12", "slant-authored", "slant-preview"],
            ["authored-baseline", "3.55e3", "slant-authored",
                "slant-preview"],
            ["height-percent", "1e2", "scale-height", "height-preview"],
            ["height-baseline", "0x8ca", "scale-height",
                "height-preview"],
            ["width-percent", "1e2", "scale-width", "width-preview"],
            ["width-anchor", "2.25e3", "scale-width", "width-preview"],
            ["ink-percent", "1e2", "scale-ink", "ink-preview"],
            ["rotate-angle", "0e0", "rotate-all", "rotate-preview"],
            ["rotate-pivot-x", "1.75e3", "rotate-all",
                "rotate-preview"],
            ["rotate-pivot-y", "0x8ca", "rotate-all",
                "rotate-preview"],
            ["reflect-pivot", "0x6d6", "reflect-all",
                "reflect-preview"],
            ["move-x", "0e0", "move-all", "move-preview"],
            ["move-y", "+0", "move-all", "move-preview"],
        ];
        for (const [field, raw, action, preview] of invalid) {
            const control = get(field);
            const before = control.value;
            control.value = raw;
            control.dispatch("input");
            assert.equal(get(preview).children.length, 0,
                `${field} accepted ${JSON.stringify(raw)} in preview`);
            get(action).dispatch("click");
            assert.match(get("status").textContent, /whole decimal/,
                `${field} accepted ${JSON.stringify(raw)} in draft`);
            assert.deepEqual(get("preview").children.map((item) =>
                [...item.attributes]), original);
            control.value = before;
            control.dispatch("input");
        }
        get("move-y").value = "-20";
        get("move-y").dispatch("input");
        assert.equal(get("move-preview").children.length, 1,
            "signed decimal physical offsets remain supported");
    } finally {
        ui.restore();
    }
});

test("two-row accented comparison keeps authored TSV", async () => {
    const ui = mountEditor();
    try {
        const { get, editor, downloads, links } = ui;
        assert.equal(get("variation-seed").disabled, true);
        assert.equal(get("variation-preview").children.length, 0);
        editor.start();
        get("character").value = "ó";
        get("character").dispatch("input");
        get("form").value = "ó";
        get("circle-radius").value = "800";
        get("construct").dispatch("click");
        const original = get("preview").children.map((item) =>
            [...item.attributes]);
        const first = get("variation-preview").children;
        assert.equal(original.length, 49);
        assert.equal(first.length, 49 * 6);
        assert.deepEqual(first.map((node) =>
            node.attributes.get("data-variation-row")).slice(0, 3),
            ["ideal", "ideal", "ideal"]);
        assert.equal(first.at(-1).attributes.get("data-variation-row"),
            "seeded");
        const snapshot = first.map((node) => [...node.attributes]);
        get("variation-seed").dispatch("input");
        assert.deepEqual(get("variation-preview").children.map((node) =>
            [...node.attributes]), snapshot, "seeded replay is identical");
        get("save").dispatch("click");
        const native = await downloads[0].text();
        get("variation-seed").value = "123";
        get("variation-seed").dispatch("input");
        const different = get("variation-preview").children.map(
            (node) => [...node.attributes]);
        assert.notDeepEqual(different, snapshot);
        assert.deepEqual(different.slice(0, 49 * 3),
            snapshot.slice(0, 49 * 3), "ideal row remains unchanged");
        get("variation-spacing").value = "500";
        get("variation-spacing").dispatch("input");
        const spaced = get("variation-preview").children.map(
            (node) => [...node.attributes]);
        assert.deepEqual(spaced.slice(0, 49 * 3),
            snapshot.slice(0, 49 * 3));
        assert.notDeepEqual(spaced.slice(49 * 3),
            different.slice(49 * 3), "spacing varies only the lower row");
        for (const line of get("variation-preview").children) {
            for (const axis of ["x1", "x2"]) {
                const x = Number(line.attributes.get(axis));
                const pen = Number(line.attributes.get("stroke-width"));
                assert.ok(x - pen / 2 >= 0 && x + pen / 2 <= 14000,
                    `preview ${axis} and pen remain in SVG at max spacing`);
            }
        }
        get("variation-width").value = "";
        get("variation-width").dispatch("input");
        assert.equal(get("variation-preview").children.length, 0);
        assert.match(get("variation-status").textContent, /whole units/);
        get("variation-width").value = "6";
        get("variation-spacing").value = "160";
        get("variation-seed").value = "4294967296";
        get("variation-seed").dispatch("input");
        assert.equal(get("variation-preview").children.length, 0);
        assert.match(get("variation-status").textContent,
            /outside native bounds/);
        assert.deepEqual(get("preview").children.map((item) =>
            [...item.attributes]), original);
        get("variation-seed").value = "42";
        get("variation-seed").dispatch("input");
        assert.equal(get("variation-preview").children.length, 49 * 6);
        get("save").dispatch("click");
        assert.equal(links.length, 2);
        assert.equal(await downloads[1].text(), native,
            "preview-only variation cannot alter native TSV");
        editor.reset();
        assert.equal(get("variation-preview").children.length, 0);
        assert.equal(get("variation-seed").disabled, true);
    } finally {
        ui.restore();
    }
});

test("explicit varied TSV preserves the authored draft", async () => {
    const ui = mountEditor();
    try {
        const { get, editor, downloads, links } = ui;
        get("variation-download").dispatch("click");
        assert.equal(links.length, 0, "session gate blocks download");
        editor.start();
        assert.equal(get("variation-download").disabled, true);
        get("character").value = "ó";
        get("character").dispatch("input");
        get("form").value = "ó";
        get("circle-radius").value = "800";
        get("construct").dispatch("click");
        get("character").value = "∞";
        get("character").dispatch("input");
        get("add").dispatch("click");
        get("character").value = "ó";
        get("character").dispatch("input");
        assert.equal(get("variation-download").disabled, false);
        get("save").dispatch("click");
        const canonical = await downloads[0].text();
        const originals = parseGlyphLibrary(canonical);
        get("variation-sample").value = "2";
        get("variation-download").dispatch("click");
        assert.equal(links.length, 2);
        assert.equal(links[0].name, "my-glyph-strokes.tsv");
        assert.equal(links[1].name, "my-varied-glyph-strokes.tsv");
        const variant = parseGlyphLibrary(await downloads[1].text());
        assert.equal(variant.length, originals.length);
        assert.deepEqual(variant.filter((item) => item.grapheme === "∞"),
            originals.filter((item) => item.grapheme === "∞"));
        assert.deepEqual(variant.filter((item) => item.grapheme === "ó"),
            varyGlyphStrokes(originals, "ó", 42, 1, {
                width: 6, height: 6, thickness: 8, slant: 4,
                spacing: 160, regularity: 20,
            }), "downloaded ink must be the displayed sample geometry");
        assert.notDeepEqual(variant, originals);
        get("variation-sample").value = "4";
        get("variation-download").dispatch("click");
        assert.equal(links.length, 2);
        assert.match(get("status").textContent, /outside native bounds/);
        get("variation-sample").value = "3";
        get("variation-width").value = "21";
        get("variation-width").dispatch("input");
        assert.equal(get("variation-download").disabled, true);
        get("variation-download").dispatch("click");
        assert.equal(links.length, 2, "invalid preview refuses variant export");
        get("variation-width").value = "6";
        get("variation-width").dispatch("input");
        assert.equal(get("variation-download").disabled, false);
        get("variation-download").dispatch("click");
        assert.equal(links.length, 3);
        assert.notDeepEqual(parseGlyphLibrary(await downloads[2].text()),
            variant, "a different selected occurrence has different ink");
        get("save").dispatch("click");
        assert.equal(await downloads[3].text(), canonical,
            "the canonical TSV is byte-identical after varied exports");
        editor.reset();
        assert.equal(get("variation-download").disabled, true);
    } finally {
        ui.restore();
    }
});

test("glyph editor refuses surrogate input without replacing valid TSV",
    async () => {
    const ui = mountEditor();
    try {
        const { get, editor, downloads } = ui;
        editor.start();
        get("add").dispatch("click");
        get("save").dispatch("click");
        const accepted = await downloads[0].text();
        for (const glyph of ["\uD800", "\uDC00"]) {
            get("character").value = glyph;
            get("character").dispatch("input");
            get("add").dispatch("click");
            assert.match(get("status").textContent, /Unicode/);
            get("save").dispatch("click");
            assert.equal(await downloads.at(-1).text(), accepted,
                "invalid glyph must not mutate the existing ink draft");
        }
        editor.reset();
    } finally {
        ui.restore();
    }
});

test("glyph editor refuses structural separators without replacing TSV",
    async () => {
    const ui = mountEditor();
    try {
        const { get, editor, downloads } = ui;
        editor.start();
        get("add").dispatch("click");
        get("save").dispatch("click");
        const accepted = await downloads[0].text();
        for (const separator of ["\u2028", "\u2029"]) {
            get("character").value = separator;
            get("character").dispatch("input");
            get("add").dispatch("click");
            assert.match(get("status").textContent, /visible Unicode/);
            get("save").dispatch("click");
            assert.equal(await downloads.at(-1).text(), accepted,
                "invalid row structure must not mutate authored ink");
        }
        editor.reset();
    } finally {
        ui.restore();
    }
});

test("zero-width glyph refusal preserves browser-authored draft", async () => {
    const ui = mountEditor();
    try {
        const { get, editor, downloads } = ui;
        editor.start();
        get("add").dispatch("click");
        get("save").dispatch("click");
        const accepted = await downloads[0].text();
        for (const hidden of ["\u200B", "\u200C", "\u200D",
            "\u2060", "\uFE0E", "\uFE0F", "\uFEFF",
            "\uFFF9", "\uFFFA", "\uFFFB",
            "\u200D\uFE0F", "\u2800\uFE0F",
            "\u0600\u200D", "\u{E0067}\u{E007F}"]) {
            get("character").value = hidden;
            get("character").dispatch("input");
            get("add").dispatch("click");
            assert.match(get("status").textContent, /visible Unicode/);
            get("save").dispatch("click");
            assert.equal(await downloads.at(-1).text(), accepted,
                "refused invisible identity never replaces exact glyph ink");
        }
    } finally {
        ui.restore();
    }
});

test("explicit SVG glyph download stays session-gated", async () => {
    const ui = mountEditor();
    try {
        const { get, editor, links, downloads } = ui;
        assert.equal(get("add").disabled, true);
        assert.equal(get("load").disabled, true);
        assert.equal(get("preview").children.length, 0);
        get("add").dispatch("click");
        get("save").dispatch("click");
        assert.equal(links.length, 0);
        editor.start();
        assert.equal(get("add").disabled, false);
        get("x1-range").value = "120";
        get("x1-range").dispatch("input");
        assert.equal(get("x1").value, "120");
        get("add").dispatch("click");
        assert.equal(get("preview").children.length, 1);
        assert.equal(get("preview").children[0].attributes.get("x1"), "120");
        assert.equal(get("preview").children[0].attributes.get("stroke"),
            "#243647");
        get("character").value = "n\u{303}";
        get("character").dispatch("input");
        assert.equal(get("preview").children.length, 0);
        get("add").dispatch("click");
        assert.equal(get("preview").children.length, 1);
        get("save").dispatch("click");
        assert.equal(links.length, 1);
        assert.equal(links[0].name, "my-glyph-strokes.tsv");
        assert.deepEqual(ui.revoked, ["blob:local-ink"]);
        assert.deepEqual(parseGlyphLibrary(await downloads[0].text())
            .map((item) => item.grapheme), ["ñ", "n\u{303}"]);
        get("remove").dispatch("click");
        assert.equal(get("preview").children.length, 0);
        get("character").value = "ñ";
        get("character").dispatch("input");
        assert.equal(get("preview").children.length, 1);
        editor.reset();
        assert.equal(get("preview").children.length, 0);
        assert.equal(get("add").disabled, true);
        get("save").dispatch("click");
        assert.equal(links.length, 1);
    } finally {
        ui.restore();
    }
});

test("import refuses bad UTF-8 and stale file operations", async () => {
    const ui = mountEditor();
    try {
        const { editor, get } = ui;
        editor.start();
        const valid = new TextEncoder().encode(encodeGlyphLibrary([
            stroke("∞"),
        ]));
        let finish;
        get("load").files = [{
            size: valid.length,
            arrayBuffer: () => new Promise((resolve) => { finish = resolve; }),
        }];
        const pending = get("load").dispatch("change");
        editor.reset();
        editor.start();
        finish(valid.buffer);
        await pending;
        get("character").value = "∞";
        get("character").dispatch("input");
        assert.equal(get("preview").children.length, 0,
            "previous session file must not resurrect after reset");
        get("load").files = [{
            size: 1,
            arrayBuffer: async () => Uint8Array.from([255]).buffer,
        }];
        await get("load").dispatch("change");
        assert.match(get("status").textContent, /TypeError|EncodingError/i);
        get("load").files = [{
            size: valid.length,
            arrayBuffer: async () => valid.buffer,
        }];
        await get("load").dispatch("change");
        assert.equal(get("preview").children.length, 1);
    } finally {
        ui.restore();
    }
});

test("glyph import refuses leading UTF-8 BOM without replacing ink",
    async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        editor.start();
        get("add").dispatch("click");
        get("save").dispatch("click");
        const accepted = await downloads[0].text();
        const canonical = new TextEncoder().encode(encodeGlyphLibrary([
            stroke("∞"),
        ]));
        const prefixed = Uint8Array.of(0xEF, 0xBB, 0xBF, ...canonical);
        get("load").files = [{
            size: prefixed.byteLength,
            arrayBuffer: async () => prefixed.buffer,
        }];
        await get("load").dispatch("change");
        assert.match(get("status").textContent, /header|BOM/i,
            "Rust rejects a BOM before the exact v1 glyph TSV header");
        get("save").dispatch("click");
        assert.equal(await downloads[1].text(), accepted,
            "failed import must keep the existing unsaved ink intact");
        get("load").files = [{
            size: canonical.byteLength,
            arrayBuffer: async () => canonical.buffer,
        }];
        await get("load").dispatch("change");
        get("save").dispatch("click");
        assert.equal(await downloads[2].text(),
            new TextDecoder().decode(canonical),
            "unprefixed UTF-8 still imports with exact bytes");
    } finally {
        ui.restore();
    }
});

test("revise any imported stroke without altering neighbors", async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        editor.start();
        const original = [stroke("ñ"), stroke("∞"),
            { ...stroke("ñ"), x1: 500, y1: 700, color: "#333333" },
            { ...stroke("ñ"), x1: 800, y1: 900 },
        ];
        const bytes = new TextEncoder().encode(encodeGlyphLibrary(original));
        get("load").files = [{
            size: bytes.length, arrayBuffer: async () => bytes.buffer,
        }];
        await get("load").dispatch("change");
        assert.equal(get("selected").children.length, 4);
        assert.equal(get("selected").disabled, false);
        assert.equal(get("update").disabled, true);
        assert.equal(get("remove").disabled, true);
        get("selected").value = "2";
        get("selected").dispatch("change");
        assert.equal(get("x1").value, "500");
        assert.equal(get("x1-range").value, "500");
        assert.equal(get("color").value, "#333333");
        assert.equal(get("update").disabled, false);
        get("x1-range").value = "1234";
        get("x1-range").dispatch("input");
        get("color").value = "#112233";
        get("update").dispatch("click");
        assert.equal(get("preview").children[1].attributes.get("x1"), "1234");
        assert.equal(get("preview").children[1].attributes
            .get("data-selected"), "true");
        get("x2").value = "1234";
        get("y2").value = "700";
        get("update").dispatch("click");
        assert.match(get("status").textContent, /endpoints must differ/i);
        assert.equal(get("preview").children[1].attributes.get("x2"), "3000");
        get("save").dispatch("click");
        assert.deepEqual(parseGlyphLibrary(await downloads[0].text()), [
            original[0], original[1],
            { ...original[2], x1: 1234, color: "#112233" }, original[3],
        ]);
        get("remove").dispatch("click");
        assert.equal(get("selected").value, "");
        assert.equal(get("preview").children.length, 2);
        get("character").value = "∞";
        get("character").dispatch("input");
        assert.equal(get("selected").children.length, 2);
        get("preview").children[0].dispatch("click");
        assert.equal(get("selected").value, "1");
        assert.equal(get("color").value, "#A1B2C3");
        editor.reset();
        assert.equal(get("selected").disabled, true);
        assert.equal(get("update").disabled, true);
        assert.equal(get("preview").children.length, 0);
    } finally {
        ui.restore();
    }
});

test("pointer drawing validates segments on release", async () => {
    const ui = mountEditor();
    try {
        const { get, editor, downloads } = ui;
        const preview = get("preview");
        const pointer = (clientX, clientY, pointerId = 12) => ({
            clientX, clientY, pointerId, isPrimary: true, button: 0,
        });
        preview.dispatch("pointerdown", pointer(10, 20));
        assert.equal(preview.children.length, 0, "no drawing before admission");
        editor.start();
        get("character").value = "ñ";
        get("character").dispatch("input");
        preview.dispatch("pointerdown", pointer(10, 20));
        assert.equal(preview.hasPointerCapture(12), true);
        assert.equal(preview.children[0].attributes.get("data-draft"), "true");
        preview.dispatch("pointermove", pointer(300, 420));
        assert.equal(preview.children[0].attributes.get("x2"), "3000");
        assert.equal(preview.children[0].attributes.get("y2"), "4200");
        preview.dispatch("pointerup", pointer(300, 420));
        assert.equal(preview.hasPointerCapture(12), false);
        assert.equal(preview.children.length, 1);
        assert.equal(preview.children[0].attributes.has("data-draft"), false);
        assert.equal(get("selected").value, "0");
        get("save").dispatch("click");
        assert.deepEqual(parseGlyphLibrary(await downloads[0].text()), [{
            grapheme: "ñ", x1: 100, y1: 200, x2: 3000, y2: 4200,
            width: 250, color: "#243647",
        }]);
        preview.dispatch("pointerdown", pointer(200, 300));
        preview.dispatch("pointerup", pointer(200, 300));
        assert.equal(preview.children.length, 1,
            "zero-length pointer gestures cannot replace the library");
        assert.match(get("status").textContent, /Drag across/);
        preview.dispatch("pointerdown", pointer(50, 60));
        preview.dispatch("pointercancel", pointer(80, 90));
        assert.equal(preview.children.length, 1, "canceled draw is atomic");
        get("width").value = "79";
        preview.dispatch("pointerdown", pointer(10, 20));
        assert.equal(preview.children.length, 1);
        assert.match(get("status").textContent, /native bounds/);
        get("width").value = "250";
        get("character").value = "∞";
        get("character").dispatch("input");
        preview.dispatch("pointerdown", pointer(100, 100));
        editor.reset();
        preview.dispatch("pointerup", pointer(300, 400));
        assert.equal(preview.children.length, 0, "reset invalidates a drag");
    } finally {
        ui.restore();
    }
});

test("curved pointer traces preserve samples and refuse excess", async () => {
    const ui = mountEditor();
    try {
        const { get, editor, downloads } = ui;
        const preview = get("preview");
        const pointer = (x, y) => ({
            clientX: x, clientY: y, pointerId: 99,
            isPrimary: true, button: 0,
        });
        editor.start();
        preview.dispatch("pointerdown", pointer(10, 10));
        preview.dispatch("pointermove", pointer(100, 100));
        preview.dispatch("pointermove", pointer(200, 70));
        assert.equal(preview.children.length, 2);
        assert.equal(preview.children[1].attributes.get("data-draft"), "true");
        preview.dispatch("pointerup", pointer(250, 140));
        assert.equal(preview.children.length, 3);
        assert.equal(get("selected").value, "2");
        get("save").dispatch("click");
        assert.deepEqual(parseGlyphLibrary(await downloads[0].text()), [
            { ...stroke(), x1: 100, y1: 100, x2: 1000, y2: 1000,
                color: "#243647" },
            { ...stroke(), x1: 1000, y1: 1000, x2: 2000, y2: 700,
                color: "#243647" },
            { ...stroke(), x1: 2000, y1: 700, x2: 2500, y2: 1400,
                color: "#243647" },
        ]);
        preview.dispatch("pointerdown", pointer(20, 20));
        for (let i = 1; i <= 129; i += 1) {
            preview.dispatch("pointermove", pointer(20 + (i % 2) * 10, 20));
        }
        preview.dispatch("pointerup", pointer(200, 200));
        assert.match(get("status").textContent, /128 sampled/);
        assert.equal(preview.children.length, 3,
            "overlong gesture must not partially append segments");
    } finally {
        ui.restore();
    }
});

test("copy creates an independent exact-glyph shape", async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        get("copy").dispatch("click");
        assert.equal(get("preview").children.length, 0);
        editor.start();
        get("add").dispatch("click");
        get("character").value = "n\u{303}";
        get("character").dispatch("input");
        get("copy-from").value = "ñ";
        get("copy").dispatch("click");
        assert.equal(get("preview").children.length, 1);
        assert.match(get("status").textContent, /Copied 1 strokes/);
        get("selected").value = "1";
        get("selected").dispatch("change");
        get("x1").value = "600";
        get("update").dispatch("click");
        get("save").dispatch("click");
        assert.deepEqual(parseGlyphLibrary(await downloads[0].text()), [
            { ...stroke(), color: "#243647" },
            { ...stroke("n\u{303}"), x1: 600, color: "#243647" },
        ]);
        get("copy-from").value = "n\u{303}";
        get("copy").dispatch("click");
        assert.match(get("status").textContent, /different exact graphemes/);
        get("copy-from").value = "∞";
        get("copy").dispatch("click");
        assert.match(get("status").textContent, /no authored strokes/);
        get("copy-from").value = "ñ";
        editor.reset();
        assert.equal(get("copy-from").value, "");
        assert.equal(get("copy").disabled, true);
    } finally {
        ui.restore();
    }
});

test("copy replaces target strokes without touching neighbors", async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        editor.start();
        const original = [
            stroke("ñ"),
            { ...stroke("n\u{303}"), x1: 500, color: "#334455" },
            stroke("∞"),
            { ...stroke("n\u{303}"), x1: 700, color: "#556677" },
        ];
        const bytes = new TextEncoder().encode(encodeGlyphLibrary(original));
        get("load").files = [{
            size: bytes.length, arrayBuffer: async () => bytes.buffer,
        }];
        await get("load").dispatch("change");
        get("character").value = "n\u{303}";
        get("character").dispatch("input");
        get("copy-from").value = "ñ";
        get("copy").dispatch("click");
        assert.equal(get("preview").children.length, 1);
        get("save").dispatch("click");
        assert.deepEqual(parseGlyphLibrary(await downloads[0].text()), [
            original[0], original[2],
            { ...original[0], grapheme: "n\u{303}" },
        ]);
    } finally {
        ui.restore();
    }
});


test("stroke reversal and reorder affect only the chosen glyph", async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        editor.start();
        const original = [
            { ...stroke("o"), x1: 100 },
            { ...stroke("ñ"), x1: 150 },
            { ...stroke("o"), x1: 200 },
            { ...stroke("ñ"), x1: 250 },
            { ...stroke("o"), x1: 300 },
        ];
        const bytes = new TextEncoder().encode(encodeGlyphLibrary(original));
        get("load").files = [{
            size: bytes.length, arrayBuffer: async () => bytes.buffer,
        }];
        await get("load").dispatch("change");
        get("character").value = "o";
        get("character").dispatch("input");
        assert.equal(get("reverse").disabled, true);
        get("selected").value = "2";
        get("selected").dispatch("change");
        assert.equal(get("reverse").disabled, false);
        assert.equal(get("earlier").disabled, false);
        assert.equal(get("later").disabled, false);
        get("reverse").dispatch("click");
        assert.equal(get("x1").value, "3000");
        assert.equal(get("y1").value, "4200");
        assert.equal(get("x2").value, "200");
        get("earlier").dispatch("click");
        assert.equal(get("selected").value, "0");
        assert.equal(get("earlier").disabled, true);
        get("earlier").dispatch("click");
        get("later").dispatch("click");
        assert.equal(get("selected").value, "2");
        get("save").dispatch("click");
        assert.deepEqual(parseGlyphLibrary(await downloads[0].text()), [
            original[0], original[1],
            { ...original[2], x1: 3000, y1: 4200, x2: 200, y2: 200 },
            original[3], original[4],
        ]);
        editor.reset();
        assert.equal(get("paths").textContent,
            "Requires a compatible backend session");
        assert.equal(get("reverse").disabled, true);
        assert.equal(get("earlier").disabled, true);
        assert.equal(get("later").disabled, true);
    } finally {
        ui.restore();
    }
});


test("closed candidate loop can change start without changing ink", () => {
    const nfd = "o\u{301}";
    const a = { ...stroke(nfd), x1: 400, y1: 500,
        x2: 1400, y2: 500 };
    const b = { ...a, x1: 1400, y1: 500,
        x2: 1400, y2: 1500 };
    const c = { ...a, x1: 1400, y1: 1500,
        x2: 400, y2: 1500 };
    const d = { ...a, x1: 400, y1: 1500,
        x2: 400, y2: 500 };
    const other = stroke("ó");
    const math = stroke("∞");
    const accent = { ...a, x1: 2000, y1: 2200,
        x2: 2100, y2: 2700, color: "#112233" };
    const original = [a, other, b, math, c, d, accent];
    const saved = encodeGlyphLibrary(original);
    const changed = rotateClosedCandidatePath(original, nfd, 4);
    assert.equal(changed.selectedIndex, 0);
    assert.deepEqual(changed.strokes, [c, other, d, math, a, b, accent]);
    assert.equal(encodeGlyphLibrary(original), saved,
        "authored library must remain immutable");
    assert.deepEqual(candidatePenPaths(changed.strokes, nfd), [
        { first: 1, last: 4, start: { x: 1400, y: 1500 },
            end: { x: 1400, y: 1500 }, width: 250,
            color: "#A1B2C3" },
        { first: 5, last: 5, start: { x: 2000, y: 2200 },
            end: { x: 2100, y: 2700 }, width: 250,
            color: "#112233" },
    ]);
    const restored = rotateClosedCandidatePath(changed.strokes, nfd, 4);
    assert.deepEqual(restored.strokes, original,
        "same half-loop rotation is exactly reversible");
    for (const index of [-1, 0, 1, 6, 9]) {
        assert.throws(() => rotateClosedCandidatePath(original, nfd, index),
            /selected|later selected/);
    }
    const open = original.slice();
    open[5] = { ...d, x2: 600, y2: 600 };
    assert.throws(() => rotateClosedCandidatePath(open, nfd, 4),
        /closed pen loop/);
    assert.deepEqual(open[5], { ...d, x2: 600, y2: 600 },
        "failed open-path rotation does not mutate a source");

    const second = [accent, other, a, math, b, c, d];
    const secondBytes = encodeGlyphLibrary(second);
    const shiftedSecond = rotateClosedCandidatePath(second, nfd, 5);
    assert.equal(shiftedSecond.selectedIndex, 2);
    assert.deepEqual(shiftedSecond.strokes,
        [accent, other, c, math, d, a, b],
        "path 2 moves only its own closed segments");
    assert.deepEqual(candidatePenPaths(shiftedSecond.strokes, nfd)
        .map((path) => [path.first, path.last]),
        [[1, 1], [2, 5]]);
    assert.deepEqual(rotateClosedCandidatePath(
        shiftedSecond.strokes, nfd, 5,
    ).strokes, second, "path 2 inverse returns exact original ink");
    assert.equal(encodeGlyphLibrary(second), secondBytes);
    assert.throws(() => rotateClosedCandidatePath(second, nfd, 0),
        /later selected start stroke/,
        "the isolated path before a loop stays ineligible");
});

test("browser chooses closed-loop start using explicit UI", async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        editor.start();
        const nfd = "o\u{301}";
        const a = { ...stroke(nfd), x1: 400, y1: 500,
            x2: 1400, y2: 500 };
        const b = { ...a, x1: 1400, y1: 500,
            x2: 1400, y2: 1500 };
        const c = { ...a, x1: 1400, y1: 1500,
            x2: 400, y2: 1500 };
        const d = { ...a, x1: 400, y1: 1500,
            x2: 400, y2: 500 };
        const other = stroke("ó");
        const source = [a, other, b, c, d];
        const bytes = new TextEncoder().encode(encodeGlyphLibrary(source));
        get("load").files = [{
            size: bytes.length, arrayBuffer: async () => bytes.buffer,
        }];
        await get("load").dispatch("change");
        get("character").value = nfd;
        get("character").dispatch("input");
        assert.equal(get("path-start").disabled, true);
        get("selected").value = "3";
        get("selected").dispatch("change");
        assert.equal(get("path-start").disabled, false);
        get("path-start").dispatch("click");
        assert.match(get("status").textContent,
            /Changed closed pen loop starting stroke; unsaved/);
        assert.equal(get("selected").value, "0",
            "new pen entry is selected in first exact-glyph slot");
        assert.equal(get("path-start").disabled, true,
            "entry cannot be moved to itself");
        assert.match(get("paths").textContent,
            /1 candidate pen path, 0 apparent pen lifts/);
        assert.match(get("paths").textContent,
            /Path 1: strokes 1–4, \(1400, 1500\) → \(1400, 1500\)/);
        assert.match(get("paths").textContent, /\[closed loop\]/);
        get("save").dispatch("click");
        assert.deepEqual(parseGlyphLibrary(await downloads[0].text()),
            [c, other, d, a, b]);
        get("character").value = "ó";
        get("character").dispatch("input");
        get("selected").value = "1";
        get("selected").dispatch("change");
        assert.equal(get("path-start").disabled, true,
            "one isolated stroke is not a closed multi-stroke loop");
        editor.reset();
        assert.equal(get("path-start").disabled, true);
    } finally {
        ui.restore();
    }
});

test("explicit path closure preserves Unicode rows and refuses gaps", () => {
    const nfd = "o\u{301}";
    const ink = (grapheme, x1, y1, x2, y2, color = "#243647") =>
        ({ grapheme, x1, y1, x2, y2, width: 250, color });
    const accent = ink(nfd, 2000, 2000, 2200, 2300, "#112233");
    const a = ink(nfd, 400, 500, 1400, 500);
    const b = ink(nfd, 1400, 500, 1400, 1500);
    const c = ink(nfd, 1400, 1500, 600, 1500);
    const other = ink("ó", 900, 800, 1000, 1100);
    const math = ink("∞", 900, 1000, 1100, 1300);
    const original = [accent, other, a, math, b, c];
    const originalBytes = encodeGlyphLibrary(original);
    const closed = closeCandidatePath(original, nfd, 4);
    const bridge = ink(nfd, 600, 1500, 400, 500);
    assert.deepEqual(closed.strokes,
        [accent, other, a, math, b, c, bridge]);
    assert.equal(closed.selectedIndex, 6);
    assert.deepEqual(candidatePenPaths(closed.strokes, nfd)
        .map((path) => [path.first, path.last, path.start, path.end]), [
        [1, 1, { x: 2000, y: 2000 }, { x: 2200, y: 2300 }],
        [2, 5, { x: 400, y: 500 }, { x: 400, y: 500 }],
    ]);
    assert.equal(encodeGlyphLibrary(original), originalBytes,
        "browser source remains byte-identical after closure");
    for (const [glyph, index] of [[nfd, 0], [nfd, 1],
        [nfd, 40], ["ó", 1], ["oo", 2], [nfd, -1]]) {
        assert.throws(() => closeCandidatePath(original, glyph, index));
    }
    assert.throws(() => closeCandidatePath(closed.strokes, nfd, 6),
        /already closed/);
    const wide = original.slice();
    wide[5] = { ...c, x2: 2000, y2: 2000 };
    assert.throws(() => closeCandidatePath(wide, nfd, 4),
        /1200 µm/);
    const edge = [ink("o", 500, 800, 900, 1200),
        ink("o", 900, 1200, 1700, 800)];
    assert.deepEqual(closeCandidatePath(edge, "o", 0).strokes.at(-1),
        ink("o", 1700, 800, 500, 800),
        "inclusive 1200 µm edge matches native behavior");
    const clipped = [ink("o", 0, 800, 900, 1200),
        ink("o", 900, 1200, 500, 800)];
    const clippedBytes = encodeGlyphLibrary(clipped);
    assert.throws(() => closeCandidatePath(clipped, "o", 0),
        /pen bridge clips glyph cell/,
        "centerline-valid imported ink cannot create a clipped bridge");
    assert.equal(encodeGlyphLibrary(clipped), clippedBytes);
    const tangent = [ink("o", 125, 800, 900, 1200),
        ink("o", 900, 1200, 500, 800)];
    assert.deepEqual(closeCandidatePath(tangent, "o", 0).strokes.at(-1),
        ink("o", 500, 800, 125, 800),
        "an inserted round pen may be exactly tangent to the cell edge");
    assert.throws(() => closeCandidatePath([
        edge[0], { ...edge[1], x2: 1701 },
    ], "o", 0), /1200 µm/);
    const full = Array.from({ length: 4096 }, (_, i) =>
        ink("o", 500 + i % 2, 800 + Math.floor(i / 4),
            500 + (i + 1) % 2, 800 + Math.floor((i + 1) / 4)));
    assert.throws(() => closeCandidatePath(full, "o", 0),
        /4096 authored strokes/);
    assert.equal(encodeGlyphLibrary(original), originalBytes);
});

test("join-next preserves neighboring same-pen paths and Unicode", () => {
    const key = "n\u0303";
    const ink = (grapheme, x1, y1, x2, y2) =>
        ({ grapheme, x1, y1, x2, y2, width: 250,
            color: "#243647" });
    const a = ink(key, 500, 700, 850, 750);
    const b = ink(key, 850, 750, 1100, 850);
    const c = ink(key, 1350, 900, 1600, 1050);
    const d = ink(key, 1600, 1050, 1900, 1150);
    const other = ink("ñ", 600, 700, 650, 1000);
    const math = ink("∞", 500, 300, 600, 400);
    const original = [a, other, b, math, c, d];
    const originalBytes = encodeGlyphLibrary(original);
    const joined = joinNextCandidatePath(original, key, 2);
    const bridge = ink(key, 1100, 850, 1350, 900);
    assert.deepEqual(joined.strokes, [a, other, b, bridge, math, c, d]);
    assert.equal(joined.selectedIndex, 3);
    assert.deepEqual(candidatePenPaths(joined.strokes, key)
        .map((path) => [path.first, path.last]), [[1, 5]]);
    assert.equal(encodeGlyphLibrary(original), originalBytes);
    for (const [glyph, selected] of [[key, 5], [key, -1],
        ["ñ", 1], [key, 99], ["nn", 2]]) {
        assert.throws(() => joinNextCandidatePath(original, glyph, selected));
    }
    assert.throws(() => joinNextCandidatePath([
        a, b, { ...c, width: 300 }, d,
    ], key, 0), /matching pen/);
    assert.throws(() => joinNextCandidatePath([
        a, b, { ...c, x1: 3400 }, d,
    ], key, 0), /1200/);
    const exactLimit = [ink(key, 500, 700, 800, 850),
        ink(key, 2000, 850, 2200, 950)];
    assert.deepEqual(joinNextCandidatePath(exactLimit, key, 0)
        .strokes[1], ink(key, 800, 850, 2000, 850),
        "an exactly 1200 µm gap is admitted");
    assert.throws(() => joinNextCandidatePath([
        exactLimit[0], { ...exactLimit[1], x1: 2001 },
    ], key, 0), /1200/,
    "1201 µm cannot be rounded down to a permitted join");
    const tangent = [ink(key, 300, 700, 125, 850),
        ink(key, 500, 850, 650, 950)];
    assert.deepEqual(joinNextCandidatePath(tangent, key, 0).strokes[1],
        ink(key, 125, 850, 500, 850),
        "round pen tangent to x=0 is permitted without clipping");
    const edge = [ink(key, 250, 700, 50, 850),
        ink(key, 250, 850, 450, 950)];
    assert.throws(() => joinNextCandidatePath(edge, key, 0),
        /pen bridge clips glyph cell/);
    const atLimit = [a, b, c, d,
        ...Array.from({ length: 4092 }, () => math)];
    assert.equal(atLimit.length, 4096);
    assert.throws(() => joinNextCandidatePath(atLimit, key, 0),
        /4096 authored strokes/);
    assert.equal(encodeGlyphLibrary(atLimit).startsWith(originalBytes.slice(
        0, originalBytes.indexOf("\n") + 1)), true);
});

test("curved pen joins sample quadratic coordinates and refuse cliffs", () => {
    const key = "n\u0303";
    const ink = (grapheme, x1, y1, x2, y2, width = 250) =>
        ({ grapheme, x1, y1, x2, y2, width, color: "#243647" });
    const a = ink(key, 500, 700, 850, 750);
    const b = ink(key, 850, 750, 1100, 850);
    const c = ink(key, 1350, 900, 1600, 1050);
    const other = ink("ñ", 600, 700, 650, 1000);
    const original = [a, other, b, c];
    const bytes = encodeGlyphLibrary(original);
    const changed = joinNextCandidatePathCurved(original, key, 2, 0, -200);
    assert.deepEqual(changed.strokes, [a, other, b,
        ink(key, 1100, 850, 1163, 788),
        ink(key, 1163, 788, 1225, 775),
        ink(key, 1225, 775, 1288, 813),
        ink(key, 1288, 813, 1350, 900), c]);
    assert.equal(changed.selectedIndex, 3);
    assert.equal(candidatePenPaths(changed.strokes, key).length, 1);
    assert.equal(encodeGlyphLibrary(original), bytes);
    assert.equal(encodeGlyphLibrary(parseGlyphLibrary(
        encodeGlyphLibrary(changed.strokes))),
    encodeGlyphLibrary(changed.strokes));
    for (const [selected, bendX, bendY] of [
        [2, 0, 0], [2, 601, 0], [2, 0, -601], [2, 0.25, 50],
        [2, NaN, 50], [2, 250, 50], [3, 0, -200], [-1, 0, -200],
    ]) {
        assert.throws(() => joinNextCandidatePathCurved(
            original, key, selected, bendX, bendY));
        assert.equal(encodeGlyphLibrary(original), bytes);
    }
    assert.throws(() => joinNextCandidatePathCurved([
        a, b, { ...c, width: 300 },
    ], key, 0, 0, -200), /matching pen/);
    assert.throws(() => joinNextCandidatePathCurved([
        a, b, { ...c, x1: 3400 },
    ], key, 0, 0, -200), /1200/);
    const edge = [ink(key, 250, 700, 125, 850),
        ink(key, 500, 850, 650, 950)];
    assert.throws(() => joinNextCandidatePathCurved(
        edge, key, 0, -600, 50), /clips glyph cell/);
    const near = [a, b, c,
        ...Array.from({ length: 4089 }, () => other)];
    assert.equal(near.length, 4092);
    const filled = joinNextCandidatePathCurved(
        near, key, 0, 0, -200);
    assert.equal(filled.strokes.length, 4096,
        "four new pieces exactly fill the native stroke budget");
    assert.equal(candidatePenPaths(filled.strokes, key).length, 1);
    const full = [...near, other];
    assert.equal(full.length, 4093);
    assert.throws(() => joinNextCandidatePathCurved(
        full, key, 0, 0, -200), /4096 authored strokes/);
});

test("curved path UI records one undoable deliberate TSV change", async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        const key = "o\u0301";
        const ink = (grapheme, x1, y1, x2, y2) =>
            ({ grapheme, x1, y1, x2, y2, width: 250,
                color: "#243647" });
        const source = [ink(key, 500, 700, 850, 750),
            ink("ó", 600, 700, 650, 1000),
            ink(key, 850, 750, 1100, 850),
            ink(key, 1350, 900, 1600, 1050)];
        const original = encodeGlyphLibrary(source);
        const bytes = new TextEncoder().encode(original);
        assert.equal(get("path-join-curved").disabled, true);
        editor.start();
        get("load").files = [{ size: bytes.length,
            arrayBuffer: async () => bytes.buffer }];
        await get("load").dispatch("change");
        get("character").value = key;
        get("character").dispatch("input");
        get("selected").value = "2";
        get("selected").dispatch("change");
        get("path-bend-x").value = "601";
        get("path-bend-y").value = "-200";
        assert.equal(get("path-join-curved").disabled, false);
        get("path-join-curved").dispatch("click");
        assert.match(get("status").textContent, /outside native bounds/);
        assert.equal(downloads.length, 0);
        assert.match(get("paths").textContent, /2 candidate pen paths/);
        assert.equal(get("slant-undo").disabled, true,
            "invalid bends do not record history");
        get("path-bend-x").value = "0";
        get("path-join-curved").dispatch("click");
        assert.match(get("status").textContent, /four curved segments/);
        assert.equal(get("selected").value, "3");
        assert.match(get("paths").textContent, /1 candidate pen path/);
        assert.equal(downloads.length, 0);
        const joined = encodeGlyphLibrary(joinNextCandidatePathCurved(
            source, key, 2, 0, -200).strokes);
        get("save").dispatch("click");
        assert.equal(await downloads[0].text(), joined);
        get("slant-undo").dispatch("click");
        get("save").dispatch("click");
        assert.equal(await downloads[1].text(), original);
        get("slant-redo").dispatch("click");
        get("save").dispatch("click");
        assert.equal(await downloads[2].text(), joined);
        editor.reset();
        assert.equal(get("path-join-curved").disabled, true);
    } finally {
        ui.restore();
    }
});

test("join-next browser action is one undoable, explicit TSV change",
    async () => {
        const ui = mountEditor();
        try {
            const { editor, get, downloads } = ui;
            const key = "o\u0301";
            const ink = (grapheme, x1, y1, x2, y2) =>
                ({ grapheme, x1, y1, x2, y2, width: 250,
                    color: "#243647" });
            const a = ink(key, 500, 700, 850, 750);
            const b = ink(key, 850, 750, 1100, 850);
            const c = ink(key, 1350, 900, 1600, 1050);
            const other = ink("ó", 600, 700, 650, 1000);
            const source = [a, other, b, c];
            const bytes = new TextEncoder().encode(encodeGlyphLibrary(source));
            assert.equal(get("path-join-next").disabled, true);
            editor.start();
            get("load").files = [{ size: bytes.length,
                arrayBuffer: async () => bytes.buffer }];
            await get("load").dispatch("change");
            get("character").value = key;
            get("character").dispatch("input");
            get("selected").value = "2";
            get("selected").dispatch("change");
            assert.equal(get("path-join-next").disabled, false);
            get("path-join-next").dispatch("click");
            assert.match(get("status").textContent,
                /Joined neighboring same-pen paths; unsaved/);
            assert.equal(downloads.length, 0);
            assert.equal(get("selected").value, "3",
                "new directed bridge becomes selected");
            assert.match(get("paths").textContent,
                /1 candidate pen path, 0 apparent pen lifts/);
            assert.equal(get("path-join-next").disabled, true);
            const joined = joinNextCandidatePath(source, key, 2).strokes;
            get("save").dispatch("click");
            assert.equal(await downloads[0].text(),
                encodeGlyphLibrary(joined));
            get("slant-undo").dispatch("click");
            get("save").dispatch("click");
            assert.equal(await downloads[1].text(),
                encodeGlyphLibrary(source));
            get("slant-redo").dispatch("click");
            get("save").dispatch("click");
            assert.equal(await downloads[2].text(),
                encodeGlyphLibrary(joined));
            editor.reset();
            assert.equal(get("path-join-next").disabled, true);
        } finally {
            ui.restore();
        }
    });

test("browser UI closes path and offers loop entry", async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        editor.start();
        const nfd = "o\u{301}";
        const ink = (grapheme, x1, y1, x2, y2) =>
            ({ grapheme, x1, y1, x2, y2, width: 250,
                color: "#243647" });
        const a = ink(nfd, 400, 500, 1400, 500);
        const b = ink(nfd, 1400, 500, 1400, 1500);
        const c = ink(nfd, 1400, 1500, 600, 1500);
        const other = ink("ó", 700, 800, 800, 900);
        const source = [a, other, b, c];
        const bytes = new TextEncoder().encode(encodeGlyphLibrary(source));
        get("load").files = [{
            size: bytes.length, arrayBuffer: async () => bytes.buffer,
        }];
        await get("load").dispatch("change");
        get("character").value = nfd;
        get("character").dispatch("input");
        assert.equal(get("path-close").disabled, true,
            "no selected path is not actionable");
        get("selected").value = "2";
        get("selected").dispatch("change");
        assert.equal(get("path-close").disabled, false);
        assert.equal(get("path-start").disabled, true);
        get("path-close").dispatch("click");
        assert.equal(get("selected").value, "4",
            "the newly added connector is selected");
        assert.equal(get("path-close").disabled, true,
            "closed paths cannot be closed again");
        assert.equal(get("path-start").disabled, false,
            "after closure the selected later entry can rotate");
        assert.match(get("paths").textContent, /\[closed loop\]/);
        get("save").dispatch("click");
        assert.deepEqual(parseGlyphLibrary(await downloads[0].text()), [
            a, other, b, c, ink(nfd, 600, 1500, 400, 500),
        ]);
        get("character").value = "ó";
        get("character").dispatch("input");
        get("selected").value = "1";
        get("selected").dispatch("change");
        assert.equal(get("path-close").disabled, true);
        editor.reset();
        assert.equal(get("path-close").disabled, true);
    } finally {
        ui.restore();
    }
});

test("browser reverses whole selected pen path and retains TSV rows",
    async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        editor.start();
        const nfd = "n\u{303}";
        const a = { ...stroke(nfd), x1: 500, y1: 600,
            x2: 1100, y2: 1400 };
        const b = { ...stroke(nfd), x1: 1100, y1: 1400,
            x2: 1800, y2: 2100 };
        const c = { ...stroke(nfd), x1: 1800, y1: 2100,
            x2: 2200, y2: 2600 };
        const other = stroke("ñ");
        const source = [a, other, b, c];
        const bytes = new TextEncoder().encode(encodeGlyphLibrary(source));
        get("load").files = [{
            size: bytes.length, arrayBuffer: async () => bytes.buffer,
        }];
        await get("load").dispatch("change");
        get("character").value = nfd;
        get("character").dispatch("input");
        assert.equal(get("reverse-path").disabled, true);
        get("selected").value = "0";
        get("selected").dispatch("change");
        assert.equal(get("reverse-path").disabled, false);
        get("reverse-path").dispatch("click");
        assert.match(get("status").textContent,
            /Reversed selected continuous pen path/);
        assert.equal(get("selected").value, "3",
            "selected physical segment follows reverse path order");
        assert.equal(get("x1").value, "1100");
        assert.equal(get("y1").value, "1400");
        assert.match(get("paths").textContent,
            /3 directed strokes, 1 candidate pen path, 0 apparent pen lifts/);
        assert.match(get("paths").textContent,
            /Path 1: strokes 1–3, \(2200, 2600\) → \(500, 600\)/);
        get("save").dispatch("click");
        const flipped = (line) => ({
            ...line, x1: line.x2, y1: line.y2,
            x2: line.x1, y2: line.y1,
        });
        assert.deepEqual(parseGlyphLibrary(await downloads[0].text()),
            [flipped(c), other, flipped(b), flipped(a)]);
        get("reverse-path").dispatch("click");
        assert.equal(get("selected").value, "0");
        get("save").dispatch("click");
        assert.equal(await downloads[1].text(), encodeGlyphLibrary(source));
        get("character").value = "ñ";
        get("character").dispatch("input");
        get("selected").value = "1";
        get("selected").dispatch("change");
        assert.equal(get("reverse-path").disabled, true,
            "a single isolated stroke is not a continuous path of two");
        editor.reset();
        assert.equal(get("reverse-path").disabled, true);
    } finally {
        ui.restore();
    }
});

test("browser translates selected whole path without saving invalid ink",
    async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        const nfd = "n\u{303}";
        const a = { ...stroke(nfd), x1: 500, y1: 600,
            x2: 1000, y2: 1300 };
        const b = { ...a, x1: 1000, y1: 1300,
            x2: 1400, y2: 1800 };
        const accent = { ...a, x1: 1800, y1: 3000,
            x2: 2300, y2: 3500, color: "#334455" };
        const other = stroke("ñ");
        const original = [a, other, b, accent];
        const bytes = new TextEncoder().encode(encodeGlyphLibrary(original));
        assert.equal(get("path-translate").disabled, true);
        editor.start();
        get("load").files = [{
            size: bytes.length, arrayBuffer: async () => bytes.buffer,
        }];
        await get("load").dispatch("change");
        get("character").value = nfd;
        get("character").dispatch("input");
        get("selected").value = "2";
        get("selected").dispatch("change");
        assert.equal(get("path-translate").disabled, false);
        get("path-dx").value = "250";
        get("path-dy").value = "-300";
        get("path-translate").dispatch("click");
        assert.match(get("status").textContent,
            /Translated selected continuous pen path; unsaved/);
        assert.equal(get("selected").value, "2");
        assert.match(get("paths").textContent,
            /Path 1: strokes 1–2, \(750, 300\) → \(1650, 1500\)/);
        get("save").dispatch("click");
        const changed = translateCandidatePath(original, nfd, 2, 250, -300);
        assert.deepEqual(parseGlyphLibrary(await downloads[0].text()),
            changed.strokes);
        for (const [dx, dy] of [
            ["0", "0"], ["-1000", "0"], ["1.5", "0"],
            ["3501", "0"], ["0", "-5000"],
        ]) {
            get("path-dx").value = dx;
            get("path-dy").value = dy;
            get("path-translate").dispatch("click");
            assert.doesNotMatch(get("status").textContent, /; unsaved/);
        }
        get("save").dispatch("click");
        assert.deepEqual(parseGlyphLibrary(await downloads[1].text()),
            changed.strokes, "invalid attempts do not update the draft");
        get("path-dx").value = "-250";
        get("path-dy").value = "300";
        get("path-translate").dispatch("click");
        get("save").dispatch("click");
        assert.equal(await downloads[2].text(),
            encodeGlyphLibrary(original), "inverse translation is exact");
        get("character").value = "ñ";
        get("character").dispatch("input");
        assert.equal(get("path-translate").disabled, true);
        editor.reset();
        assert.equal(get("path-translate").disabled, true);
        assert.equal(get("path-dx").disabled, true);
        assert.equal(get("path-dy").disabled, true);
    } finally {
        ui.restore();
    }
});

test("browser path copying is undoable and preserves prior target ink",
    async () => {
        const ui = mountEditor();
        try {
            const { editor, get, downloads } = ui;
            const nfd = "n\u0303";
            const source = { ...stroke(nfd), x1: 500, y1: 600,
                x2: 1000, y2: 1300 };
            const joined = { ...source, x1: 1000, y1: 1300,
                x2: 1400, y2: 1800 };
            const accent = { ...source, x1: 1800, y1: 3000,
                x2: 2300, y2: 3500, color: "#334455" };
            const other = stroke("ñ");
            const original = [source, other, joined, accent];
            const bytes = new TextEncoder().encode(
                encodeGlyphLibrary(original));
            editor.start();
            get("load").files = [{
                size: bytes.length, arrayBuffer: async () => bytes.buffer,
            }];
            await get("load").dispatch("change");
            get("character").value = nfd;
            get("character").dispatch("input");
            get("selected").value = "2";
            get("selected").dispatch("change");
            assert.equal(get("path-copy").disabled, false);
            get("path-copy-target").value = "ñ";
            get("path-copy").dispatch("click");
            assert.match(get("status").textContent,
                /Copied 2 directed path strokes to ñ \(unsaved\)/);
            assert.equal(get("selected").value, "2");
            get("save").dispatch("click");
            const copied = copyCandidatePath(original, nfd, 2, "ñ");
            assert.deepEqual(parseGlyphLibrary(await downloads[0].text()),
                copied);
            assert.equal(get("slant-undo").disabled, false);
            get("slant-undo").dispatch("click");
            assert.match(get("status").textContent,
                /Undid last pen path copy; unsaved/);
            get("save").dispatch("click");
            assert.equal(await downloads[1].text(),
                encodeGlyphLibrary(original));
            get("slant-redo").dispatch("click");
            assert.match(get("status").textContent,
                /Redid last pen path copy; unsaved/);
            get("save").dispatch("click");
            assert.equal(await downloads[2].text(),
                encodeGlyphLibrary(copied));
            for (const invalidTarget of [nfd, "ab", " ", "\n"]) {
                get("path-copy-target").value = invalidTarget;
                get("path-copy").dispatch("click");
                assert.doesNotMatch(get("status").textContent,
                    /Copied .*unsaved/);
            }
            get("save").dispatch("click");
            assert.equal(await downloads[3].text(),
                encodeGlyphLibrary(copied));
            get("path-copy-target").value = "á";
            editor.reset();
            assert.equal(get("path-copy-target").value, "");
            assert.equal(get("path-copy").disabled, true);
            editor.start();
            assert.equal(get("path-copy").disabled, true);
        } finally {
            ui.restore();
        }
    });

test("shifted path copy UI downloads exact ink and supports recovery",
    async () => {
        const ui = mountEditor();
        try {
            const { editor, get, downloads } = ui;
            editor.start();
            const nfd = "n\u0303";
            const a = { ...stroke(nfd), x1: 500, y1: 600,
                x2: 900, y2: 1200 };
            const b = { ...a, x1: 900, y1: 1200,
                x2: 1400, y2: 1900 };
            const original = [a, stroke("ñ"), b, stroke("∞")];
            const bytes = new TextEncoder().encode(
                encodeGlyphLibrary(original));
            get("load").files = [{ size: bytes.length,
                arrayBuffer: async () => bytes.buffer }];
            await get("load").dispatch("change");
            get("character").value = nfd;
            get("character").dispatch("input");
            get("selected").value = "2";
            get("selected").dispatch("change");
            assert.equal(get("path-copy-shifted").disabled, false);
            get("path-copy-target").value = nfd;
            get("path-dx").value = "100";
            get("path-dy").value = "200";
            get("path-copy-shifted").dispatch("click");
            assert.match(get("status").textContent,
                /Copied 2 shifted path strokes/);
            get("save").dispatch("click");
            const expected = copyShiftedCandidatePath(
                original, nfd, 2, nfd, 100, 200);
            assert.equal(await downloads[0].text(),
                encodeGlyphLibrary(expected));
            get("slant-undo").dispatch("click");
            assert.match(get("status").textContent,
                /Undid last pen path shifted copy/);
            get("save").dispatch("click");
            assert.equal(await downloads[1].text(),
                encodeGlyphLibrary(original));
            get("slant-redo").dispatch("click");
            get("save").dispatch("click");
            assert.equal(await downloads[2].text(),
                encodeGlyphLibrary(expected));
            get("path-dx").value = "0";
            get("path-dy").value = "0";
            get("path-copy-shifted").dispatch("click");
            assert.match(get("status").textContent, /nonzero/);
            get("save").dispatch("click");
            assert.equal(await downloads[3].text(),
                encodeGlyphLibrary(expected));
            editor.reset();
            assert.equal(get("path-copy-shifted").disabled, true);
        } finally {
            ui.restore();
        }
    });

test("whole candidate path pen style preserves exact ink and refuses clipping",
    () => {
        const nfd = "n\u0303";
        const a = { ...stroke(nfd), x1: 500, y1: 600,
            x2: 900, y2: 1200, width: 251, color: "#112233" };
        const b = { ...a, x1: 900, y1: 1200,
            x2: 1400, y2: 1900 };
        const tail = { ...a, x1: 1800, y1: 2400,
            x2: 2200, y2: 2800, width: 200, color: "#445566" };
        const other = stroke("ñ");
        const source = [a, other, b, stroke("∞"), tail];
        const original = encodeGlyphLibrary(source);
        const changed = styleCandidatePath(source, nfd, 2, 501, "#abCDef");
        assert.equal(changed.selectedIndex, 2);
        assert.deepEqual(changed.strokes, [
            { ...a, width: 501, color: "#ABCDEF" },
            other, { ...b, width: 501, color: "#ABCDEF" },
            source[3], tail,
        ]);
        assert.equal(candidatePenPaths(changed.strokes, nfd).length, 2);
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(changed.strokes)),
            changed.strokes);
        const tailOnly = styleCandidatePath(source, nfd, 4,
            125, "#010203");
        assert.deepEqual(tailOnly.strokes.slice(0, 4), source.slice(0, 4));
        assert.equal(tailOnly.strokes[4].width, 125);
        for (const [index, width, color] of [
            [-1, 250, "#123456"], [1, 250, "#123456"],
            [2, 79, "#123456"], [2, 1501, "#123456"],
            [2, 250.5, "#123456"], [2, 250, "123456"],
            [2, 250, "#GGGGGG"], [2, 1500, "#123456"],
            [2, 251, "#112233"],
        ]) {
            assert.throws(() => styleCandidatePath(
                source, nfd, index, width, color),
            /selected|80|clip|#RRGGBB|already/);
        }
        assert.equal(encodeGlyphLibrary(source), original,
            "accepted and refused transformations cannot mutate source");
        const full = Array.from({ length: 4096 }, (_, index) => ({
            ...a, x1: 500 + index % 100,
            y1: 600, x2: 900, y2: 1200,
        }));
        const fullResult = styleCandidatePath(full, nfd, 0, 501, "#ABCDEF");
        assert.equal(fullResult.strokes.length, 4096);
        assert.equal(full[0].width, 251);
    });

test("selected path style UI downloads only deliberately styled ink",
    async () => {
        const ui = mountEditor();
        try {
            const { editor, get, downloads } = ui;
            const nfd = "n\u0303";
            const a = { ...stroke(nfd), x1: 500, y1: 600,
                x2: 900, y2: 1200, width: 251, color: "#112233" };
            const b = { ...a, x1: 900, y1: 1200,
                x2: 1400, y2: 1900 };
            const original = [a, stroke("ñ"), b, stroke("∞")];
            const bytes = new TextEncoder().encode(
                encodeGlyphLibrary(original));
            assert.equal(get("path-style").disabled, true);
            editor.start();
            get("load").files = [{ size: bytes.length,
                arrayBuffer: async () => bytes.buffer }];
            await get("load").dispatch("change");
            get("character").value = nfd;
            get("character").dispatch("input");
            get("selected").value = "2";
            get("selected").dispatch("change");
            assert.equal(get("path-style").disabled, false);
            get("width").value = "501";
            get("color").value = "#abcdef";
            get("path-style").dispatch("click");
            assert.match(get("status").textContent,
                /Restyled selected continuous pen path; unsaved/);
            get("save").dispatch("click");
            const changed = styleCandidatePath(original, nfd, 2,
                501, "#abcdef");
            assert.equal(await downloads[0].text(),
                encodeGlyphLibrary(changed.strokes));
            get("slant-undo").dispatch("click");
            assert.match(get("status").textContent,
                /Undid last pen path pen style/);
            get("save").dispatch("click");
            assert.equal(await downloads[1].text(),
                encodeGlyphLibrary(original));
            get("slant-redo").dispatch("click");
            get("save").dispatch("click");
            assert.equal(await downloads[2].text(),
                encodeGlyphLibrary(changed.strokes));
            get("width").value = "1500";
            get("color").value = "#111111";
            get("path-style").dispatch("click");
            assert.match(get("status").textContent, /clip/);
            get("save").dispatch("click");
            assert.equal(await downloads[3].text(),
                encodeGlyphLibrary(changed.strokes));
            editor.reset();
            assert.equal(get("path-style").disabled, true);
        } finally {
            ui.restore();
        }
    });

test("remove candidate path preserves interleaved NFC/NFD and nonempty TSV",
    () => {
        const nfd = "n\u0303";
        const a = { ...stroke(nfd), x1: 500, y1: 600,
            x2: 900, y2: 1200, width: 251, color: "#112233" };
        const b = { ...a, x1: 900, y1: 1200,
            x2: 1400, y2: 1900 };
        const tail = { ...a, x1: 1800, y1: 2400,
            x2: 2200, y2: 2800, width: 200, color: "#445566" };
        const other = stroke("ñ");
        const source = [a, other, b, stroke("∞"), tail];
        const before = encodeGlyphLibrary(source);
        assert.deepEqual(removeCandidatePath(source, nfd, 2),
            [other, source[3], tail]);
        assert.deepEqual(removeCandidatePath(source, nfd, 4),
            source.slice(0, 4));
        assert.deepEqual(removeCandidatePath(source, "ñ", 1),
            [a, b, source[3], tail]);
        assert.equal(encodeGlyphLibrary(source), before);
        for (const index of [-1, 1, 9999]) {
            assert.throws(() => removeCandidatePath(source, nfd, index),
                /selected exact glyph/);
        }
        assert.throws(() => removeCandidatePath([a, b], nfd, 0),
            /final authored pen path/);
    });

test("UI removes only selected path and recovers it with bounded undo",
    async () => {
        const ui = mountEditor();
        try {
            const { editor, get, downloads } = ui;
            const nfd = "n\u0303";
            const a = { ...stroke(nfd), x1: 500, y1: 600,
                x2: 900, y2: 1200, width: 251, color: "#112233" };
            const b = { ...a, x1: 900, y1: 1200,
                x2: 1400, y2: 1900 };
            const source = [a, stroke("ñ"), b, stroke("∞")];
            const bytes = new TextEncoder().encode(
                encodeGlyphLibrary(source));
            assert.equal(get("path-remove").disabled, true);
            editor.start();
            get("load").files = [{ size: bytes.length,
                arrayBuffer: async () => bytes.buffer }];
            await get("load").dispatch("change");
            get("character").value = nfd;
            get("character").dispatch("input");
            get("selected").value = "2";
            get("selected").dispatch("change");
            assert.equal(get("path-remove").disabled, false);
            get("path-remove").dispatch("click");
            assert.match(get("status").textContent,
                /Removed 2 selected pen path strokes; unsaved/);
            get("save").dispatch("click");
            const revised = removeCandidatePath(source, nfd, 2);
            assert.equal(await downloads[0].text(),
                encodeGlyphLibrary(revised));
            get("slant-undo").dispatch("click");
            get("save").dispatch("click");
            assert.equal(await downloads[1].text(),
                encodeGlyphLibrary(source));
            get("slant-redo").dispatch("click");
            get("save").dispatch("click");
            assert.equal(await downloads[2].text(),
                encodeGlyphLibrary(revised));
            editor.reset();
            assert.equal(get("path-remove").disabled, true);
        } finally {
            ui.restore();
        }
    });

test("browser moves entire paths using selected glyph slots", async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        editor.start();
        const nfd = "n\u{303}";
        const a = { ...stroke(nfd), x1: 100, y1: 200,
            x2: 400, y2: 600 };
        const b = { ...a, x1: 400, y1: 600, x2: 700, y2: 900 };
        const c = { ...a, x1: 1500, y1: 1600,
            x2: 1800, y2: 1900 };
        const other = stroke("ñ");
        const source = [a, other, b, c];
        const bytes = new TextEncoder().encode(encodeGlyphLibrary(source));
        get("load").files = [{
            size: bytes.length, arrayBuffer: async () => bytes.buffer,
        }];
        await get("load").dispatch("change");
        get("character").value = nfd;
        get("character").dispatch("input");
        assert.equal(get("path-earlier").disabled, true);
        assert.equal(get("path-later").disabled, true);
        get("selected").value = "2";
        get("selected").dispatch("change");
        assert.equal(get("path-earlier").disabled, true);
        assert.equal(get("path-later").disabled, false);
        get("path-later").dispatch("click");
        assert.match(get("status").textContent,
            /Reordered selected continuous pen path/);
        assert.equal(get("selected").value, "3",
            "selected segment follows whole-path movement");
        assert.deepEqual(get("selected").children.slice(1)
            .map((item) => item.textContent), [
            "Stroke 1 of 3 (path 1)",
            "Stroke 2 of 3 (path 2)",
            "Stroke 3 of 3 (path 2)",
        ]);
        assert.match(get("paths").textContent,
            /3 directed strokes, 2 candidate pen paths, 1 apparent pen lift/);
        assert.equal(get("path-earlier").disabled, false);
        assert.equal(get("path-later").disabled, true);
        get("save").dispatch("click");
        assert.deepEqual(parseGlyphLibrary(await downloads[0].text()),
            [c, other, a, b]);
        get("path-earlier").dispatch("click");
        assert.equal(get("selected").value, "2");
        get("save").dispatch("click");
        assert.equal(await downloads[1].text(), encodeGlyphLibrary(source));
        get("character").value = "ñ";
        get("character").dispatch("input");
        get("selected").value = "1";
        get("selected").dispatch("change");
        assert.equal(get("path-earlier").disabled, true);
        assert.equal(get("path-later").disabled, true);
        editor.reset();
        assert.equal(get("path-later").disabled, true);
    } finally {
        ui.restore();
    }
});

test("browser joins preserve Unicode rows and export Rust TSV", async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        editor.start();
        const grapheme = "n\u{303}";
        const original = [
            { grapheme, x1: 500, y1: 600, x2: 1100, y2: 1400,
                width: 250, color: "#102030" },
            { grapheme: "ñ", x1: 1000, y1: 1000, x2: 1200,
                y2: 2200, width: 350, color: "#AABBCC" },
            { grapheme, x1: 1500, y1: 1800, x2: 2200, y2: 2600,
                width: 250, color: "#102030" },
        ];
        const bytes = new TextEncoder().encode(encodeGlyphLibrary(original));
        get("load").files = [{
            size: bytes.length, arrayBuffer: async () => bytes.buffer,
        }];
        await get("load").dispatch("change");
        get("character").value = grapheme;
        get("character").dispatch("input");
        assert.match(get("paths").textContent,
            /2 directed strokes, 2 candidate pen paths, 1 apparent pen lift/);
        assert.match(get("paths").textContent,
            /Path 1: strokes 1–1.*Path 2: strokes 2–2/);
        assert.equal(get("selected").children[1].textContent,
            "Stroke 1 of 2 (path 1)");
        assert.equal(get("join").disabled, true,
            "joining requires a deliberate exact-glyph selection");
        get("selected").value = "0";
        get("selected").dispatch("change");
        assert.equal(get("join").disabled, false);
        get("join").dispatch("click");
        assert.match(get("status").textContent, /Connected selected/);
        assert.equal(get("selected").value, "1");
        const bridge = {
            grapheme, x1: 1100, y1: 1400, x2: 1500, y2: 1800,
            width: 250, color: "#102030",
        };
        const expected = [original[0], bridge, original[1], original[2]];
        assert.equal(get("preview").children.length, 3);
        assert.match(get("paths").textContent,
            /3 directed strokes, 1 candidate pen path, 0 apparent pen lifts/);
        assert.match(get("paths").textContent,
            /Path 1: strokes 1–3, \(500, 600\) → \(2200, 2600\)/);
        assert.equal(get("selected").children[2].textContent,
            "Stroke 2 of 3 (path 1)");
        get("save").dispatch("click");
        const exported = parseGlyphLibrary(await downloads[0].text());
        assert.deepEqual(exported, expected);
        assert.equal(get("selected").value, "1");
        get("selected").value = "0";
        get("selected").dispatch("change");
        get("join").dispatch("click");
        assert.match(get("status").textContent, /already continuous/);
        get("save").dispatch("click");
        assert.deepEqual(parseGlyphLibrary(await downloads[1].text()),
            expected, "rejected repeat must not mutate the draft");
    } finally {
        ui.restore();
    }
});

test("browser refuses unsafe joins without mutating authored ink", async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        editor.start();
        const first = { ...stroke("á"), x1: 500, y1: 500,
            x2: 1000, y2: 1000, color: "#102030" };
        const second = { ...stroke("á"), x1: 2200, y1: 1000,
            x2: 2600, y2: 1600, color: "#102030" };
        const invalid = [
            [{ ...first }, { ...second, x1: 2201 }],
            [{ ...first }, { ...second, width: 350 }],
            [{ ...first }, { ...second, color: "#AABBCC" }],
            [{ ...first }, { ...second, x1: 1000, y1: 1000 }],
            [{ ...first, x2: 0 }, { ...second, x1: 500 }],
        ];
        for (const source of invalid) {
            const bytes = new TextEncoder().encode(
                encodeGlyphLibrary(source));
            get("load").files = [{
                size: bytes.length, arrayBuffer: async () => bytes.buffer,
            }];
            await get("load").dispatch("change");
            get("character").value = "á";
            get("character").dispatch("input");
            get("selected").value = "0";
            get("selected").dispatch("change");
            get("join").dispatch("click");
            assert.match(get("status").textContent,
                /(?:1200|matching|already continuous|pen bridge clips)/);
            get("save").dispatch("click");
            const last = downloads.at(-1);
            assert.deepEqual(parseGlyphLibrary(await last.text()), source,
                "rejected join cannot alter any authored segment");
        }
        const valid = new TextEncoder().encode(
            encodeGlyphLibrary([first, second]));
        get("load").files = [{
            size: valid.length, arrayBuffer: async () => valid.buffer,
        }];
        await get("load").dispatch("change");
        get("character").value = "á";
        get("character").dispatch("input");
        get("selected").value = "0";
        get("selected").dispatch("change");
        get("join").dispatch("click");
        assert.match(get("status").textContent, /Connected selected/,
            "inclusive 1200 µm gap remains admissible");
        get("save").dispatch("click");
        assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
            [first, { ...first, x1: 1000, y1: 1000,
                x2: 2200, y2: 1000 }, second]);
        const tangent = [{ ...first, x2: 125 },
            { ...second, x1: 425 }];
        const tangible = new TextEncoder().encode(
            encodeGlyphLibrary(tangent));
        get("load").files = [{
            size: tangible.length, arrayBuffer: async () => tangible.buffer,
        }];
        await get("load").dispatch("change");
        get("character").value = "á";
        get("character").dispatch("input");
        get("selected").value = "0";
        get("selected").dispatch("change");
        get("join").dispatch("click");
        assert.match(get("status").textContent, /Connected selected/);
        get("save").dispatch("click");
        assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
            [tangent[0], { ...first, x1: 125, y1: 1000,
                x2: 425, y2: 1000 }, tangent[1]],
            "inclusive half-width tangent remains a valid new bridge");
    } finally {
        ui.restore();
    }
});

test("geometric starters share loop and validate acute strokes", () => {
    const circle = geometricStarter("o", 1550, 2450, 1100, 250, "#123456");
    const simpleA = geometricStarter("a", 1550, 2450, 1100, 250,
        "#123456");
    assert.equal(circle.length, 48);
    assert.equal(simpleA.length, 49);
    assert.deepEqual(simpleA.slice(0, 48), circle.map((line) => ({
        ...line, grapheme: "a",
    })));
    assert.deepEqual(simpleA.at(-1), {
        grapheme: "a", x1: 2650, y1: 1350, x2: 2650, y2: 3550,
        width: 250, color: "#123456",
    });
    for (let i = 0; i < 48; i += 1) {
        const segment = circle[i];
        const next = circle[(i + 1) % 48];
        assert.equal(segment.x2, next.x1);
        assert.equal(segment.y2, next.y1);
        for (const [x, y] of [
            [segment.x1, segment.y1], [segment.x2, segment.y2],
        ]) {
            assert.ok(Math.abs(Math.hypot(x - 1550, y - 2450) - 1100) < 1,
                "rounded sample lies within 1 µm of ideal circle");
        }
    }
    assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(simpleA)), simpleA);
    const acuteO = geometricStarter("ó", 1550, 2450, 1100, 250,
        "#123456");
    const acuteA = geometricStarter("á", 1550, 2450, 1100, 250,
        "#123456");
    assert.equal(acuteO.length, 49);
    assert.equal(acuteA.length, 50);
    assert.deepEqual(acuteO.slice(0, 48), circle.map((line) => ({
        ...line, grapheme: "ó",
    })));
    assert.deepEqual(acuteA.slice(0, 49), simpleA.map((line) => ({
        ...line, grapheme: "á",
    })));
    const accent = {
        x1: 1165, y1: 980, x2: 1825, y2: 485,
        width: 250, color: "#123456",
    };
    assert.deepEqual(acuteO.at(-1), { ...accent, grapheme: "ó" });
    assert.deepEqual(acuteA.at(-1), { ...accent, grapheme: "á" });
    assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(acuteO)), acuteO);
    const shifted = geometricStarter(
        "ó", 1550, 2450, 1100, 250, "#123456",
        { dx: -250, lift: 150 },
    );
    assert.deepEqual(shifted.slice(0, 48), acuteO.slice(0, 48));
    assert.deepEqual(shifted.at(-1), {
        ...accent, grapheme: "ó",
        x1: 915, y1: 830, x2: 1575, y2: 335,
    });
    for (const position of [
        { dx: 501, lift: 0 }, { dx: -501, lift: 0 },
        { dx: 0, lift: -1 }, { dx: 0, lift: 501 },
        { dx: 1.25, lift: 0 },
    ]) assert.throws(() => geometricStarter(
        "ó", 1550, 2450, 1100, 250, "#123456", position,
    ));
    assert.throws(() => geometricStarter(
        "ó", 250, 2450, 200, 80, "#123456", { dx: -500, lift: 0 },
    ), /Acute accent/);
    assert.throws(() => geometricStarter(
        "ó", 1550, 1600, 1100, 250, "#123456",
    ), /Acute accent/);
    for (const bad of [
        ["ß", 1550, 2450, 1100, 250, "#123456"],
        ["o", -1, 2450, 1100, 250, "#123456"],
        ["o", 200, 2450, 1100, 250, "#123456"],
        ["a", 1550, 2450, 0, 250, "#123456"],
        ["o", 1550, 2450, 1100.25, 250, "#123456"],
        ["o", 1550, 2450, 1100, 79, "#123456"],
        ["o", 1550, 2450, 1100, 250, "red"],
    ]) assert.throws(() => geometricStarter(...bad));
});

test("one-click Spanish accents reuse exact bases and honest SVG guides",
    async () => {
        const args = [1550, 2450, 1100, 250, "#243647"];
        const forms = [
            ["é", "e", 38, 1], ["ú", "u", 35, 1],
            ["ü", "u", 36, 2], ["ñ", "n", 37, 3],
        ];
        for (const [glyph, base, count, marks] of forms) {
            const source = geometricStarter(base, ...args);
            const proposed = geometricStarter(glyph, ...args);
            assert.equal(proposed.length, count);
            assert.deepEqual(proposed, composeAccentedGlyph(source, base,
                glyph, "#243647"), "same one-source accent geometry");
            assert.deepEqual(proposed.slice(0, source.length),
                source.map((line) => ({ ...line, grapheme: glyph })));
            assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(proposed)),
                proposed, "exact precomposed Unicode survives native TSV");
            assert.equal(proposed.slice(-marks).length, marks);
            const shifted = geometricStarter(glyph, ...args,
                { dx: -200, lift: 100 });
            assert.deepEqual(shifted.slice(0, source.length),
                proposed.slice(0, source.length),
                "shift and lift never alter the base ink");
            assert.notDeepEqual(shifted.slice(-marks), proposed.slice(-marks));
            for (const position of [
                { dx: 501, lift: 0 }, { dx: 0, lift: 501 },
                { dx: 0.5, lift: 0 },
            ]) assert.throws(() => geometricStarter(glyph,
                ...args, position));
            assert.throws(() => geometricStarter(glyph,
                1550, 1350, 1100, 250, "#243647"),
            /Accent and pen must fit inside glyph cell/);
            assert.throws(() => geometricStarter(glyph,
                ...args, undefined, "italic"),
            /Italic variant requires matching o\/a glyph/);
        }
        const ui = mountEditor();
        try {
            const { editor, get, downloads } = ui;
            editor.start();
            for (const [glyph, , count, marks] of forms) {
                get("form").value = glyph;
                get("character").value = glyph;
                get("form").dispatch("input");
                assert.equal(get("ideal-rounded").attributes.get("visibility"),
                    glyph === "é" ? "hidden" : "visible");
                assert.equal(get("ideal-c-loop").attributes.get("visibility"),
                    glyph === "é" ? "visible" : "hidden");
                assert.equal(get("ideal-accent").attributes.get("visibility"),
                    marks === 1 ? "visible" : "hidden");
                const diacritic = get("ideal-diacritic");
                assert.equal(diacritic.attributes.get("visibility"),
                    marks > 1 ? "visible" : "hidden");
                assert.equal((diacritic.attributes.get("d")
                    .match(/ L /g) ?? []).length, marks > 1 ? marks : 0);
                get("construct").dispatch("click");
                assert.match(get("status").textContent, /Constructed/);
                get("save").dispatch("click");
                const saved = parseGlyphLibrary(
                    await downloads.at(-1).text());
                assert.equal(saved.filter((line) => line.grapheme === glyph)
                    .length, count);
            }
            const accepted = await downloads.at(-1).text();
            get("circle-y").value = "1350";
            get("circle-y").dispatch("input");
            assert.equal(get("ideal-rounded").attributes.get("visibility"),
                "hidden", "invalid accent must hide the entire guide");
            assert.equal(get("ideal-diacritic").attributes.get("visibility"),
                "hidden", "never display stale accepted tilde ink");
            get("construct").dispatch("click");
            assert.match(get("status").textContent, /Accent and pen/);
            get("save").dispatch("click");
            assert.equal(await downloads.at(-1).text(), accepted,
                "refusal cannot replace previously authored Unicode ink");
        } finally {
            ui.restore();
        }
    });

test("uppercase Spanish starters inherit exact native capitals and accents",
    async () => {
        const args = [1550, 2450, 1100, 250, "#243647"];
        const cases = [
            ["Á", "A", 4, 1], ["É", "E", 5, 1],
            ["Í", "I", 4, 1], ["Ó", "O", 49, 1],
            ["Ú", "U", 35, 1], ["Ñ", "N", 6, 3],
            ["Ü", "U", 36, 2],
        ];
        for (const [glyph, base, length, marks] of cases) {
            const source = geometricStarter(base, ...args);
            const produced = geometricStarter(glyph, ...args);
            assert.equal(produced.length, length);
            assert.deepEqual(produced, composeAccentedGlyph(source, base,
                glyph, "#243647"));
            assert.deepEqual(produced.slice(0, -marks),
                source.map((line) => ({ ...line, grapheme: glyph })));
            assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(produced)),
                produced, "precomposed uppercase identity stays exact");
            assert.deepEqual(geometricStarter(glyph, ...args,
                { dx: -100, lift: 100 }).slice(0, -marks),
            produced.slice(0, -marks));
            assert.throws(() => geometricStarter(glyph,
                1550, 1350, 1100, 250, "#243647"),
            /Accent and pen must fit inside glyph cell/);
        }
        const ui = mountEditor();
        try {
            const { editor, get, downloads } = ui;
            editor.start();
            for (const [glyph, , count, marks] of cases) {
                get("form").value = glyph;
                get("character").value = glyph;
                get("form").dispatch("input");
                assert.equal(get("ideal-rounded").attributes.get("visibility"),
                    "visible", "capital SVG uses its own base geometry");
                const diacritic = get("ideal-diacritic");
                assert.equal(diacritic.attributes.get("visibility"),
                    marks > 1 ? "visible" : "hidden");
                assert.equal((diacritic.attributes.get("d")
                    .match(/ L /g) ?? []).length, marks > 1 ? marks : 0);
                assert.equal(get("ideal-accent").attributes.get("visibility"),
                    marks === 1 ? "visible" : "hidden");
                assert.equal(get("ideal-circle").attributes.get("visibility"),
                    glyph === "Ó" ? "visible" : "hidden");
                get("construct").dispatch("click");
                assert.match(get("status").textContent,
                    new RegExp(`Constructed ${count} sampled segments`));
            }
            get("save").dispatch("click");
            const original = await downloads.at(-1).text();
            assert.deepEqual(parseGlyphLibrary(original), cases.flatMap(
                ([glyph]) => geometricStarter(glyph, ...args)));
            get("circle-y").value = "1350";
            get("circle-y").dispatch("input");
            assert.equal(get("ideal-diacritic").attributes.get("visibility"),
                "hidden", "clipped diaeresis hides all stale marks");
            get("construct").dispatch("click");
            assert.match(get("status").textContent, /Accent and pen/);
            get("save").dispatch("click");
            assert.equal(await downloads.at(-1).text(), original,
                "invalid uppercase edit preserves prior Unicode library");
        } finally {
            ui.restore();
        }
    });

test("printed two-storey a samples a continuous analytic upper story", () => {
    const args = [1550, 2450, 1100, 250, "#123456"];
    const circle = geometricStarter("o", ...args);
    const two = geometricStarter("a", ...args, undefined, "two-storey");
    const simple = geometricStarter("a", ...args);
    assert.equal(two.length, 81, "48 bowl, 32 arch, 1 stem");
    assert.deepEqual(two.slice(0, 48), circle.map((line) => ({
        ...line, grapheme: "a",
    })));
    assert.deepEqual(two.at(-1), simple.at(-1),
        "same precisely positioned right stem");
    const arc = two.slice(48, 80);
    assert.deepEqual([arc[0].x1, arc[0].y1], [772, 1672]);
    assert.deepEqual([arc[15].x2, arc[15].y2], [1550, 250]);
    assert.deepEqual([arc.at(-1).x2, arc.at(-1).y2], [2650, 1350]);
    for (let i = 0; i < arc.length; i += 1) {
        const line = arc[i];
        assert.ok(line.x1 !== line.x2 || line.y1 !== line.y2);
        if (i + 1 < arc.length) {
            assert.deepEqual([line.x2, line.y2],
                [arc[i + 1].x1, arc[i + 1].y1]);
        }
    }
    assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(two)), two);
    const acuteTwo = geometricStarter("á", 1550, 2450, 600, 250,
        "#123456", { dx: 150, lift: 100 }, "two-storey");
    assert.equal(acuteTwo.length, 82);
    assert.deepEqual(acuteTwo.at(-1), {
        grapheme: "á", x1: 1490, y1: 780, x2: 1850, y2: 510,
        width: 250, color: "#123456",
    });
    assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(acuteTwo)),
        acuteTwo);
    assert.throws(() => geometricStarter("o", ...args,
        undefined, "two-storey"), /Two-storey/);
    assert.throws(() => geometricStarter("a", 1550, 2000, 1100, 250,
        "#123456", undefined, "two-storey"), /Two-storey arch/);
    assert.throws(() => geometricStarter("á", ...args,
        undefined, "two-storey"), /Acute accent/);
});

test("b/d/p/q reuse the exact o bowl with correctly extended stems", () => {
    const args = [1550, 2450, 1100, 250, "#123456"];
    const circle = geometricStarter("o", ...args);
    const stems = [
        ["b", 450, 800, 450, 3550],
        ["d", 2650, 800, 2650, 3550],
        ["p", 450, 1350, 450, 4100],
        ["q", 2650, 1350, 2650, 4100],
    ];
    for (const [glyph, x1, y1, x2, y2] of stems) {
        const lines = geometricStarter(glyph, ...args);
        assert.equal(lines.length, 49);
        assert.deepEqual(lines.slice(0, 48), circle.map((line) => ({
            ...line, grapheme: glyph,
        })), `same o circular segment geometry for ${glyph}`);
        assert.deepEqual(lines.at(-1), {
            grapheme: glyph, x1, y1, x2, y2,
            width: 250, color: "#123456",
        });
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(lines)),
            lines);
    }
    assert.throws(() => geometricStarter("b", ...args,
        undefined, "two-storey"), /Two-storey/);
    assert.throws(() => geometricStarter("b", 1550, 1450, 1100,
        250, "#123456"), /Loop stem and pen/);
    assert.throws(() => geometricStarter("d", 1550, 1450, 1100,
        250, "#123456"), /Loop stem and pen/);
    assert.throws(() => geometricStarter("p", 1550, 3000, 1100,
        250, "#123456"), /Loop stem and pen/);
    assert.throws(() => geometricStarter("q", 1550, 3000, 1100,
        250, "#123456"), /Loop stem and pen/);
    assert.throws(() => geometricStarter("p", 1550, 2450, 1100,
        900, "#123456"), /Loop stem and pen/);
});

test("loop-stem SVG guide and TSV replacement stay in sync", async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        editor.start();
        const previous = [stroke("∞"), stroke("b")];
        const input = new TextEncoder().encode(encodeGlyphLibrary(previous));
        get("load").files = [{
            size: input.length, arrayBuffer: async () => input.buffer,
        }];
        await get("load").dispatch("change");
        for (const [glyph, x, y1, y2] of [
            ["b", "450", "800", "3550"],
            ["d", "2650", "800", "3550"],
            ["p", "450", "1350", "4100"],
            ["q", "2650", "1350", "4100"],
        ]) {
            get("character").value = glyph;
            get("character").dispatch("input");
            get("form").value = glyph;
            get("form").dispatch("change");
            assert.equal(get("ideal-circle").attributes.get("visibility"),
                "visible");
            assert.equal(get("ideal-stem").attributes.get("visibility"),
                "visible");
            assert.equal(get("ideal-stem").attributes.get("x1"), x);
            assert.equal(get("ideal-stem").attributes.get("y1"), y1);
            assert.equal(get("ideal-stem").attributes.get("x2"), x);
            assert.equal(get("ideal-stem").attributes.get("y2"), y2);
            assert.equal(get("ideal-arch").attributes.get("visibility"),
                "hidden");
            get("construct").dispatch("click");
            assert.equal(get("preview").children.length, 49);
        }
        get("save").dispatch("click");
        const accepted = parseGlyphLibrary(await downloads.at(-1).text());
        assert.deepEqual(accepted.slice(0, 1), [previous[0]]);
        assert.equal(accepted.length, 197);
        for (const glyph of ["b", "d", "p", "q"]) {
            assert.deepEqual(accepted.filter((line) => line.grapheme === glyph),
                geometricStarter(glyph, 1550, 2450, 1100, 250,
                    "#243647"));
        }
        get("character").value = "q";
        get("character").dispatch("input");
        get("circle-y").value = "3000";
        get("circle-y").dispatch("input");
        assert.equal(get("ideal-circle").attributes.get("visibility"),
            "hidden", "invalid descender must hide guide");
        get("construct").dispatch("click");
        assert.match(get("status").textContent, /Loop stem and pen/);
        get("save").dispatch("click");
        assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
            accepted, "refused replacement is atomic for all graphemes");
    } finally {
        ui.restore();
    }
});

test("accent composition preserves authored base and target Unicode", () => {
    const base = { ...stroke("e"), x1: 900, y1: 1900,
        x2: 2350, y2: 3300, color: "#112233" };
    const source = [base, { ...base, grapheme: "n" },
        { ...base, grapheme: "u" },
        { ...base, grapheme: "A" }];
    const acute = composeAccentedGlyph(source, "e", "é", "#ABCDEF");
    assert.deepEqual(acute, [
        { ...base, grapheme: "é" },
        { grapheme: "é", x1: 1393, y1: 1530,
            x2: 1857, y2: 1090, width: 250, color: "#ABCDEF" },
    ]);
    assert.deepEqual(source[0], base, "composition does not edit the base");
    const decomposed = composeAccentedGlyph(source, "e", "e\u0301",
        "#ABCDEF", { dx: -150, lift: 100 });
    assert.deepEqual(decomposed.slice(0, 1), [{
        ...base, grapheme: "e\u0301",
    }]);
    assert.deepEqual(decomposed.at(-1), {
        ...acute.at(-1), grapheme: "e\u0301",
        x1: 1243, x2: 1707, y1: 1430, y2: 990,
    });
    assert.notEqual(acute[0].grapheme, decomposed[0].grapheme);
    const tilde = composeAccentedGlyph(source, "n", "ñ", "#123456");
    assert.equal(tilde.length, 4, "one base plus ordered wave segments");
    for (let i = 2; i < tilde.length; i += 1) {
        assert.deepEqual([tilde[i - 1].x2, tilde[i - 1].y2],
            [tilde[i].x1, tilde[i].y1]);
    }
    const dots = composeAccentedGlyph(source, "u", "u\u0308",
        "#DDEEFF");
    assert.equal(dots.length, 3, "one base plus two separate dots");
    assert.notEqual(dots[1].x1, dots[2].x1);
    assert.deepEqual(composeAccentedGlyph(source, "A", "Á", "#123456")
        .map((line) => line.grapheme), ["Á", "Á"]);
    assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary([
        ...source, ...acute, ...decomposed, ...tilde, ...dots,
    ])), [...source, ...acute, ...decomposed, ...tilde, ...dots]);
});

test("acute i composition removes only an identifiable upper dot", () => {
    const source = geometricStarter("i", 1550, 2450, 1100,
        250, "#123456");
    assert.equal(source.length, 2);
    const before = structuredClone(source);
    for (const target of ["í", "i\u0301"]) {
        const result = composeAccentedGlyph(source, "i", target,
            "#ABCDEF");
        assert.equal(result.length, 2, "stem and acute, never a second dot");
        assert.deepEqual(result[0], {
            ...source[0], grapheme: target,
        }, "unchanged pen-down stem is copied to target");
        assert.equal(result[1].grapheme, target);
        assert.ok(Math.max(result[1].y1, result[1].y2)
            < Math.min(result[0].y1, result[0].y2),
        "accent stays above the stem");
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(result)),
            result);
    }
    assert.deepEqual(source, before, "base i keeps its separate dot");
    const clippedDot = [source[0], { ...source[1], y1: 0, y2: 80 }];
    const clippedBefore = encodeGlyphLibrary(clippedDot);
    for (const target of ["í", "i\u0301"]) {
        const ink = composeAccentedGlyph(clippedDot,
            "i", target, "#ABCDEF");
        assert.deepEqual(ink[0], { ...source[0], grapheme: target },
            "removed dotted-i ink is not carried into an acute target");
        assert.equal(ink.length, 2);
    }
    assert.equal(encodeGlyphLibrary(clippedDot), clippedBefore);
    const oneStroke = source.slice(0, 1);
    assert.throws(() => composeAccentedGlyph(
        oneStroke, "i", "í", "#ABCDEF"), /clear upper dot/);
    assert.throws(() => composeAccentedGlyph([
        oneStroke[0], { ...source[1], y1: 1980, y2: 2060 },
    ], "i", "í", "#ABCDEF"), /clear upper dot/);
    assert.throws(() => composeAccentedGlyph([
        ...source, { ...source[0], x1: 2000, x2: 2000 },
    ], "i", "í", "#ABCDEF"), /clear upper dot|continuous stem path/,
    "the unrelated third mark cannot be mistaken for continuous stem ink");
});

test("acute i refuses an off-axis dot rather than silently erasing ink", () => {
    const printed = geometricStarter("i", 1550, 2450, 1100,
        250, "#123456");
    const shifted = [printed[0], { ...printed[1],
        x1: 2750, x2: 2750 }];
    const original = encodeGlyphLibrary(shifted);
    for (const exact of ["í", "i\u0301"]) {
        assert.throws(() => composeAccentedGlyph(shifted,
            "i", exact, "#ABCDEF"), /clear upper dot|aligned upper dot/);
        assert.equal(encodeGlyphLibrary(shifted), original);
    }
    for (const x of [1300, 1650, 1800]) {
        const near = [printed[0], { ...printed[1], x1: x, x2: x }];
        assert.equal(composeAccentedGlyph(near, "i", "í",
            "#ABCDEF").length, 2,
        "a pen-width offset must still count as an upper dot");
    }
    for (const x of [1299, 1801]) {
        const outside = [printed[0], { ...printed[1], x1: x, x2: x }];
        assert.throws(() => composeAccentedGlyph(outside, "i", "í",
            "#ABCDEF"), /aligned upper dot/,
        "a dot beyond pen-width clearance must be refused");
    }
});

test("connected i composes exact NFC and NFD acute without its dot", () => {
    const whole = fillMissingConnectedLowercase([], 1550, 2450,
        1100, 250, "#243647");
    const printed = whole.filter((line) => line.grapheme === "i");
    const source = structuredClone(whole);
    for (const target of ["í", "i\u0301"]) {
        const ink = composeAccentedGlyph(whole, "i", target, "#ABCDEF");
        assert.equal(ink.length, 4, "entry, stem, exit and lifted acute");
        assert.deepEqual(ink.slice(0, 3), printed.slice(0, 3)
            .map((line) => ({ ...line, grapheme: target })));
        assert.deepEqual(candidatePenPaths(ink, target).map(
            ({ first, last }) => [first, last]), [[1, 3], [4, 4]]);
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(ink)), ink);
    }
    const disconnected = printed.map((line) => ({ ...line }));
    disconnected[1].y1 += 1;
    assert.throws(() => composeAccentedGlyph(disconnected,
        "i", "í", "#ABCDEF"), /one continuous stem path/,
    "a disconnected body must not silently lose its other pen marks");
    const secondDot = { ...printed[3], y1: 900, y2: 980 };
    assert.throws(() => composeAccentedGlyph([...printed, secondDot],
        "i", "í", "#ABCDEF"), /clear upper dot|continuous stem path/,
    "a second lifted dot must not be confused with a stem segment");
    assert.deepEqual(whole, source, "composition never edits the base");
});

test("connected lowercase batch composes NFC/NFD Spanish accents in UI",
    async () => {
        const ui = mountEditor();
        try {
            const { editor, get, downloads } = ui;
            editor.start();
            get("seed-connected").dispatch("click");
            get("seed-spanish").dispatch("click");
            assert.match(get("status").textContent,
                /Filled 7 missing Spanish NFC accents/);
            get("seed-spanish-nfd").dispatch("click");
            assert.match(get("status").textContent,
                /Filled 7 missing Spanish NFD accents/);
            assert.equal(downloads.length, 0);
            get("save").dispatch("click");
            const saved = parseGlyphLibrary(await downloads.at(-1).text());
            const connected = fillMissingConnectedLowercase([], 1550,
                2450, 1100, 250, "#243647");
            assert.deepEqual(saved.slice(0, connected.length), connected);
            const targets = ["á", "é", "í", "ó", "ú", "ñ", "ü"];
            for (const target of targets) {
                for (const exact of [target, target.normalize("NFD")]) {
                    const glyph = saved.filter((line) =>
                        line.grapheme === exact);
                    assert.ok(glyph.length > 1, `${exact} has authored ink`);
                    const paths = candidatePenPaths(glyph, exact);
                    assert.equal(paths.length, target === "ü" ? 3 : 2,
                        `${exact} retains deliberate accent pen lifts`);
                    assert.equal(paths.at(-1).last, glyph.length);
                    assert.ok(paths.at(-1).first > 1);
                }
            }
            assert.equal(saved.filter((line) => line.grapheme === "í")
                .length, 4, "connected i never carries its dot into í");
        } finally {
            ui.restore();
        }
    });

test("acute i button distinguishes Unicode targets", async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        editor.start();
        const original = [
            ...geometricStarter("i", 1550, 2450, 1100, 250,
                "#123456"), stroke("∞"),
        ];
        const bytes = new TextEncoder().encode(encodeGlyphLibrary(original));
        get("load").files = [{
            size: bytes.length, arrayBuffer: async () => bytes.buffer,
        }];
        await get("load").dispatch("change");
        get("copy-from").value = "i";
        get("character").value = "i\u0301";
        get("character").dispatch("input");
        get("compose").dispatch("click");
        assert.match(get("status").textContent, /Composed 2 ordered/);
        get("save").dispatch("click");
        const saved = parseGlyphLibrary(await downloads.at(-1).text());
        assert.deepEqual(saved.slice(0, 3), original);
        assert.deepEqual(saved.slice(3), composeAccentedGlyph(
            original, "i", "i\u0301", "#243647"));
        get("character").value = "í";
        get("character").dispatch("input");
        get("compose").dispatch("click");
        get("save").dispatch("click");
        const both = parseGlyphLibrary(await downloads.at(-1).text());
        assert.equal(both.filter((line) => line.grapheme === "í").length,
            2, "composed form has two strokes");
        assert.equal(both.filter((line) => line.grapheme === "i\u0301").length,
            2, "decomposed form remains independent");
    } finally {
        ui.restore();
    }
});

test("acute i refuses ambiguous ink without replacing target", async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        editor.start();
        const original = [stroke("i"), stroke("í"), stroke("∞")];
        const bytes = new TextEncoder().encode(encodeGlyphLibrary(original));
        get("load").files = [{
            size: bytes.length, arrayBuffer: async () => bytes.buffer,
        }];
        await get("load").dispatch("change");
        get("copy-from").value = "i";
        get("character").value = "í";
        get("character").dispatch("input");
        get("compose").dispatch("click");
        assert.match(get("status").textContent, /clear upper dot/);
        get("save").dispatch("click");
        assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
            original, "ambiguous accent must not corrupt existing í");
    } finally {
        ui.restore();
    }
});

test("off-axis i dot refuses browser accent without replacing either form",
    async () => {
        const ui = mountEditor();
        try {
            const { editor, get, downloads } = ui;
            const original = [
                ...geometricStarter("i", 1550, 2450, 1100, 250,
                    "#123456").map((line, index) => index === 1
                    ? { ...line, x1: 2750, x2: 2750 } : line),
                stroke("í"), stroke("i\u0301"), stroke("∞"),
            ];
            const bytes = new TextEncoder().encode(
                encodeGlyphLibrary(original));
            editor.start();
            get("load").files = [{ size: bytes.length,
                arrayBuffer: async () => bytes.buffer }];
            await get("load").dispatch("change");
            get("copy-from").value = "i";
            for (const target of ["í", "i\u0301"]) {
                get("character").value = target;
                get("character").dispatch("input");
                get("compose").dispatch("click");
                assert.match(get("status").textContent, /aligned upper dot/);
            }
            get("save").dispatch("click");
            assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
                original, "neither target nor source can lose off-axis ink");
        } finally {
            ui.restore();
        }
    });

test("accent composition refuses mismatches and clipped marks", () => {
    const base = { ...stroke("e"), x1: 500, y1: 1500,
        x2: 2600, y2: 3300 };
    const cases = [
        [[base], "o", "ó", "#123456"],
        [[base], "e", "á", "#123456"],
        [[base], "e", "è", "#123456"],
        [[base], "e", "e\u0301\u0301", "#123456"],
        [[base], "e", "é", "red"],
        [[base], "e", "é", "#123456", { dx: 501, lift: 0 }],
        [[base], "e", "é", "#123456", { dx: 0, lift: -1 }],
        [[base], "e", "é", "#123456", { dx: 0, lift: 501 }],
        [[{ ...base, y1: 500 }], "e", "é", "#123456"],
        [[{ ...base, x1: 10, x2: 200 }], "e", "é", "#123456",
            { dx: -500, lift: 0 }],
    ];
    for (const args of cases) {
        assert.throws(() => composeAccentedGlyph(...args));
    }
});

test("accent composition refuses source strokes with clipped pen edges", () => {
    const safe = { ...stroke("e"), x1: 125, y1: 1900,
        x2: 3375, y2: 4375 };
    const clipped = [
        { ...safe, x1: 0 },
        { ...safe, x2: 3500 },
        { ...safe, y2: 4500 },
        { ...safe, x1: 100 },
    ];
    const source = [safe, stroke("∞")];
    const before = encodeGlyphLibrary(source);
    for (const target of ["é", "e\u0301"]) {
        const admitted = composeAccentedGlyph(source, "e", target,
            "#243647");
        assert.equal(admitted[0].x1, 125,
            "full-pen contact with the cell boundary remains valid");
        for (const bad of clipped) {
            const library = [safe, bad, stroke("∞")];
            const original = encodeGlyphLibrary(library);
            assert.throws(() => composeAccentedGlyph(library,
                "e", target, "#243647"), /Source pen.*glyph cell/);
            assert.equal(encodeGlyphLibrary(library), original,
                "refused composed accents must not edit source TSV");
        }
    }
    assert.equal(encodeGlyphLibrary(source), before);
});

test("browser clipped source pen refuses accent without replacing ink",
    async () => {
        const ui = mountEditor();
        try {
            const { editor, get, downloads } = ui;
            const before = [
                { ...stroke("e"), x1: 0, y1: 1900,
                    x2: 2300, y2: 3300 },
                { ...stroke("é"), x1: 700, y1: 800,
                    x2: 1300, y2: 1000 },
                stroke("∞"),
            ];
            const bytes = new TextEncoder().encode(
                encodeGlyphLibrary(before));
            editor.start();
            get("load").files = [{ size: bytes.length,
                arrayBuffer: async () => bytes.buffer }];
            await get("load").dispatch("change");
            get("copy-from").value = "e";
            get("character").value = "é";
            get("character").dispatch("input");
            get("compose").dispatch("click");
            assert.match(get("status").textContent,
                /Source pen must fit inside glyph cell/);
            get("save").dispatch("click");
            assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
                before, "refusal cannot replace the already authored accent");
        } finally {
            ui.restore();
        }
    });

test("accent action replaces target and refuses atomically", async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        const base = { ...stroke("e"), x1: 900, y1: 1900,
            x2: 2350, y2: 3300 };
        const previous = [base, stroke("∞"), stroke("é")];
        assert.equal(get("compose").disabled, true);
        get("compose").dispatch("click");
        editor.start();
        const bytes = new TextEncoder().encode(encodeGlyphLibrary(previous));
        get("load").files = [{
            size: bytes.length, arrayBuffer: async () => bytes.buffer,
        }];
        await get("load").dispatch("change");
        get("copy-from").value = "e";
        get("character").value = "é";
        get("character").dispatch("input");
        get("compose").dispatch("click");
        assert.match(get("status").textContent, /Composed 2 ordered strokes/);
        assert.equal(get("preview").children.length, 2);
        get("save").dispatch("click");
        const accepted = parseGlyphLibrary(await downloads.at(-1).text());
        assert.deepEqual(accepted, [base, previous[1],
            ...composeAccentedGlyph(previous, "e", "é", "#243647")]);
        get("accent-lift").value = "501";
        get("compose").dispatch("click");
        assert.match(get("status").textContent, /accent lift/);
        get("save").dispatch("click");
        assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
            accepted, "a refused accent does not replace a saved draft");
        get("accent-lift").value = "0";
    get("italic-angle").value = "12";
        get("character").value = "è";
        get("character").dispatch("input");
        get("compose").dispatch("click");
        assert.match(get("status").textContent, /Only vowel acute/);
        get("character").value = "é";
        get("character").dispatch("input");
        get("copy-from").value = "u";
        get("compose").dispatch("click");
        assert.match(get("status").textContent, /selected base/);
        editor.reset();
        assert.equal(get("compose").disabled, true);
        assert.equal(get("copy-from").value, "");
    } finally {
        ui.restore();
    }
});

test("rounded n and u use smooth quadratic guides and sampled TSV", () => {
    const args = [1550, 2450, 1100, 250, "#123456"];
    const n = geometricStarter("n", ...args);
    const u = geometricStarter("u", ...args);
    assert.equal(n.length, 34, "one stem, 32 bow samples, one stem");
    assert.equal(u.length, 34);
    assert.deepEqual([n[0].x1, n[0].y1, n[0].x2, n[0].y2],
        [450, 3550, 450, 1350]);
    assert.deepEqual([n[1].x1, n[1].y1], [450, 2450]);
    assert.deepEqual([n[16].x2, n[16].y2], [1550, 1350]);
    assert.deepEqual([n[32].x2, n[32].y2], [2650, 2450]);
    assert.deepEqual([n[33].x1, n[33].y1,
        n[33].x2, n[33].y2], [2650, 2450, 2650, 3550]);
    assert.deepEqual([u[0].x1, u[0].y1,
        u[0].x2, u[0].y2], [450, 1350, 450, 2450]);
    assert.deepEqual([u[16].x2, u[16].y2], [1550, 3550]);
    assert.deepEqual([u[33].x1, u[33].y1,
        u[33].x2, u[33].y2], [2650, 2450, 2650, 1350]);
    for (const lines of [n, u]) {
        for (let i = 1; i < 33; i += 1) {
            if (i > 1) assert.deepEqual(
                [lines[i - 1].x2, lines[i - 1].y2],
                [lines[i].x1, lines[i].y1],
                "the two 16-segment bows have continuous sampling",
            );
        }
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(lines)),
            lines);
    }
    const tilde = composeAccentedGlyph(n, "n", "ñ", "#123456");
    const diaeresis = composeAccentedGlyph(u, "u", "ü", "#123456");
    assert.equal(tilde.length, 37);
    assert.equal(diaeresis.length, 36);
    assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary([
        ...n, ...u, ...tilde, ...diaeresis,
    ])).length, 141);
    assert.throws(() => geometricStarter("n", ...args,
        undefined, "two-storey"), /Two-storey/);
    assert.throws(() => geometricStarter("u", 1550, 2450, 1600, 250,
        "#123456"), /Shape and pen/);
});

test("n/u guides replace only matching glyphs", async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        editor.start();
        get("character").value = "n";
        get("character").dispatch("input");
        get("form").value = "n";
        get("form").dispatch("change");
        assert.equal(get("ideal-circle").attributes.get("visibility"),
            "hidden", "n is not a circular bowl");
        assert.equal(get("ideal-rounded").attributes.get("visibility"),
            "visible");
        assert.equal(get("ideal-rounded-left").attributes.get("x1"),
            "450");
        assert.equal(get("ideal-rounded-right").attributes.get("y2"),
            "3550");
        assert.equal(get("ideal-rounded").attributes.get("d"),
            "M 450 2450 Q 450 1350 1550 1350 Q 2650 1350 2650 2450");
        get("construct").dispatch("click");
        assert.equal(get("preview").children.length, 34);
        get("character").value = "u";
        get("character").dispatch("input");
        get("form").value = "u";
        get("form").dispatch("change");
        assert.equal(get("ideal-rounded").attributes.get("d"),
            "M 450 2450 Q 450 3550 1550 3550 Q 2650 3550 2650 2450");
        get("construct").dispatch("click");
        assert.equal(get("preview").children.length, 34);
        get("character").value = "ñ";
        get("character").dispatch("input");
        get("copy-from").value = "n";
        get("compose").dispatch("click");
        assert.equal(get("preview").children.length, 37);
        get("character").value = "ü";
        get("character").dispatch("input");
        get("copy-from").value = "u";
        get("compose").dispatch("click");
        assert.equal(get("preview").children.length, 36);
        get("save").dispatch("click");
        const saved = parseGlyphLibrary(await downloads[0].text());
        assert.equal(saved.length, 141);
        for (const [glyph, expected] of [
            ["n", 34], ["u", 34], ["ñ", 37], ["ü", 36],
        ]) assert.equal(saved.filter((line) => line.grapheme === glyph)
            .length, expected);
        get("circle-radius").value = "1600";
        get("form").value = "u";
        get("form").dispatch("change");
        get("character").value = "u";
        get("character").dispatch("input");
        get("construct").dispatch("click");
        assert.match(get("status").textContent, /Shape and pen/);
        get("save").dispatch("click");
        assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
            saved, "invalid quadratic parameters preserve target strokes");
        editor.reset();
        assert.equal(get("ideal-rounded").attributes.get("visibility"),
            "hidden");
    } finally {
        ui.restore();
    }
});

test("h extends the n ascender; m samples two exact quadratic humps", () => {
    const args = [1550, 2450, 1100, 250, "#123456"];
    const n = geometricStarter("n", ...args);
    const h = geometricStarter("h", ...args);
    const m = geometricStarter("m", ...args);
    assert.equal(h.length, 34);
    assert.deepEqual(h[0], {
        grapheme: "h", x1: 450, y1: 3550, x2: 450, y2: 800,
        width: 250, color: "#123456",
    });
    assert.deepEqual(h.slice(1), n.slice(1).map((line) => ({
        ...line, grapheme: "h",
    })), "h preserves the analytic n upper-hump samples");
    assert.equal(m.length, 67, "64 quadratic chords, three stems");
    for (const [index, expected] of [
        [0, [450, 3550, 450, 1350]],
        [1, [450, 2450]],
        [16, [1000, 1350]],
        [32, [1550, 2450]],
        [33, [1550, 2450, 1550, 3550]],
        [34, [1550, 2450]],
        [49, [2100, 1350]],
        [65, [2650, 2450]],
        [66, [2650, 2450, 2650, 3550]],
    ]) {
        const stroke = m[index];
        const actual = expected.length === 2
            ? index === 1 || index === 34
                ? [stroke.x1, stroke.y1] : [stroke.x2, stroke.y2]
            : [stroke.x1, stroke.y1, stroke.x2, stroke.y2];
        assert.deepEqual(actual, expected, `m geometric point ${index}`);
    }
    for (const [start, finish] of [[1, 32], [34, 65]]) {
        for (let index = start + 1; index <= finish; index += 1) {
            assert.deepEqual([m[index].x1, m[index].y1],
                [m[index - 1].x2, m[index - 1].y2]);
        }
    }
    assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary([
        ...h, ...m,
    ])).length, 101);
    assert.throws(() => geometricStarter("h", 1550, 1450, 1100,
        250, "#123456"), /Ascender stem and pen/);
    assert.throws(() => geometricStarter("m", 1550, 3900, 500,
        250, "#123456"), /Shape and pen/);
    assert.throws(() => geometricStarter("m", ...args,
        undefined, "two-storey"), /Two-storey/);
});

test("h/m SVG guides and TSV replacement remain session-safe", async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        editor.start();
        get("character").value = "h";
        get("character").dispatch("input");
        get("form").value = "h";
        get("form").dispatch("change");
        assert.equal(get("ideal-rounded").attributes.get("d"),
            "M 450 2450 Q 450 1350 1550 1350 Q 2650 1350 2650 2450");
        assert.equal(get("ideal-rounded-left").attributes.get("y2"),
            "800");
        assert.equal(get("ideal-circle").attributes.get("visibility"),
            "hidden");
        get("construct").dispatch("click");
        assert.equal(get("preview").children.length, 34);
        get("character").value = "m";
        get("character").dispatch("input");
        get("form").value = "m";
        get("form").dispatch("change");
        assert.equal(get("ideal-rounded").attributes.get("d"),
            "M 450 2450 Q 450 1350 1000 1350 Q 1550 1350 1550 2450"
            + " Q 1550 1350 2100 1350 Q 2650 1350 2650 2450"
            + " M 1550 2450 L 1550 3550");
        assert.equal(get("ideal-rounded").attributes.get("visibility"),
            "visible");
        assert.equal(get("ideal-rounded-right").attributes.get("y2"),
            "3550");
        get("construct").dispatch("click");
        assert.equal(get("preview").children.length, 67);
        get("save").dispatch("click");
        const accepted = parseGlyphLibrary(await downloads.at(-1).text());
        assert.equal(accepted.length, 101);
        for (const glyph of ["h", "m"]) {
            assert.deepEqual(accepted.filter((item) => item.grapheme === glyph),
                geometricStarter(glyph, 1550, 2450, 1100, 250,
                    "#243647"));
        }
        get("character").value = "h";
        get("character").dispatch("input");
        get("form").value = "h";
        get("form").dispatch("change");
        get("circle-y").value = "1450";
        get("circle-y").dispatch("input");
        assert.equal(get("ideal-rounded").attributes.get("visibility"),
            "hidden", "an invalid ascender hides stale geometry");
        get("construct").dispatch("click");
        assert.match(get("status").textContent, /Ascender stem and pen/);
        get("save").dispatch("click");
        assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
            accepted, "a refused h must not change either saved form");
    } finally {
        ui.restore();
    }
});

test("v and w share exact zigzag coordinates and ordered lines", () => {
    const args = [1550, 2450, 1100, 250, "#123456"];
    const v = geometricStarter("v", ...args);
    const w = geometricStarter("w", ...args);
    const vertices = {
        v: [[450, 1350], [1550, 3550], [2650, 1350]],
        w: [[450, 1350], [1000, 3550], [1550, 2450],
            [2100, 3550], [2650, 1350]],
    };
    for (const [glyph, lines] of [["v", v], ["w", w]]) {
        const corners = vertices[glyph];
        assert.equal(lines.length, corners.length - 1);
        for (let index = 0; index < lines.length; index += 1) {
            assert.deepEqual([lines[index].x1, lines[index].y1],
                corners[index]);
            assert.deepEqual([lines[index].x2, lines[index].y2],
                corners[index + 1]);
        }
    }
    assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary([...v, ...w])),
        [...v, ...w]);
    assert.throws(() => geometricStarter("w", ...args,
        undefined, "two-storey"), /Two-storey/);
    assert.throws(() => geometricStarter("v", 1550, 1100, 1100,
        250, "#123456"), /Shape and pen/);
});

test("v/w exact SVG lines and native TSV remain session-safe", async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        editor.start();
        for (const [glyph, d, count] of [
            ["v", "M 450 1350 L 1550 3550 L 2650 1350", 2],
            ["w", "M 450 1350 L 1000 3550 L 1550 2450"
                + " L 2100 3550 L 2650 1350", 4],
        ]) {
            get("character").value = glyph;
            get("character").dispatch("input");
            get("form").value = glyph;
            get("form").dispatch("change");
            assert.equal(get("ideal-rounded").attributes.get("d"), d);
            assert.equal(get("ideal-rounded").attributes.get("visibility"),
                "visible");
            for (const stem of ["ideal-rounded-left", "ideal-rounded-right"]) {
                assert.equal(get(stem).attributes.get("visibility"), "hidden");
            }
            get("construct").dispatch("click");
            assert.equal(get("preview").children.length, count);
        }
        get("save").dispatch("click");
        const saved = parseGlyphLibrary(await downloads.at(-1).text());
        assert.deepEqual(saved, [
            ...geometricStarter("v", 1550, 2450, 1100, 250,
                "#243647"),
            ...geometricStarter("w", 1550, 2450, 1100, 250,
                "#243647"),
        ]);
        get("character").value = "w";
        get("character").dispatch("input");
        get("form").value = "w";
        get("form").dispatch("change");
        get("circle-y").value = "1100";
        get("circle-y").dispatch("input");
        assert.equal(get("ideal-rounded").attributes.get("visibility"),
            "hidden");
        get("construct").dispatch("click");
        assert.match(get("status").textContent, /Shape and pen/);
        get("save").dispatch("click");
        assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
            saved, "refused zigzag must preserve prior authored ink");
    } finally {
        ui.restore();
    }
});

test("s completes selectable lowercase starters with a smooth S path", () => {
    const args = [1550, 2450, 1100, 250, "#123456"];
    for (const glyph of "abcdefghijklmnopqrstuvwxyz") {
        assert.ok(geometricStarter(glyph, ...args).length > 0,
            `missing lowercase geometric starter: ${glyph}`);
    }
    const s = geometricStarter("s", ...args);
    assert.equal(s.length, 64, "four analytic arcs, 16 ordered chords each");
    assert.deepEqual([s[0].x1, s[0].y1], [2320, 1570]);
    assert.deepEqual([s.at(-1).x2, s.at(-1).y2], [780, 3330]);
    const spans = [
        [[2320, 1570], [1550, 1350], [780, 1900]],
        [[780, 1900], [10, 2450], [1550, 2450]],
        [[1550, 2450], [3090, 2450], [2320, 3000]],
        [[2320, 3000], [1550, 3550], [780, 3330]],
    ];
    for (let i = 0; i < spans.length; i += 1) {
        const [start, control, end] = spans[i];
        for (let j = 0; j < 16; j += 1) {
            const t = (j + 1) / 16;
            const u = 1 - t;
            const sampled = s[i * 16 + j];
            assert.ok(Math.abs(sampled.x2 -
                (u * u * start[0] + 2 * t * u * control[0]
                    + t * t * end[0])) <= 0.5);
            assert.ok(Math.abs(sampled.y2 -
                (u * u * start[1] + 2 * t * u * control[1]
                    + t * t * end[1])) <= 0.5);
            if (i * 16 + j > 0) {
                const previous = s[i * 16 + j - 1];
                assert.deepEqual([sampled.x1, sampled.y1],
                    [previous.x2, previous.y2], "no pen lift within S");
            }
        }
        if (i > 0) {
            const [, priorControl, joint] = spans[i - 1];
            assert.deepEqual(start, joint);
            assert.deepEqual(
                start.map((value, axis) => value - priorControl[axis]),
                control.map((value, axis) => value - start[axis]),
                "quadratic handles give exact C1 tangent continuity",
            );
        }
    }
    assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(s)), s);
    assert.throws(() => geometricStarter("s", 200, 2450, 1100,
        250, "#123456"), /Shape and pen/);
    assert.throws(() => geometricStarter("s", ...args,
        undefined, "two-storey"), /Two-storey/);
});

test("s SVG and authored TSV replacement stay separate", async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        editor.start();
        get("character").value = "s";
        get("character").dispatch("input");
        get("form").value = "s";
        get("form").dispatch("change");
        assert.equal(get("ideal-rounded").attributes.get("d"),
            "M 2320 1570 Q 1550 1350 780 1900"
            + " Q 10 2450 1550 2450 Q 3090 2450 2320 3000"
            + " Q 1550 3550 780 3330");
        assert.equal(get("ideal-rounded").attributes.get("visibility"),
            "visible");
        for (const part of ["ideal-circle", "ideal-rounded-left",
            "ideal-rounded-right", "ideal-i-stem", "ideal-accent"]) {
            assert.equal(get(part).attributes.get("visibility"), "hidden");
        }
        get("construct").dispatch("click");
        assert.equal(get("preview").children.length, 64);
        get("save").dispatch("click");
        const original = parseGlyphLibrary(await downloads.at(-1).text());
        assert.deepEqual(original, geometricStarter("s", 1550, 2450,
            1100, 250, "#243647"));
        get("circle-x").value = "200";
        get("circle-x").dispatch("input");
        assert.equal(get("ideal-rounded").attributes.get("visibility"),
            "hidden", "invalid dimensions must hide outdated SVG");
        get("construct").dispatch("click");
        assert.match(get("status").textContent, /Shape and pen/);
        get("save").dispatch("click");
        assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
            original, "invalid construction preserves the authored glyph");
        editor.reset();
        assert.equal(get("ideal-rounded").attributes.get("visibility"),
            "hidden");
    } finally {
        ui.restore();
    }
});

test("connected lowercase batch preserves exact ink and refuses atomically",
    () => {
        const existing = [stroke("a"), stroke("∞"), stroke("n\u{303}")];
        const before = encodeGlyphLibrary(existing);
        const next = fillMissingConnectedLowercase(existing,
            1550, 2450, 1100, 250, "#243647");
        const missing = [..."bcdefghijklmnopqrstuvwxyz"];
        const added = missing.flatMap((glyph) => geometricStarter(glyph,
            1550, 2450, 1100, 250, "#243647", undefined, "cursive"));
        assert.deepEqual(next, [...existing, ...added]);
        assert.equal(encodeGlyphLibrary(existing), before);
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(next)), next);
        assert.equal(fillMissingConnectedLowercase(next,
            1550, 2450, 1100, 250, "#FFFFFF"), next,
        "fully authored lowercase remains unchanged after repeated click");
        for (const glyph of missing) {
            const paths = candidatePenPaths(next, glyph);
            const lifts = ["f", "i", "j", "t"].includes(glyph) ? 1 : 0;
            assert.equal(paths.length, 1 + lifts,
                `${glyph} retains only its deliberate dot/crossbar lift`);
        }
        assert.throws(() => fillMissingConnectedLowercase(existing,
            200, 2450, 1100, 250, "#243647"), /Shape and pen/);
        assert.deepEqual(existing, [stroke("a"), stroke("∞"),
            stroke("n\u{303}")], "clipped batch never changes source");
        const nearCap = Array.from({ length: 4090 }, () => stroke("∞"));
        assert.throws(() => fillMissingConnectedLowercase(nearCap,
            1550, 2450, 1100, 250, "#243647"),
        /Save requires 1..4096/);
        assert.equal(nearCap.length, 4090);
    });

test("connected lowercase fill is session-gated, unsaved and idempotent",
    async () => {
        const ui = mountEditor();
        try {
            const { editor, get, downloads } = ui;
            assert.equal(get("seed-connected").disabled, true);
            get("seed-connected").dispatch("click");
            assert.equal(get("preview").children.length, 0);
            editor.start();
            assert.equal(get("seed-connected").disabled, false);
            get("seed-connected").dispatch("click");
            assert.match(get("status").textContent,
                /Filled 26 missing connected lowercase/);
            assert.equal(downloads.length, 0);
            get("character").value = "g";
            get("character").dispatch("input");
            assert.equal(get("preview").children.length, 91);
            get("save").dispatch("click");
            const first = await downloads.at(-1).text();
            const expected = [..."abcdefghijklmnopqrstuvwxyz"].flatMap(
                (glyph) => geometricStarter(glyph, 1550, 2450, 1100,
                    250, "#243647", undefined, "cursive"));
            assert.deepEqual(parseGlyphLibrary(first), expected);
            get("color").value = "#ABCDEF";
            get("seed-connected").dispatch("click");
            assert.match(get("status").textContent, /All 26 lowercase/);
            get("save").dispatch("click");
            assert.equal(await downloads.at(-1).text(), first);
            editor.reset();
            assert.equal(get("seed-connected").disabled, true);
        } finally {
            ui.restore();
        }
    });

test("connected fill refuses bad settings without losing custom ink",
    async () => {
        const ui = mountEditor();
        try {
            const { editor, get, downloads } = ui;
            editor.start();
            const custom = [stroke("a"), stroke("∞")];
            const bytes = new TextEncoder().encode(
                encodeGlyphLibrary(custom));
            get("load").files = [{ size: bytes.length,
                arrayBuffer: async () => bytes.buffer }];
            await get("load").dispatch("change");
            get("circle-radius").value = "1600";
            get("seed-connected").dispatch("click");
            assert.match(get("status").textContent, /Shape and pen/);
            get("save").dispatch("click");
            assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
                custom);
            get("circle-radius").value = "1100";
            get("seed-connected").dispatch("click");
            assert.match(get("status").textContent,
                /Filled 25 missing connected lowercase/);
            get("save").dispatch("click");
            const saved = parseGlyphLibrary(await downloads.at(-1).text());
            assert.deepEqual(saved.slice(0, 2), custom,
                "custom a and unrelated symbol retain exact authored ink");
            assert.equal(saved.filter((line) => line.grapheme === "a")
                .length, 1);
        } finally {
            ui.restore();
        }
    });

test("fill-missing gesture seeds a-z only after admitted session", async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        assert.equal(get("seed-lowercase").disabled, true);
        get("seed-lowercase").dispatch("click");
        assert.equal(get("preview").children.length, 0);
        editor.start();
        assert.equal(get("seed-lowercase").disabled, false);
        get("seed-lowercase").dispatch("click");
        assert.match(get("status").textContent, /Filled 26 missing/);
        assert.equal(downloads.length, 0, "no implicit TSV download");
        get("character").value = "s";
        get("character").dispatch("input");
        assert.equal(get("preview").children.length, 64);
        get("save").dispatch("click");
        const saved = parseGlyphLibrary(await downloads.at(-1).text());
        const expected = [..."abcdefghijklmnopqrstuvwxyz"].flatMap((glyph) =>
            geometricStarter(glyph, 1550, 2450, 1100, 250, "#243647"));
        assert.deepEqual(saved, expected,
            "one click authors every missing lowercase template in order");
        assert.deepEqual([...new Set(saved.map((line) => line.grapheme))],
            [..."abcdefghijklmnopqrstuvwxyz"]);
        get("color").value = "#AA9988";
        get("seed-lowercase").dispatch("click");
        assert.match(get("status").textContent, /All 26 lowercase/);
        get("save").dispatch("click");
        assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
            saved, "repeated fill cannot replace an existing glyph");
        editor.reset();
        assert.equal(get("seed-lowercase").disabled, true);
        assert.equal(get("preview").children.length, 0);
    } finally {
        ui.restore();
    }
});

test("fill-missing preserves custom ink and refuses bad geometry", async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        editor.start();
        const originals = [stroke("a"), stroke("ñ"), stroke("∞")];
        const bytes = new TextEncoder().encode(encodeGlyphLibrary(originals));
        get("load").files = [{
            size: bytes.length, arrayBuffer: async () => bytes.buffer,
        }];
        await get("load").dispatch("change");
        get("circle-radius").value = "1600";
        get("seed-lowercase").dispatch("click");
        assert.match(get("status").textContent, /Shape and pen/);
        get("save").dispatch("click");
        assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
            originals, "invalid settings refuse the entire batch");
        get("circle-radius").value = "1100";
        get("seed-lowercase").dispatch("click");
        assert.match(get("status").textContent, /Filled 25 missing/);
        get("save").dispatch("click");
        const saved = parseGlyphLibrary(await downloads.at(-1).text());
        assert.deepEqual(saved.slice(0, 3), originals);
        assert.deepEqual(saved.slice(3),
            [..."bcdefghijklmnopqrstuvwxyz"].flatMap((glyph) =>
                geometricStarter(glyph, 1550, 2450, 1100,
                    250, "#243647")));
        assert.equal(saved.filter((line) => line.grapheme === "a").length,
            1, "never overwrite manually authored a");
    } finally {
        ui.restore();
    }
});

test("fill-missing atomically refuses over-cap glyph libraries", async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        editor.start();
        const existing = Array.from({ length: 4090 }, () => stroke("∞"));
        const bytes = new TextEncoder().encode(encodeGlyphLibrary(existing));
        get("load").files = [{
            size: bytes.length, arrayBuffer: async () => bytes.buffer,
        }];
        await get("load").dispatch("change");
        get("seed-lowercase").dispatch("click");
        assert.match(get("status").textContent, /Save requires 1..4096/);
        get("save").dispatch("click");
        const saved = parseGlyphLibrary(await downloads.at(-1).text());
        assert.equal(saved.length, 4090);
        assert.deepEqual(saved[0], existing[0]);
        assert.deepEqual(saved.at(-1), existing.at(-1));
    } finally {
        ui.restore();
    }
});

test("Spanish accent batch fills missing NFC forms", async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        assert.equal(get("seed-spanish").disabled, true);
        editor.start();
        get("seed-spanish").dispatch("click");
        assert.match(get("status").textContent, /base glyphs first/);
        get("seed-lowercase").dispatch("click");
        get("seed-spanish").dispatch("click");
        assert.match(get("status").textContent, /Filled 7 missing/);
        assert.equal(downloads.length, 0, "generation remains unsaved");
        get("character").value = "í";
        get("character").dispatch("input");
        assert.equal(get("preview").children.length, 2);
        get("save").dispatch("click");
        const saved = parseGlyphLibrary(await downloads.at(-1).text());
        const base = [..."abcdefghijklmnopqrstuvwxyz"].flatMap((g) =>
            geometricStarter(g, 1550, 2450, 1100, 250, "#243647"));
        const pairs = [["a", "á"], ["e", "é"], ["i", "í"],
            ["o", "ó"], ["u", "ú"], ["n", "ñ"], ["u", "ü"]];
        const accented = pairs.flatMap(([source, target]) =>
            composeAccentedGlyph(base, source, target, "#243647"));
        assert.deepEqual(saved, [...base, ...accented]);
        get("color").value = "#998877";
        get("seed-spanish").dispatch("click");
        assert.match(get("status").textContent, /All seven Spanish/);
        get("save").dispatch("click");
        assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
            saved, "repeat click never replaces authored accents");
        editor.reset();
        assert.equal(get("seed-spanish").disabled, true);
    } finally {
        ui.restore();
    }
});

test("Spanish accent batch refuses ambiguous i", async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        editor.start();
        const base = ["a", "e", "i", "o", "u", "n"].map((glyph) =>
            ({ ...stroke(glyph), x1: 900, y1: 1900,
                x2: 2350, y2: 3300 }));
        base.push(stroke("á"), stroke("e\u0301"));
        const bytes = new TextEncoder().encode(encodeGlyphLibrary(base));
        get("load").files = [{
            size: bytes.length, arrayBuffer: async () => bytes.buffer,
        }];
        await get("load").dispatch("change");
        get("seed-spanish").dispatch("click");
        assert.match(get("status").textContent, /clear upper dot/);
        get("save").dispatch("click");
        assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
            base, "one ambiguous dot refuses every pending accent");
        const cleanI = geometricStarter("i", 1550, 2450, 1100,
            250, "#123456");
        const fixed = [...base.filter((stroke) => stroke.grapheme !== "i"),
            ...cleanI];
        const fresh = new TextEncoder().encode(encodeGlyphLibrary(fixed));
        get("load").files = [{
            size: fresh.length, arrayBuffer: async () => fresh.buffer,
        }];
        await get("load").dispatch("change");
        get("seed-spanish").dispatch("click");
        assert.match(get("status").textContent, /Filled 6 missing/);
        get("save").dispatch("click");
        const saved = parseGlyphLibrary(await downloads.at(-1).text());
        assert.deepEqual(saved.slice(0, fixed.length), fixed,
            "preserve existing á and decomposed é plus all bases");
        assert.equal(saved.filter((line) => line.grapheme === "á").length,
            1, "custom NFC á was not overwritten");
        assert.equal(saved.filter((line) => line.grapheme === "e\u0301")
            .length, 1, "decomposed source key remains separate");
    } finally {
        ui.restore();
    }
});

test("decomposed Spanish batch preserves custom NFD and NFC", async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        assert.equal(get("seed-spanish-nfd").disabled, true);
        editor.start();
        get("seed-lowercase").dispatch("click");
        get("seed-spanish").dispatch("click");
        get("save").dispatch("click");
        const nfc = parseGlyphLibrary(await downloads.at(-1).text());
        const custom = { ...stroke("e\u0301"), color: "#ABCDEF" };
        const initial = [...nfc, custom];
        const bytes = new TextEncoder().encode(encodeGlyphLibrary(initial));
        get("load").files = [{
            size: bytes.length, arrayBuffer: async () => bytes.buffer,
        }];
        await get("load").dispatch("change");
        get("seed-spanish-nfd").dispatch("click");
        assert.match(get("status").textContent, /Filled 6 missing Spanish NFD/);
        assert.equal(downloads.length, 1, "no implicit file write");
        get("save").dispatch("click");
        const saved = parseGlyphLibrary(await downloads.at(-1).text());
        const targets = [["a", "a\u0301"], ["i", "i\u0301"],
            ["o", "o\u0301"], ["u", "u\u0301"],
            ["n", "n\u0303"], ["u", "u\u0308"]];
        const additions = targets.flatMap(([base, target]) =>
            composeAccentedGlyph(initial, base, target, "#243647"));
        assert.deepEqual(saved, [...initial, ...additions]);
        assert.equal(saved.filter((s) => s.grapheme === "e\u0301").length,
            1, "custom decomposed ink is kept byte-for-byte");
        assert.equal(saved.filter((s) => s.grapheme === "é").length,
            nfc.filter((s) => s.grapheme === "é").length,
            "independent NFC source ink remains untouched");
        assert.equal(saved.filter((s) => s.grapheme === "i\u0301").length,
            2, "decomposed acute i consists only of stem and acute");
        get("color").value = "#123456";
        get("seed-spanish-nfd").dispatch("click");
        assert.match(get("status").textContent,
            /All seven Spanish lowercase NFD/);
        get("save").dispatch("click");
        assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
            saved, "repeated NFD fill must not overwrite existing glyphs");
        editor.reset();
        assert.equal(get("seed-spanish-nfd").disabled, true);
    } finally {
        ui.restore();
    }
});

test("decomposed Spanish batch rejects ambiguous i atomically", async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        editor.start();
        const initial = ["a", "e", "i", "o", "u", "n"].map((glyph) =>
            ({ ...stroke(glyph), x1: 900, y1: 1900,
                x2: 2350, y2: 3300 }));
        initial.push(stroke("a\u0301"));
        const bytes = new TextEncoder().encode(encodeGlyphLibrary(initial));
        get("load").files = [{
            size: bytes.length, arrayBuffer: async () => bytes.buffer,
        }];
        await get("load").dispatch("change");
        get("seed-spanish-nfd").dispatch("click");
        assert.match(get("status").textContent, /clear upper dot/);
        get("save").dispatch("click");
        assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
            initial, "NFD batch refusal must preserve all earlier ink");
    } finally {
        ui.restore();
    }
});

test("Spanish punctuation has exact pen lifts and inverted geometry", () => {
    const counts = { ".": 1, ",": 1, "!": 2, "¡": 2,
        "?": 50, "¿": 50 };
    for (const [glyph, expected] of Object.entries(counts)) {
        const lines = geometricStarter(glyph, 1550, 2450, 1100,
            250, "#243647");
        assert.equal(lines.length, expected, `sampled ${glyph}`);
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(lines)),
            lines, `native TSV keeps exact ${glyph}`);
    }
    assert.deepEqual(geometricStarter("!", 1550, 2450, 1100,
        250, "#243647").map(({ x1, y1, x2, y2 }) =>
        [x1, y1, x2, y2]), [
        [1550, 1460, 1550, 2835], [1550, 3308, 1550, 3440],
    ]);
    assert.deepEqual(geometricStarter(".", 1550, 2450, 1100,
        250, "#243647")[0], {
        grapheme: ".", x1: 1550, y1: 3308,
        x2: 1550, y2: 3440, width: 250, color: "#243647",
    });
    for (const [standard, upsideDown] of [["!", "¡"], ["?", "¿"]]) {
        const normal = geometricStarter(standard, 1550, 2450, 1100,
            250, "#243647");
        const turned = geometricStarter(upsideDown, 1550, 2450, 1100,
            250, "#243647");
        assert.deepEqual(turned.map(({ x1, y1, x2, y2 }) =>
            [x1, y1, x2, y2]), normal.map(({ x1, y1, x2, y2 }) =>
            [3100 - x1, 4900 - y1, 3100 - x2, 4900 - y2]),
        `${upsideDown} is precisely the rotated ${standard} outline`);
    }
    for (const glyph of ["!", "¡", "?", "¿"]) {
        assert.throws(() => geometricStarter(glyph, 1550, 2450,
            200, 250, "#243647"), /visible pen-clearance/);
    }
});

test("question SVG guide stays analytic; TSV samples curves", async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        editor.start();
        get("form").value = "?";
        get("character").value = "?";
        get("form").dispatch("input");
        assert.equal(get("ideal-rounded").attributes.get("d"),
            "M 945 1955 Q 835 1460 1550 1460"
            + " Q 2540 1460 2265 2175"
            + " Q 2210 2450 1550 2725 L 1550 2835"
            + " M 1550 3308 L 1550 3440");
        assert.equal(get("ideal-rounded").attributes.get("visibility"),
            "visible");
        assert.equal(get("ideal-rounded-left").attributes.get("visibility"),
            "hidden");
        get("construct").dispatch("click");
        assert.equal(get("preview").children.length, 50);
        get("form").value = "¿";
        get("character").value = "¿";
        get("form").dispatch("input");
        assert.match(get("ideal-rounded").attributes.get("d"),
            /^M 2155 2945 Q /);
        get("construct").dispatch("click");
        get("save").dispatch("click");
        const saved = parseGlyphLibrary(await downloads.at(-1).text());
        assert.deepEqual(saved, ["?", "¿"].flatMap((glyph) =>
            geometricStarter(glyph, 1550, 2450, 1100,
                250, "#243647")));
        get("circle-radius").value = "200";
        get("circle-radius").dispatch("input");
        assert.equal(get("ideal-rounded").attributes.get("visibility"),
            "hidden", "refused punctuation must hide stale guide");
        get("construct").dispatch("click");
        assert.match(get("status").textContent, /visible pen-clearance/);
        get("save").dispatch("click");
        assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
            saved, "invalid geometry leaves prior punctuation untouched");
    } finally {
        ui.restore();
    }
});

test("fill missing punctuation never replaces custom ink", async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        assert.equal(get("seed-punctuation").disabled, true);
        editor.start();
        const original = [stroke("!"), stroke("ñ")];
        const bytes = new TextEncoder().encode(encodeGlyphLibrary(original));
        get("load").files = [{
            size: bytes.length, arrayBuffer: async () => bytes.buffer,
        }];
        await get("load").dispatch("change");
        get("seed-punctuation").dispatch("click");
        assert.match(get("status").textContent, /Filled 5 missing punctuation/);
        assert.equal(downloads.length, 0, "the draft is not auto-downloaded");
        get("save").dispatch("click");
        const saved = parseGlyphLibrary(await downloads.at(-1).text());
        assert.deepEqual(saved, [...original,
            ...[".", ",", "¡", "?", "¿"].flatMap((glyph) =>
                geometricStarter(glyph, 1550, 2450, 1100,
                    250, "#243647"))]);
        get("seed-punctuation").dispatch("click");
        assert.match(get("status").textContent, /All six basic punctuation/);
        get("save").dispatch("click");
        assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
            saved, "second click does not change custom punctuation");
        get("circle-radius").value = "200";
        get("seed-punctuation").dispatch("click");
        assert.match(get("status").textContent, /All six basic punctuation/);
        editor.reset();
        assert.equal(get("seed-punctuation").disabled, true);
    } finally {
        ui.restore();
    }
});

test("punctuation batch refuses invalid geometry and too much ink", async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        editor.start();
        get("circle-radius").value = "200";
        get("seed-punctuation").dispatch("click");
        assert.match(get("status").textContent, /visible pen-clearance/);
        assert.equal(get("preview").children.length, 0);
        get("circle-radius").value = "1100";
        const full = Array.from({ length: 4090 }, () => stroke("∞"));
        const bytes = new TextEncoder().encode(encodeGlyphLibrary(full));
        get("load").files = [{
            size: bytes.length, arrayBuffer: async () => bytes.buffer,
        }];
        await get("load").dispatch("change");
        get("seed-punctuation").dispatch("click");
        assert.match(get("status").textContent, /Save requires 1..4096/);
        get("save").dispatch("click");
        assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
            full, "refused batch cannot partially append punctuation");
    } finally {
        ui.restore();
    }
});

test("curly quotes and guillemets preserve exact Unicode", () => {
    const counts = { "«": 4, "»": 4, "‘": 16, "’": 16,
        "“": 32, "”": 32 };
    const samples = Object.entries(counts).map(([glyph, count]) => {
        const lines = geometricStarter(glyph, 1550, 2450, 1100,
            250, "#243647");
        assert.equal(lines.length, count);
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(lines)), lines);
        return [glyph, lines];
    });
    const generated = Object.fromEntries(samples);
    assert.deepEqual(generated["«"].map(({ x1, y1, x2, y2 }) =>
        [x1, y1, x2, y2]), [
        [1220, 1900, 780, 2450], [780, 2450, 1220, 3000],
        [1935, 1900, 1495, 2450], [1495, 2450, 1935, 3000],
    ]);
    for (const [left, right] of [["«", "»"], ["‘", "’"],
        ["“", "”"]]) {
        assert.deepEqual(generated[right].map(({ x1, y1, x2, y2 }) =>
            [x1, y1, x2, y2]),
        generated[left].map(({ x1, y1, x2, y2 }) =>
            [3100 - x1, y1, 3100 - x2, y2]),
        `${left}/${right} are exact integer mirrors`);
    }
    const opening = generated["“"];
    assert.notDeepEqual([opening[15].x2, opening[15].y2],
        [opening[16].x1, opening[16].y1],
        "two quote curls retain an independent pen lift");
});

test("quotation SVG guides match separate native sampled ink", async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        editor.start();
        get("form").value = "«";
        get("character").value = "«";
        get("form").dispatch("input");
        assert.equal(get("ideal-rounded").attributes.get("d"),
            "M 1220 1900 L 780 2450 L 1220 3000"
            + " M 1935 1900 L 1495 2450 L 1935 3000");
        assert.equal(get("ideal-rounded").attributes.get("visibility"),
            "visible");
        assert.equal(get("ideal-rounded-left").attributes.get("visibility"),
            "hidden");
        get("construct").dispatch("click");
        assert.equal(get("preview").children.length, 4);
        get("form").value = "”";
        get("character").value = "”";
        get("form").dispatch("input");
        assert.match(get("ideal-rounded").attributes.get("d"),
            /^M .*Q .* M .*Q /);
        get("construct").dispatch("click");
        get("save").dispatch("click");
        const saved = parseGlyphLibrary(await downloads.at(-1).text());
        assert.deepEqual(saved, ["«", "”"].flatMap((glyph) =>
            geometricStarter(glyph, 1550, 2450, 1100,
                250, "#243647")));
        get("circle-x").value = "150";
        get("circle-x").dispatch("input");
        assert.equal(get("ideal-rounded").attributes.get("visibility"),
            "hidden");
        get("construct").dispatch("click");
        assert.match(get("status").textContent, /fit inside glyph cell/);
        get("save").dispatch("click");
        assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
            saved, "refused quote geometry cannot replace ink");
    } finally {
        ui.restore();
    }
});

test("quotation fill keeps custom ink and refuses over-cap", async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        assert.equal(get("seed-quotes").disabled, true);
        editor.start();
        const originals = [stroke("‘"), stroke("ñ")];
        const bytes = new TextEncoder().encode(encodeGlyphLibrary(originals));
        get("load").files = [{
            size: bytes.length, arrayBuffer: async () => bytes.buffer,
        }];
        await get("load").dispatch("change");
        get("seed-quotes").dispatch("click");
        assert.match(get("status").textContent, /Filled 5 missing quotation/);
        assert.equal(downloads.length, 0, "no implicit file writing");
        get("save").dispatch("click");
        const saved = parseGlyphLibrary(await downloads.at(-1).text());
        assert.deepEqual(saved, [...originals, ...["«", "»", "’", "“", "”"]
            .flatMap((glyph) => geometricStarter(glyph, 1550, 2450,
                1100, 250, "#243647"))]);
        get("seed-quotes").dispatch("click");
        assert.match(get("status").textContent, /All six quotation glyphs/);
        get("save").dispatch("click");
        assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
            saved, "repeat quote fill must preserve prior ink");
        const full = Array.from({ length: 4050 }, () => stroke("∞"));
        const big = new TextEncoder().encode(encodeGlyphLibrary(full));
        get("load").files = [{
            size: big.length, arrayBuffer: async () => big.buffer,
        }];
        await get("load").dispatch("change");
        get("seed-quotes").dispatch("click");
        assert.match(get("status").textContent, /Save requires 1..4096/);
        get("save").dispatch("click");
        assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
            full, "oversized quotation batches refuse atomically");
        editor.reset();
        assert.equal(get("seed-quotes").disabled, true);
    } finally {
        ui.restore();
    }
});

test("five fill gestures author 52 exact glyphs", async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        editor.start();
        for (const action of ["seed-lowercase", "seed-spanish",
            "seed-spanish-nfd", "seed-punctuation", "seed-quotes"]) {
            get(action).dispatch("click");
            assert.match(get("status").textContent, /Filled/);
        }
        assert.equal(downloads.length, 0);
        get("save").dispatch("click");
        const all = parseGlyphLibrary(await downloads.at(-1).text());
        assert.equal(all.length, 1443);
        const exact = [..."abcdefghijklmnopqrstuvwxyz",
            ...["á", "é", "í", "ó", "ú", "ñ", "ü"],
            ...["a\u0301", "e\u0301", "i\u0301", "o\u0301",
                "u\u0301", "n\u0303", "u\u0308"],
            ...[".", ",", "!", "¡", "?", "¿"],
            ...["«", "»", "‘", "’", "“", "”"]];
        assert.deepEqual([...new Set(all.map((line) => line.grapheme))],
            exact, "no NFC/NFD collisions or missing Unicode forms");
        editor.reset();
        assert.equal(get("seed-quotes").disabled, true);
    } finally {
        ui.restore();
    }
});

test("selectable math glyphs never borrow division ink", () => {
    const args = [1750, 2250, 900, 100, "#243647"];
    const geometry = (glyph) => geometricStarter(glyph, ...args)
        .map(({ x1, y1, x2, y2 }) => [x1, y1, x2, y2]);
    const division = geometry("÷");
    assert.ok(SELECTABLE_MATH_GLYPHS.length > 150,
        "audit both automatic and manual educational math glyphs");
    assert.equal(new Set(SELECTABLE_MATH_GLYPHS).size,
        SELECTABLE_MATH_GLYPHS.length, "every selected key is unique");
    for (const glyph of SELECTABLE_MATH_GLYPHS) {
        const strokes = geometry(glyph);
        assert.ok(strokes.length > 0, `no missing authored ink for ${glyph}`);
        if (glyph !== "÷") assert.notDeepEqual(strokes, division,
            `math ${glyph} must not silently display division ink`);
    }
});

test("selected marks never borrow unrelated fallback geometry", () => {
    const args = [1750, 2250, 900, 100, "#243647"];
    const geometry = (glyph) => geometricStarter(glyph, ...args)
        .map(({ x1, y1, x2, y2 }) => [x1, y1, x2, y2]);
    const families = [
        ["?", [".", ",", "!", "¡", "¿"]],
        ["“", ["«", "»", "‘", "’", "”"]],
        ["€", ["$"]],
        ["%", ["°"]],
        ["(", [";", ":", "\"", "'", "…", "–", "—",
            ")", "[", "]", "/", "\\", "{", "}"]],
    ];
    for (const [fallback, marks] of families) {
        const alternate = geometry(fallback);
        for (const mark of marks) {
            assert.notDeepEqual(geometry(mark), alternate,
                `exact ${mark} cannot silently borrow ${fallback} ink`);
        }
    }
});

// Centralize the exact selected baseline for both shape identity and
// physical pen-clearance regression, so newly added selectors enter both.
const SELECTABLE_GEOMETRIC_GLYPHS = [...new Set([
    ..."abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789",
    ..."áéíóúñüÁÉÍÓÚÑÜ",
    ".", ",", "!", "¡", "?", "¿", "«", "»", "‘", "’", "“", "”",
    "°", "%", "$", "€", ";", ":", "\"", "'", "…", "–", "—",
    "(", ")", "[", "]", "/", "\\", "{", "}",
    ...SELECTABLE_MATH_GLYPHS,
])];

test("all selected geometric starters avoid unintended glyph aliases", () => {
    const glyphs = SELECTABLE_GEOMETRIC_GLYPHS;
    const geometries = new Map();
    const geometry = (glyph) => geometricStarter(glyph, 1750, 2250,
        900, 100, "#243647").map(({ x1, y1, x2, y2 }) =>
        [x1, y1, x2, y2]);
    assert.equal(glyphs.length, 296, "inventory changes require audit");
    for (const glyph of glyphs) {
        const shape = geometry(glyph);
        assert.ok(shape.length > 0, `missing authored ${glyph} ink`);
        const signature = JSON.stringify(shape);
        geometries.set(signature, [...(geometries.get(signature) ?? []),
            glyph]);
    }
    const shared = [...geometries.values()].filter((keys) =>
        keys.length > 1).map((keys) => keys.sort()).sort((a, b) =>
        a[0].localeCompare(b[0]));
    assert.deepEqual(shared, [
        ["-", "−"], ["C", "c"], ["O", "o"], ["S", "s"],
    ].map((keys) => keys.sort()).sort((a, b) =>
        a[0].localeCompare(b[0])),
    "only deliberate case/ASCII-minus pairs share physical pen geometry");
});

test("selected geometric strokes fit their full pen inside the glyph cell",
    () => {
        const connected = ["o", "a", "ó", "á", "c", "e", "é", "i", "í",
            "j", "l", "t", "f", "n", "ñ", "u", "ú", "ü", "m", "h",
            "r", "v", "w", "z", "s", "y", "k", "x", "g", "b", "p",
            "d", "q"];
        const accented = [..."áéíóúñüÁÉÍÓÚÑÜ"].map((glyph) =>
            [glyph.normalize("NFD"), "single"]);
        const choices = [
            ...SELECTABLE_GEOMETRIC_GLYPHS.map((glyph) =>
                [glyph, "single"]),
            ...connected.map((glyph) => [glyph, "cursive"]),
            ...["o", "a", "ó", "á"].map((glyph) => [glyph, "italic"]),
            ...["a", "á"].flatMap((glyph) =>
                [[glyph, "two-storey"], [glyph, "two-storey-italic"]]),
            ...accented,
        ];
        assert.equal(SELECTABLE_GEOMETRIC_GLYPHS.length, 296,
            "physical clearance inventory changes require review");
        let accepted = 0;
        let refused = 0;
        for (const [glyph, variant] of choices) {
            let acceptedForVariant = 0;
            for (const radius of [350, 900]) {
                for (const width of [100, 250]) {
                    const halfPen = width / 2;
                    const xMin = radius + halfPen;
                    const xMax = 3500 - xMin;
                    const yMin = radius + halfPen;
                    const yMax = 4500 - yMin;
                    // Exercise four physical corners and the interior: a
                    // sampled centerline may fit even when the pen does not.
                    const centers = [
                        [xMin, yMin], [xMax, yMin], [1750, 2250],
                        [xMin, yMax], [xMax, yMax],
                    ];
                    for (const [x, y] of centers) {
                        let strokes;
                        try {
                            strokes = geometricStarter(glyph, x, y, radius,
                                width, "#243647", undefined, variant);
                        } catch (error) {
                            assert.match(error.message,
                                /pen|clip|cell|clearance/,
                                `${glyph} ${variant} must refuse only `
                                    + "physical geometry");
                            refused += 1;
                            continue;
                        }
                        accepted += 1;
                        acceptedForVariant += 1;
                        assert.ok(strokes.length > 0,
                            `${glyph} ${variant} silently lost all ink`);
                        for (const line of strokes) {
                            assert.equal(line.grapheme, glyph,
                                "variant must preserve exact Unicode key");
                            for (const xEnd of [line.x1, line.x2]) {
                                assert.ok(xEnd - width / 2 >= 0
                                    && xEnd + width / 2 <= 3500,
                                `${glyph} ${variant} clipped horizontal ink`);
                            }
                            for (const yEnd of [line.y1, line.y2]) {
                                assert.ok(yEnd - width / 2 >= 0
                                    && yEnd + width / 2 <= 4500,
                                `${glyph} ${variant} clipped vertical ink`);
                            }
                        }
                    }
                }
            }
            assert.ok(acceptedForVariant > 0,
                `${glyph} ${variant} has no safe geometric construction`);
        }
        assert.ok(accepted > 0 && refused > 0,
            "boundary matrix must exercise both acceptance and refusal");
    });

test("eight basic math operators have exact bounded pen geometry", () => {
    const expected = {
        "+": [[780, 2450, 2320, 2450], [1550, 1680, 1550, 3220]],
        "-": [[780, 2450, 2320, 2450]],
        "−": [[780, 2450, 2320, 2450]],
        "±": [[780, 2098, 2320, 2098], [1550, 1515, 1550, 2670],
            [780, 3275, 2320, 3275]],
        "∓": [[780, 1625, 2320, 1625], [780, 2802, 2320, 2802],
            [1550, 2230, 1550, 3385]],
        "=": [[780, 2120, 2320, 2120], [780, 2780, 2320, 2780]],
        "×": [[890, 1790, 2210, 3110], [2210, 1790, 890, 3110]],
        "÷": [[780, 2450, 2320, 2450],
            [1550, 1680, 1550, 1812], [1550, 3088, 1550, 3220]],
    };
    for (const [glyph, segments] of Object.entries(expected)) {
        const lines = geometricStarter(glyph, 1550, 2450, 1100,
            250, "#243647");
        assert.deepEqual(lines.map(({ x1, y1, x2, y2 }) =>
            [x1, y1, x2, y2]), segments);
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(lines)), lines);
    }
    const plusMinus = geometricStarter("±", 1550, 2450, 1100,
        250, "#243647");
    assert.deepEqual(candidatePenPaths(plusMinus, "±")
        .map(({ first, last }) => [first, last]),
    [[1, 1], [2, 2], [3, 3]],
    "upper plus and lower minus preserve three directed pen lifts");
    const minusPlus = geometricStarter("∓", 1550, 2450, 1100,
        250, "#243647");
    assert.deepEqual(candidatePenPaths(minusPlus, "∓")
        .map(({ first, last }) => [first, last]),
    [[1, 1], [2, 2], [3, 3]],
    "upper minus and lower plus remain three directed pen lifts");
    assert.throws(() => geometricStarter("÷", 1550, 2450, 200,
        250, "#243647"), /Division dots need visible pen-clearance/);
});

test("extended mathematical symbols keep exact directed pen geometry", () => {
    const expected = {
        "√": [[615, 2670, 1000, 2560], [1000, 2560, 1440, 3330],
            [1440, 3330, 2045, 1625], [2045, 1625, 2485, 1625]],
        "π": [[670, 1790, 2430, 1790], [1000, 1790, 1000, 3275],
            [2100, 1790, 2100, 3055], [2100, 3055, 2320, 3275]],
        "≠": [[780, 2120, 2320, 2120], [780, 2780, 2320, 2780],
            [945, 3275, 2155, 1625]],
        "<": [[2155, 1790, 890, 2505], [890, 2505, 2155, 3110]],
        ">": [[945, 1790, 2210, 2505], [2210, 2505, 945, 3110]],
        "≤": [[2155, 1790, 890, 2505], [890, 2505, 2155, 3110],
            [780, 3418, 2320, 3418]],
        "≥": [[945, 1790, 2210, 2505], [2210, 2505, 945, 3110],
            [780, 3418, 2320, 3418]],
        "∧": [[835, 3110, 1550, 1790], [1550, 1790, 2265, 3110]],
        "∨": [[835, 1790, 1550, 3110], [1550, 3110, 2265, 1790]],
    };
    for (const [glyph, segments] of Object.entries(expected)) {
        const lines = geometricStarter(glyph, 1550, 2450, 1100,
            250, "#243647");
        assert.deepEqual(lines.map(({ x1, y1, x2, y2 }) =>
            [x1, y1, x2, y2]), segments);
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(lines)), lines);
        if (["<", ">", "∧", "∨"].includes(glyph)) {
            assert.deepEqual(candidatePenPaths(lines, glyph).map(
                ({ first, last }) => [first, last]), [[1, 2]],
            "comparison chevron keeps a continuous directed pen path");
        }
    }
    for (const [glyph, expected, closed] of [
        ["∑", [[725, 1515, 2375, 1515], [2375, 1515, 1165, 2450],
            [1165, 2450, 2375, 3385], [2375, 3385, 725, 3385]], false],
        ["Δ", [[1550, 1460, 670, 3440], [670, 3440, 2430, 3440],
            [2430, 3440, 1550, 1460]], true],
    ]) {
        const ink = geometricStarter(glyph, 1550, 2450, 1100,
            250, "#243647");
        assert.deepEqual(ink.map(({ x1, y1, x2, y2 }) =>
            [x1, y1, x2, y2]), expected);
        assert.deepEqual(candidatePenPaths(ink, glyph).length, 1);
        if (closed) assert.deepEqual(
            [ink[0].x1, ink[0].y1],
            [ink.at(-1).x2, ink.at(-1).y2]);
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(ink)), ink);
    }
    for (const [glyph, count, paths] of [["∫", 48, 1],
        ["∂", 64, 1], ["≈", 64, 2]]) {
        const ink = geometricStarter(glyph, 1550, 2450, 1100,
            250, "#243647");
        assert.equal(ink.length, count);
        assert.equal(candidatePenPaths(ink, glyph).length, paths);
        for (let i = 1; i < ink.length; i += 1) {
            if (glyph === "≈" && i === 32) continue;
            assert.deepEqual([ink[i - 1].x2, ink[i - 1].y2],
                [ink[i].x1, ink[i].y1],
                "quadratic pen chords preserve authored path continuity");
        }
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(ink)), ink);
    }
    const derivative = geometricStarter("∂", 1550, 2450, 1100,
        250, "#243647");
    assert.equal(derivative.length, 64, "four sampled ideal curves");
    assert.deepEqual([derivative[0].x1, derivative[0].y1],
        [2210, 1515], "the high tail precedes the rounded bowl");
    assert.deepEqual([derivative.at(-1).x2, derivative.at(-1).y2],
        [1165, 2230], "the bowl returns to its exact shoulder");
    assert.equal(candidatePenPaths(derivative, "∂").length, 1,
        "the derivative is a single continuous directed pen path");
    assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(derivative)),
        derivative, "exact derivative identity and pen order survive TSV");
    assert.throws(() => geometricStarter("∂", 150, 2450, 1100,
        250, "#243647"), /fit inside glyph cell|clips glyph cell/);
    const infinity = geometricStarter("∞", 1550, 2450, 1100,
        250, "#243647");
    assert.equal(infinity.length, 64,
        "four SVG quadratics, 16 ink chords each");
    assert.deepEqual([infinity[0].x1, infinity[0].y1], [1550, 2450]);
    assert.deepEqual([infinity.at(-1).x2, infinity.at(-1).y2],
        [1550, 2450], "continuous closed infinity pen loop");
    for (let i = 1; i < infinity.length; i += 1) {
        assert.deepEqual([infinity[i - 1].x2, infinity[i - 1].y2],
            [infinity[i].x1, infinity[i].y1],
            "each interpolated lobe keeps its directed pen join");
    }
    assert.deepEqual(candidatePenPaths(infinity, "∞").length, 1);
    assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(infinity)), infinity);
    assert.notDeepEqual(geometricStarter("-", 1550, 2450, 1100,
        250, "#243647"), geometricStarter("−", 1550, 2450, 1100,
        250, "#243647"), "ASCII and Unicode minus keys remain exact");
});

test("double and negated inequalities keep exact directed ink", () => {
    const shared = [1550, 2450, 1100, 250, "#243647"];
    const examples = [
        ["≪", [[1418, 1735, 714, 2450], [714, 2450, 1418, 3165],
            [2342, 1735, 1638, 2450], [1638, 2450, 2342, 3165]],
        [[1, 2], [3, 4]]],
        ["≫", [[758, 1735, 1462, 2450], [1462, 2450, 758, 3165],
            [1682, 1735, 2386, 2450], [2386, 2450, 1682, 3165]],
        [[1, 2], [3, 4]]],
        ["≮", [[2155, 1790, 890, 2505], [890, 2505, 2155, 3110],
            [648, 3352, 2452, 1548]], [[1, 2], [3, 3]]],
        ["≯", [[945, 1790, 2210, 2505], [2210, 2505, 945, 3110],
            [648, 3352, 2452, 1548]], [[1, 2], [3, 3]]],
    ];
    for (const [glyph, expected, paths] of examples) {
        const ink = geometricStarter(glyph, ...shared);
        assert.deepEqual(ink.map(({ x1, y1, x2, y2 }) =>
            [x1, y1, x2, y2]), expected);
        assert.deepEqual(candidatePenPaths(ink, glyph).map(
            ({ first, last }) => [first, last]), paths);
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(ink)), ink);
        assert.throws(() => geometricStarter(glyph, 120, 2450, 1100,
            250, "#243647"), /fit inside glyph cell|clips glyph cell/);
    }
    for (const [negative, positive] of [["≮", "<"], ["≯", ">"]]) {
        const ink = geometricStarter(negative, ...shared);
        const base = geometricStarter(positive, ...shared);
        assert.deepEqual(ink.slice(0, base.length).map(
            ({ grapheme, ...line }) => line), base.map(
                ({ grapheme, ...line }) => line));
        assert.equal(ink.length, base.length + 1);
    }
    assert.notDeepEqual(geometricStarter("≪", ...shared),
        geometricStarter("≫", ...shared));
});

test("negated inclusive signs and diagonal arrows keep exact pen order", () => {
    const shared = [1550, 2450, 1100, 250, "#243647"];
    const cases = [
        ["≰", [[2155, 1790, 890, 2505], [890, 2505, 2155, 3110],
            [780, 3418, 2320, 3418], [780, 3550, 2320, 1350]],
        [[1, 2], [3, 3], [4, 4]]],
        ["≱", [[945, 1790, 2210, 2505], [2210, 2505, 945, 3110],
            [780, 3418, 2320, 3418], [780, 3550, 2320, 1350]],
        [[1, 2], [3, 3], [4, 4]]],
        ["↗", [[780, 3220, 2320, 1680], [2320, 2560, 2320, 1680],
            [2320, 1680, 1440, 1680]], [[1, 1], [2, 3]]],
        ["↘", [[780, 1680, 2320, 3220], [1440, 3220, 2320, 3220],
            [2320, 3220, 2320, 2340]], [[1, 1], [2, 3]]],
    ];
    for (const [glyph, coordinates, penPaths] of cases) {
        const ink = geometricStarter(glyph, ...shared);
        assert.deepEqual(ink.map(({ x1, y1, x2, y2 }) =>
            [x1, y1, x2, y2]), coordinates);
        assert.deepEqual(candidatePenPaths(ink, glyph).map(
            ({ first, last }) => [first, last]), penPaths);
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(ink)), ink);
        assert.throws(() => geometricStarter(glyph, 120, 2450, 1100,
            250, "#243647"), /fit inside glyph cell|clips glyph cell/);
    }
    for (const [negative, positive] of [["≰", "≤"], ["≱", "≥"]]) {
        const ink = geometricStarter(negative, ...shared);
        const base = geometricStarter(positive, ...shared);
        assert.deepEqual(ink.slice(0, base.length).map(
            ({ grapheme, ...line }) => line), base.map(
            ({ grapheme, ...line }) => line));
        assert.equal(ink.length, base.length + 1);
    }
    assert.notDeepEqual(geometricStarter("↗", ...shared),
        geometricStarter("↘", ...shared));
});

test("left diagonal arrows and converse implication use directed ink", () => {
    const args = [1550, 2450, 1100, 250, "#243647"];
    const cases = [
        ["↖", [[2320, 3220, 780, 1680], [780, 2560, 780, 1680],
            [780, 1680, 1660, 1680]], [[1, 1], [2, 3]]],
        ["↙", [[2320, 1680, 780, 3220], [1660, 3220, 780, 3220],
            [780, 3220, 780, 2340]], [[1, 1], [2, 3]]],
        ["⇐", [[2430, 2274, 1055, 2274], [2430, 2626, 1055, 2626],
            [1330, 1845, 670, 2450], [670, 2450, 1330, 3055]],
        [[1, 1], [2, 2], [3, 4]]],
    ];
    for (const [glyph, expected, paths] of cases) {
        const ink = geometricStarter(glyph, ...args);
        assert.deepEqual(ink.map(({ x1, y1, x2, y2 }) =>
            [x1, y1, x2, y2]), expected);
        assert.deepEqual(candidatePenPaths(ink, glyph).map(
            ({ first, last }) => [first, last]), paths);
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(ink)), ink);
        assert.throws(() => geometricStarter(glyph, 120, 2450, 1100,
            250, "#243647"), /fit inside glyph cell|clips glyph cell/);
    }
    assert.notDeepEqual(geometricStarter("⇐", ...args),
        geometricStarter("⇒", ...args));
    assert.notDeepEqual(geometricStarter("↖", ...args),
        geometricStarter("↗", ...args));
    assert.notDeepEqual(geometricStarter("↙", ...args),
        geometricStarter("↘", ...args));
});

test("divisibility signs use distinct exact pen lifts", () => {
    const args = [1550, 2450, 1100, 250, "#243647"];
    for (const [glyph, coords, paths] of [
        ["∣", [[1550, 1570, 1550, 3330]], [[1, 1]]],
        ["∤", [[1550, 1570, 1550, 3330],
            [835, 3165, 2265, 1735]], [[1, 1], [2, 2]]],
    ]) {
        const ink = geometricStarter(glyph, ...args);
        assert.deepEqual(ink.map(({ x1, y1, x2, y2 }) =>
            [x1, y1, x2, y2]), coords);
        assert.deepEqual(candidatePenPaths(ink, glyph).map(
            ({ first, last }) => [first, last]), paths);
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(ink)), ink);
        assert.throws(() => geometricStarter(glyph, 100, 2450, 1100,
            250, "#243647"), /fit inside glyph cell|clips glyph cell/);
    }
    const plain = geometricStarter("∣", ...args);
    const negated = geometricStarter("∤", ...args);
    assert.deepEqual(negated[0], { ...plain[0], grapheme: "∤" },
        "negated divisibility retains the same directed upright");
    assert.notDeepEqual(plain, geometricStarter("+", ...args),
        "divisibility upright is not an arithmetic plus");
});

test("math prime strokes and dot/star operators retain exact pen paths", () => {
    const args = [1550, 2450, 1100, 250, "#243647"];
    for (const [glyph, coordinates] of [
        ["′", [[1550, 1570, 1275, 2285]]],
        ["″", [[1187, 1570, 912, 2285], [1913, 1570, 1638, 2285]]],
        ["‴", [[945, 1570, 670, 2285], [1550, 1570, 1275, 2285],
            [2155, 1570, 1880, 2285]]],
        ["⋅", [[1462, 2450, 1638, 2450]]],
        ["∗", [[780, 2450, 2320, 2450],
            [1165, 1790, 1935, 3110], [1935, 1790, 1165, 3110]]],
    ]) {
        const ink = geometricStarter(glyph, ...args);
        assert.deepEqual(ink.map(({ x1, y1, x2, y2 }) =>
            [x1, y1, x2, y2]), coordinates);
        assert.deepEqual(candidatePenPaths(ink, glyph).map(
            ({ first, last }) => [first, last]),
        coordinates.map((_, index) => [index + 1, index + 1]),
        "each deliberate operator stroke lifts independently");
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(ink)), ink);
        assert.throws(() => geometricStarter(glyph, 120, 2450, 1100,
            250, "#243647"), /fit inside glyph cell|clips glyph cell/);
    }
    assert.notDeepEqual(geometricStarter("⋅", ...args),
        geometricStarter(".", ...args),
        "center dot is independent of baseline punctuation");
    assert.notDeepEqual(geometricStarter("′", ...args),
        geometricStarter("'", ...args),
        "prime cannot borrow typographic apostrophe ink");
});

test("manual mathematical angle brackets preserve directed Unicode ink", () => {
    const args = [1550, 2450, 1100, 250, "#243647"];
    for (const [glyph, expected, ordinary] of [
        ["⟨", [[2232, 1570, 912, 2450],
            [912, 2450, 2232, 3330]], "<"],
        ["⟩", [[868, 1570, 2188, 2450],
            [2188, 2450, 868, 3330]], ">"],
    ]) {
        const ink = geometricStarter(glyph, ...args);
        assert.deepEqual(ink.map(({ x1, y1, x2, y2 }) =>
            [x1, y1, x2, y2]), expected);
        assert.deepEqual(candidatePenPaths(ink, glyph).map(
            ({ first, last }) => [first, last]), [[1, 2]]);
        assert.notDeepEqual(ink, geometricStarter(ordinary, ...args));
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(ink)), ink);
        assert.throws(() => geometricStarter(glyph, 100, 2450, 1100,
            250, "#243647"), /fit inside glyph cell|clips glyph cell/);
    }
});

test("manual Greek math keeps exact alpha beta theta lambda pen paths", () => {
    const args = [1550, 2450, 1100, 250, "#243647"];
    const expectedCounts = [["α", 65], ["β", 65],
        ["θ", 65], ["λ", 2]];
    const ink = expectedCounts.flatMap(([glyph, count]) => {
        const strokes = geometricStarter(glyph, ...args);
        assert.equal(strokes.length, count, `${glyph} native pen count`);
        assert.ok(strokes.every((part) => part.grapheme === glyph));
        const paths = candidatePenPaths(strokes, glyph);
        assert.equal(paths.length, 2, `${glyph} deliberate pen lifts`);
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(strokes)),
            strokes, `${glyph} exact Unicode and directed stroke replay`);
        assert.throws(() => geometricStarter(glyph, 100, 2450, 1100,
            250, "#243647"), /fit inside glyph cell|clips glyph cell/);
        return strokes;
    });
    assert.equal(ink.length, 197);
    assert.deepEqual(new Set(ink.map((line) => line.grapheme)),
        new Set(expectedCounts.map(([glyph]) => glyph)));
    assert.notDeepEqual(geometricStarter("θ", ...args),
        geometricStarter("o", ...args),
        "theta needs separate crossbar and cannot alias Latin o");
    assert.notDeepEqual(geometricStarter("α", ...args),
        geometricStarter("a", ...args),
        "Greek alpha cannot borrow single-storey Latin a");
    assert.throws(() => geometricStarter("ϑ", ...args),
        /supported exact geometric glyph/);
});

test("manual Greek mu and gamma keep exact non-Latin pen paths", () => {
    const args = [1550, 2450, 1100, 250, "#243647"];
    const glyphs = [["μ", 34, 1], ["γ", 33, 2]];
    const strokes = glyphs.flatMap(([glyph, count, lifts]) => {
        const ink = geometricStarter(glyph, ...args);
        assert.equal(ink.length, count, glyph + " sampled stroke count");
        assert.equal(candidatePenPaths(ink, glyph).length, lifts,
            glyph + " deliberate pen lifts");
        assert.ok(ink.every((part) => part.grapheme === glyph));
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(ink)), ink);
        assert.throws(() => geometricStarter(glyph, 100, 2450, 1100,
            250, "#243647"), /fit inside glyph cell|clips glyph cell/);
        return ink;
    });
    assert.equal(strokes.length, 67);
    assert.notDeepEqual(geometricStarter("μ", ...args),
        geometricStarter("u", ...args), "mu cannot borrow Latin u");
    assert.notDeepEqual(geometricStarter("γ", ...args),
        geometricStarter("y", ...args), "gamma cannot borrow Latin y");
    assert.throws(() => geometricStarter("ϑ", ...args),
        /supported exact geometric glyph/);
});

test("manual Greek delta and sigma keep exact Unicode pen paths", () => {
    const args = [1550, 2450, 1100, 250, "#243647"];
    for (const glyph of ["δ", "σ"]) {
        const ink = geometricStarter(glyph, ...args);
        assert.equal(ink.length, 65, glyph + " sampled stroke count");
        assert.equal(candidatePenPaths(ink, glyph).length, 2,
            glyph + " separate accent or exit path");
        assert.ok(ink.every((part) => part.grapheme === glyph));
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(ink)), ink);
        assert.throws(() => geometricStarter(glyph, 100, 2450, 1100,
            250, "#243647"), /fit inside glyph cell|clips glyph cell/);
    }
    assert.notDeepEqual(geometricStarter("δ", ...args),
        geometricStarter("o", ...args), "delta cannot borrow Latin o");
    assert.notDeepEqual(geometricStarter("σ", ...args),
        geometricStarter("θ", ...args), "sigma cannot borrow Greek theta");
    assert.throws(() => geometricStarter("ϑ", ...args),
        /supported exact geometric glyph/);
});

test("manual Greek rho and tau keep independent pen paths", () => {
    const args = [1550, 2450, 1100, 250, "#243647"];
    for (const [glyph, count] of [["ρ", 65], ["τ", 3]]) {
        const ink = geometricStarter(glyph, ...args);
        assert.equal(ink.length, count, glyph + " exact stroke count");
        assert.equal(candidatePenPaths(ink, glyph).length, 2,
            glyph + " preserves deliberate pen lifts");
        assert.ok(ink.every((part) => part.grapheme === glyph));
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(ink)), ink);
        assert.throws(() => geometricStarter(glyph, 100, 2450, 1100,
            250, "#243647"), /fit inside glyph cell|clips glyph cell/);
    }
    assert.notDeepEqual(geometricStarter("ρ", ...args),
        geometricStarter("p", ...args), "rho is not Latin p");
    assert.notDeepEqual(geometricStarter("τ", ...args),
        geometricStarter("t", ...args), "tau is not Latin t");
    assert.throws(() => geometricStarter("ϑ", ...args),
        /supported exact geometric glyph/);
});

test("manual Greek epsilon and omega keep distinct pen lifts", () => {
    const args = [1550, 2450, 1100, 250, "#243647"];
    for (const [glyph, count] of [["ε", 33], ["ω", 65]]) {
        const ink = geometricStarter(glyph, ...args);
        assert.equal(ink.length, count, glyph + " exact sampled strokes");
        assert.equal(candidatePenPaths(ink, glyph).length, 2,
            glyph + " keeps its independent crossbar or foot");
        assert.ok(ink.every((line) => line.grapheme === glyph));
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(ink)), ink);
        assert.throws(() => geometricStarter(glyph, 100, 2450, 1100,
            250, "#243647"), /fit inside glyph cell|clips glyph cell/);
    }
    assert.notDeepEqual(geometricStarter("ε", ...args),
        geometricStarter("e", ...args), "epsilon is not Latin e");
    assert.notDeepEqual(geometricStarter("ω", ...args),
        geometricStarter("w", ...args), "omega is not Latin w");
    assert.throws(() => geometricStarter("ϑ", ...args),
        /supported exact geometric glyph/);
});

test("manual Greek kappa and chi preserve separate native pen paths", () => {
    const args = [1550, 2450, 1100, 250, "#243647"];
    for (const [glyph, count, start] of [
        ["κ", 33, [835, 1515, 835, 3385]],
        ["χ", 32, [835, 1570]],
    ]) {
        const ink = geometricStarter(glyph, ...args);
        assert.equal(ink.length, count, `${glyph} sampled stroke count`);
        assert.equal(candidatePenPaths(ink, glyph).length, 2,
            `${glyph} maintains distinct directed pen paths`);
        assert.deepEqual([ink[0].x1, ink[0].y1,
            ...(start.length === 4 ? [ink[0].x2, ink[0].y2] : [])],
        start, `${glyph} first ink is not borrowed`);
        assert.ok(ink.every((line) => line.grapheme === glyph));
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(ink)), ink);
        assert.throws(() => geometricStarter(glyph, 100, 2450, 1100,
            250, "#243647"), /fit inside glyph cell|clips glyph cell/);
    }
    assert.notDeepEqual(geometricStarter("κ", ...args),
        geometricStarter("k", ...args), "kappa is not Latin k");
    assert.notDeepEqual(geometricStarter("χ", ...args),
        geometricStarter("x", ...args), "chi is not Latin x");
    assert.throws(() => geometricStarter("ϑ", ...args),
        /supported exact geometric glyph/);
});

test("manual Greek phi and psi retain separate ink and Unicode", () => {
    const args = [1550, 2450, 1100, 250, "#243647"];
    for (const [glyph, count, first] of [
        ["φ", 65, [1550, 1625]],
        ["ψ", 33, [725, 1680]],
    ]) {
        const ink = geometricStarter(glyph, ...args);
        assert.equal(ink.length, count, `${glyph} native segment count`);
        assert.equal(candidatePenPaths(ink, glyph).length, 2,
            `${glyph} deliberately lifts before its stem`);
        assert.deepEqual([ink[0].x1, ink[0].y1], first,
            `${glyph} retains its authored entry`);
        assert.ok(ink.every((line) => line.grapheme === glyph));
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(ink)), ink);
        assert.throws(() => geometricStarter(glyph, 100, 2450, 1100,
            250, "#243647"), /fit inside glyph cell|clips glyph cell/);
    }
    assert.notDeepEqual(geometricStarter("φ", ...args),
        geometricStarter("o", ...args), "phi is not Latin o");
    assert.notDeepEqual(geometricStarter("ψ", ...args),
        geometricStarter("y", ...args), "psi is not Latin y");
    assert.throws(() => geometricStarter("ϕ", ...args),
        /supported exact geometric glyph/);
});

test("manual Greek zeta and eta keep exact lifted pen paths", () => {
    const args = [1550, 2450, 1100, 250, "#243647"];
    for (const [glyph, count, lifts, first] of [
        ["ζ", 48, 1, [2320, 1515]],
        ["η", 33, 2, [835, 1680]],
    ]) {
        const ink = geometricStarter(glyph, ...args);
        assert.equal(ink.length, count, `${glyph} sampled stroke count`);
        assert.equal(candidatePenPaths(ink, glyph).length, lifts,
            `${glyph} keeps intended pen lift boundaries`);
        assert.deepEqual([ink[0].x1, ink[0].y1], first);
        assert.ok(ink.every((line) => line.grapheme === glyph));
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(ink)), ink);
        assert.throws(() => geometricStarter(glyph, 100, 2450, 1100,
            250, "#243647"), /fit inside glyph cell|clips glyph cell/);
    }
    assert.notDeepEqual(geometricStarter("ζ", ...args),
        geometricStarter("z", ...args), "zeta cannot borrow Latin z");
    assert.notDeepEqual(geometricStarter("η", ...args),
        geometricStarter("n", ...args), "eta cannot borrow Latin n");
    assert.throws(() => geometricStarter("ϑ", ...args),
        /supported exact geometric glyph/);
});

test("manual Greek xi and iota retain exact continuous pen geometry", () => {
    const args = [1550, 2450, 1100, 250, "#243647"];
    for (const [glyph, count, first] of [
        ["ξ", 64, [2210, 1515]],
        ["ι", 32, [1330, 1680]],
    ]) {
        const ink = geometricStarter(glyph, ...args);
        assert.equal(ink.length, count, `${glyph} native segment count`);
        assert.equal(candidatePenPaths(ink, glyph).length, 1,
            `${glyph} stays one continuous directed pen path`);
        assert.deepEqual([ink[0].x1, ink[0].y1], first);
        assert.ok(ink.every((line) => line.grapheme === glyph));
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(ink)), ink);
        assert.throws(() => geometricStarter(glyph, 100, 2450, 1100,
            250, "#243647"), /fit inside glyph cell|clips glyph cell/);
    }
    assert.notDeepEqual(geometricStarter("ξ", ...args),
        geometricStarter("z", ...args), "xi cannot borrow Latin z");
    assert.notDeepEqual(geometricStarter("ι", ...args),
        geometricStarter("i", ...args), "iota has no Latin i dot");
    assert.throws(() => geometricStarter("Ϡ", ...args),
        /supported exact geometric glyph/);
});

test("manual Greek omicron and upsilon keep separate exact pen paths", () => {
    const args = [1550, 2450, 1100, 250, "#243647"];
    const all = [];
    for (const [glyph, count, first] of [
        ["ο", 64, [1550, 1625]],
        ["υ", 32, [780, 1680]],
    ]) {
        const ink = geometricStarter(glyph, ...args);
        assert.equal(ink.length, count, glyph + " exact pen count");
        assert.equal(candidatePenPaths(ink, glyph).length, 1,
            glyph + " has one unbroken directed pen path");
        assert.deepEqual([ink[0].x1, ink[0].y1], first);
        assert.ok(ink.every((line) => line.grapheme === glyph));
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(ink)), ink);
        assert.throws(() => geometricStarter(glyph, 100, 2450, 1100,
            250, "#243647"), /fit inside glyph cell|clips glyph cell/);
        all.push(...ink);
    }
    assert.equal(all.length, 96);
    assert.notDeepEqual(geometricStarter("ο", ...args),
        geometricStarter("o", ...args), "omicron is not Latin o");
    assert.notDeepEqual(geometricStarter("υ", ...args),
        geometricStarter("u", ...args), "upsilon is not Latin u");
    assert.throws(() => geometricStarter("ϑ", ...args),
        /supported exact geometric glyph/);
});

test("manual Greek nu and final sigma preserve separate joined ink", () => {
    const args = [1550, 2450, 1100, 250, "#243647"];
    for (const [glyph, count, first] of [
        ["ν", 32, [780, 1735]],
        ["ς", 48, [2265, 1735]],
    ]) {
        const ink = geometricStarter(glyph, ...args);
        assert.equal(ink.length, count, glyph + " native chord count");
        assert.equal(candidatePenPaths(ink, glyph).length, 1,
            glyph + " remains one uninterrupted pen gesture");
        assert.deepEqual([ink[0].x1, ink[0].y1], first);
        assert.ok(ink.every((segment) => segment.grapheme === glyph));
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(ink)), ink);
        assert.throws(() => geometricStarter(glyph, 100, 2450, 1100,
            250, "#243647"), /fit inside glyph cell|clips glyph cell/);
    }
    assert.notDeepEqual(geometricStarter("ν", ...args),
        geometricStarter("v", ...args), "Greek nu is not Latin v");
    assert.notDeepEqual(geometricStarter("ς", ...args),
        geometricStarter("σ", ...args), "final sigma is not medial sigma");
    assert.notDeepEqual(geometricStarter("ς", ...args),
        geometricStarter("s", ...args), "final sigma is not Latin s");
    assert.throws(() => geometricStarter("ϑ", ...args),
        /supported exact geometric glyph/);
});

test("manual uppercase Greek Gamma and Omega keep exact distinct ink", () => {
    const args = [1550, 2450, 1100, 250, "#243647"];
    for (const [glyph, count, paths, first] of [
        ["Γ", 2, 1, [780, 3385]],
        ["Ω", 66, 3, [725, 3330]],
    ]) {
        const ink = geometricStarter(glyph, ...args);
        assert.equal(ink.length, count, glyph + " segment count");
        assert.equal(candidatePenPaths(ink, glyph).length, paths,
            glyph + " preserves deliberate pen lift boundaries");
        assert.deepEqual([ink[0].x1, ink[0].y1], first);
        assert.ok(ink.every((line) => line.grapheme === glyph));
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(ink)), ink);
        assert.throws(() => geometricStarter(glyph, 100, 2450, 1100,
            250, "#243647"), /fit inside glyph cell|clips glyph cell/);
    }
    assert.notDeepEqual(geometricStarter("Γ", ...args),
        geometricStarter("L", ...args), "Gamma is not Latin L");
    assert.notDeepEqual(geometricStarter("Ω", ...args),
        geometricStarter("ω", ...args), "uppercase Omega is not lowercase");
    assert.throws(() => geometricStarter("Ϡ", ...args),
        /supported exact geometric glyph/);
});

test("manual uppercase Greek Pi and Sigma preserve exact pen order", () => {
    const args = [1550, 2450, 1100, 250, "#243647"];
    for (const [glyph, coords, first] of [
        ["Π", 3, [780, 3385]],
        ["Σ", 4, [2375, 1515]],
    ]) {
        const ink = geometricStarter(glyph, ...args);
        assert.equal(ink.length, coords, glyph + " native chord count");
        assert.equal(candidatePenPaths(ink, glyph).length, 1,
            glyph + " must keep joined corners without pen lifts");
        assert.deepEqual([ink[0].x1, ink[0].y1], first);
        assert.ok(ink.every((line) => line.grapheme === glyph));
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(ink)), ink);
        assert.throws(() => geometricStarter(glyph, 100, 2450, 1100,
            250, "#243647"), /fit inside glyph cell|clips glyph cell/);
    }
    assert.notDeepEqual(geometricStarter("Π", ...args),
        geometricStarter("Γ", ...args), "Pi is not capital Gamma");
    assert.notDeepEqual(geometricStarter("Σ", ...args),
        geometricStarter("∑", ...args), "capital Sigma is not sum symbol");
    assert.throws(() => geometricStarter("Ϡ", ...args),
        /supported exact geometric glyph/);
});

test("manual Greek capital Lambda and Theta retain exact ordered ink", () => {
    const args = [1550, 2450, 1100, 250, "#243647"];
    for (const [glyph, count, paths, first] of [
        ["Λ", 2, 1, [780, 3385]],
        ["Θ", 65, 2, [1550, 1515]],
    ]) {
        const ink = geometricStarter(glyph, ...args);
        assert.equal(ink.length, count, glyph + " native ink count");
        assert.equal(candidatePenPaths(ink, glyph).length, paths,
            glyph + " keeps intended pen lifts");
        assert.deepEqual([ink[0].x1, ink[0].y1], first);
        assert.ok(ink.every((line) => line.grapheme === glyph));
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(ink)), ink);
        assert.throws(() => geometricStarter(glyph, 100, 2450, 1100,
            250, "#243647"), /fit inside glyph cell|clips glyph cell/);
    }
    assert.notDeepEqual(geometricStarter("Λ", ...args),
        geometricStarter("λ", ...args), "capital Lambda keeps exact identity");
    assert.notDeepEqual(geometricStarter("Θ", ...args),
        geometricStarter("θ", ...args), "capital Theta is not lowercase");
    assert.throws(() => geometricStarter("Ϡ", ...args),
        /supported exact geometric glyph/);
});

test("manual uppercase Greek Xi and Phi keep distinct ink", () => {
    const args = [1550, 2450, 1100, 250, "#243647"];
    for (const [glyph, count, paths, first] of [
        ["Ξ", 3, 3, [780, 1625]],
        ["Φ", 65, 2, [1550, 1658]],
    ]) {
        const ink = geometricStarter(glyph, ...args);
        assert.equal(ink.length, count, glyph + " native ink count");
        assert.equal(candidatePenPaths(ink, glyph).length, paths,
            glyph + " preserves exact pen lifts");
        assert.deepEqual([ink[0].x1, ink[0].y1], first);
        assert.ok(ink.every((line) => line.grapheme === glyph));
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(ink)), ink);
        assert.throws(() => geometricStarter(glyph, 100, 2450, 1100,
            250, "#243647"), /fit inside glyph cell|clips glyph cell/);
    }
    assert.notDeepEqual(geometricStarter("Ξ", ...args),
        geometricStarter("≡", ...args), "Xi is not identity operator");
    assert.notDeepEqual(geometricStarter("Φ", ...args),
        geometricStarter("φ", ...args), "capital Phi is not lowercase");
    assert.throws(() => geometricStarter("Ϡ", ...args),
        /supported exact geometric glyph/);
});

test("manual capital Greek Psi and Chi have exact lifted ink", () => {
    const args = [1550, 2450, 1100, 250, "#243647"];
    for (const [glyph, count, first] of [
        ["Ψ", 33, [780, 1570]],
        ["Χ", 2, [780, 1515]],
    ]) {
        const ink = geometricStarter(glyph, ...args);
        assert.equal(ink.length, count, glyph + " native segment count");
        assert.equal(candidatePenPaths(ink, glyph).length, 2,
            glyph + " must lift between independent pen paths");
        assert.deepEqual([ink[0].x1, ink[0].y1], first);
        assert.ok(ink.every((line) => line.grapheme === glyph));
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(ink)), ink);
        assert.throws(() => geometricStarter(glyph, 100, 2450, 1100,
            250, "#243647"), /fit inside glyph cell|clips glyph cell/);
    }
    assert.notDeepEqual(geometricStarter("Ψ", ...args),
        geometricStarter("ψ", ...args), "capital Psi is not lowercase");
    assert.notDeepEqual(geometricStarter("Χ", ...args),
        geometricStarter("χ", ...args), "capital Chi is not lowercase");
    assert.throws(() => geometricStarter("Ϡ", ...args),
        /supported exact geometric glyph/);
});

test("manual Greek capitals Beta and Upsilon keep exact lifted ink", () => {
    const args = [1550, 2450, 1100, 250, "#243647"];
    for (const [glyph, count, first] of [
        ["Β", 65, [835, 1515]],
        ["Υ", 3, [780, 1515]],
    ]) {
        const ink = geometricStarter(glyph, ...args);
        assert.equal(ink.length, count, glyph + " native segment count");
        assert.equal(candidatePenPaths(ink, glyph).length, 2,
            glyph + " preserves independently lifted pen paths");
        assert.deepEqual([ink[0].x1, ink[0].y1], first);
        assert.ok(ink.every((line) => line.grapheme === glyph));
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(ink)), ink);
        assert.throws(() => geometricStarter(glyph, 100, 2450, 1100,
            250, "#243647"), /fit inside glyph cell|clips glyph cell/);
    }
    assert.notDeepEqual(geometricStarter("Β", ...args),
        geometricStarter("β", ...args), "Beta is not lowercase beta");
    assert.notDeepEqual(geometricStarter("Β", ...args),
        geometricStarter("B", ...args), "Greek Beta is not Latin B");
    assert.notDeepEqual(geometricStarter("Υ", ...args),
        geometricStarter("υ", ...args), "capital Upsilon is not lowercase");
    assert.notDeepEqual(geometricStarter("Υ", ...args),
        geometricStarter("Y", ...args), "Greek Upsilon is not Latin Y");
});

test("manual Greek capitals Alpha and Epsilon keep unique pen paths", () => {
    const args = [1550, 2450, 1100, 250, "#243647"];
    for (const [glyph, count, first] of [
        ["Α", 3, [780, 3385]],
        ["Ε", 4, [2265, 1515]],
    ]) {
        const ink = geometricStarter(glyph, ...args);
        assert.equal(ink.length, count, glyph + " sampled segments");
        assert.equal(candidatePenPaths(ink, glyph).length, 2,
            glyph + " retains one deliberate lift");
        assert.deepEqual([ink[0].x1, ink[0].y1], first);
        assert.ok(ink.every((stroke) => stroke.grapheme === glyph));
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(ink)), ink);
        assert.throws(() => geometricStarter(glyph, 100, 2450, 1100,
            250, "#243647"), /fit inside glyph cell|clips glyph cell/);
    }
    assert.notDeepEqual(geometricStarter("Α", ...args),
        geometricStarter("A", ...args), "Greek Alpha is not Latin A");
    assert.notDeepEqual(geometricStarter("Ε", ...args),
        geometricStarter("E", ...args), "Greek Epsilon is not Latin E");
    assert.notDeepEqual(geometricStarter("Α", ...args),
        geometricStarter("α", ...args), "capital is not lowercase Alpha");
    assert.notDeepEqual(geometricStarter("Ε", ...args),
        geometricStarter("ε", ...args), "capital is not lowercase Epsilon");
    assert.throws(() => geometricStarter("Ϡ", ...args),
        /supported exact geometric glyph/);
});

test("manual Greek capitals Iota and Omicron preserve exact pen paths", () => {
    const args = [1550, 2450, 1100, 250, "#243647"];
    for (const [glyph, count, first] of [
        ["Ι", 1, [1550, 1515]],
        ["Ο", 64, [1550, 1570]],
    ]) {
        const ink = geometricStarter(glyph, ...args);
        assert.equal(ink.length, count);
        assert.equal(candidatePenPaths(ink, glyph).length, 1,
            glyph + " draws one uninterrupted exact path");
        assert.deepEqual([ink[0].x1, ink[0].y1], first);
        assert.ok(ink.every((stroke) => stroke.grapheme === glyph));
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(ink)), ink);
        assert.throws(() => geometricStarter(glyph, 100, 2450, 1100,
            250, "#243647"), /fit inside glyph cell|clips glyph cell/);
    }
    assert.notDeepEqual(geometricStarter("Ι", ...args),
        geometricStarter("I", ...args), "capital Iota differs from Latin I");
    assert.notDeepEqual(geometricStarter("Ο", ...args),
        geometricStarter("O", ...args), "capital Omicron differs from O");
    assert.notDeepEqual(geometricStarter("Ι", ...args),
        geometricStarter("ι", ...args), "capital Iota differs from lower");
    assert.notDeepEqual(geometricStarter("Ο", ...args),
        geometricStarter("ο", ...args), "capital Omicron differs from lower");
});

test("manual Greek capitals Zeta and Eta keep ordered lifted ink", () => {
    const args = [1550, 2450, 1100, 250, "#243647"];
    for (const [glyph, first, lifts] of [
        ["Ζ", [780, 1515], 1],
        ["Η", [780, 1515], 3],
    ]) {
        const ink = geometricStarter(glyph, ...args);
        assert.equal(ink.length, 3);
        assert.deepEqual([ink[0].x1, ink[0].y1], first);
        assert.equal(candidatePenPaths(ink, glyph).length, lifts);
        assert.ok(ink.every((stroke) => stroke.grapheme === glyph));
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(ink)), ink);
        assert.throws(() => geometricStarter(glyph, 100, 2450,
            1100, 250, "#243647"),
        /fit inside glyph cell|clips glyph cell/);
    }
    assert.notDeepEqual(geometricStarter("Ζ", ...args),
        geometricStarter("ζ", ...args), "capital and lower zeta differ");
    assert.notDeepEqual(geometricStarter("Η", ...args),
        geometricStarter("η", ...args), "capital and lower eta differ");
    assert.throws(() => geometricStarter("Ϡ", ...args),
        /supported exact geometric glyph/);
});

test("remaining Greek capitals keep exact pen routes and inventory", () => {
    const args = [1550, 2450, 1100, 250, "#243647"];
    for (const [glyph, count, first, lifts] of [
        ["Κ", 3, [780, 1515], 2],
        ["Μ", 4, [780, 3385], 1],
        ["Ν", 3, [780, 3385], 1],
        ["Ρ", 33, [835, 1515], 2],
        ["Τ", 2, [725, 1515], 2],
    ]) {
        const ink = geometricStarter(glyph, ...args);
        assert.equal(ink.length, count, glyph + " sampled chords");
        assert.deepEqual([ink[0].x1, ink[0].y1], first);
        assert.equal(candidatePenPaths(ink, glyph).length, lifts);
        assert.ok(ink.every((stroke) => stroke.grapheme === glyph));
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(ink)), ink);
        assert.throws(() => geometricStarter(glyph, 100, 2450,
            1100, 250, "#243647"), /fit inside glyph cell/);
    }
    const capitals = [..."ΑΒΓΔΕΖΗΘΙΚΛΜΝΞΟΠΡΣΤΥΦΧΨΩ"];
    assert.equal(capitals.length, 24);
    const ink = capitals.flatMap((glyph) => geometricStarter(
        glyph, ...args));
    assert.deepEqual([...new Set(ink.map((line) => line.grapheme))],
        capitals, "each Greek uppercase must have exact authored ink");
    assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(ink)), ink);
    assert.throws(() => geometricStarter("Ϡ", ...args),
        /supported exact geometric glyph/);
});

test("n-ary product and coproduct have independent lifted paths", () => {
    const args = [1550, 2450, 1100, 250, "#243647"];
    for (const [glyph, first] of [
        ["∏", [615, 1515]],
        ["∐", [615, 3385]],
    ]) {
        const ink = geometricStarter(glyph, ...args);
        assert.equal(ink.length, 5, glyph + " directed pen segments");
        assert.equal(candidatePenPaths(ink, glyph).length, 5);
        assert.deepEqual([ink[0].x1, ink[0].y1], first);
        assert.ok(ink.every((segment) => segment.grapheme === glyph));
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(ink)), ink);
        assert.throws(() => geometricStarter(glyph, 100, 2450, 1100,
            250, "#243647"), /fit inside glyph cell/);
    }
    assert.notDeepEqual(geometricStarter("∏", ...args),
        geometricStarter("Π", ...args),
        "n-ary product cannot borrow uppercase Greek Pi ink");
    assert.notDeepEqual(geometricStarter("∐", ...args),
        geometricStarter("∏", ...args),
        "n-ary coproduct has an independently inverted cap");
});

test("manual contour and repeated integrals retain exact joined paths", () => {
    const args = [1550, 2450, 1100, 250, "#243647"];
    const expected = [
        ["∮", 112, 2], ["∬", 96, 2], ["∭", 144, 3],
    ];
    for (const [glyph, count, paths] of expected) {
        const ink = geometricStarter(glyph, ...args);
        assert.equal(ink.length, count, glyph + " native chords");
        assert.equal(candidatePenPaths(ink, glyph).length, paths,
            glyph + " independently joined pen paths");
        assert.ok(ink.every((line) => line.grapheme === glyph));
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(ink)), ink);
        assert.throws(() => geometricStarter(glyph, 100, 2450, 1100,
            250, "#243647"), /fit inside glyph cell/);
        assert.notDeepEqual(ink, geometricStarter("∫", ...args),
            glyph + " cannot borrow the single integral");
    }
    const [contour, double, triple] = expected.map(([glyph]) =>
        geometricStarter(glyph, ...args));
    assert.deepEqual(contour.slice(0, 48),
        geometricStarter("∫", ...args)
            .map((line) => ({ ...line, grapheme: "∮" })),
    "contour integral retains one uninterrupted base integral");
    assert.notDeepEqual(double.slice(0, 48), triple.slice(0, 48),
        "two- and three-fold integrals have distinct deliberate spacing");
    assert.throws(() => geometricStarter("∯", ...args),
        /supported exact geometric glyph/,
        "an unsupplied lookalike must not silently inherit ink");
});

test("blackboard number sets retain distinct directed Unicode ink", () => {
    const args = [1550, 2450, 1100, 250, "#243647"];
    for (const [glyph, count, paths, latin] of [
        ["ℕ", 4, 2, "N"], ["ℤ", 4, 2, "Z"],
        ["ℚ", 66, 3, "Q"], ["ℝ", 35, 2, "R"],
        ["ℂ", 65, 2, "C"],
    ]) {
        const ink = geometricStarter(glyph, ...args);
        assert.equal(ink.length, count, glyph + " sampled pen segments");
        assert.equal(candidatePenPaths(ink, glyph).length, paths,
            glyph + " connected paths retain intended lifts");
        assert.ok(ink.every((line) => line.grapheme === glyph));
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(ink)), ink);
        assert.notDeepEqual(ink, geometricStarter(latin, ...args),
            glyph + " must not silently inherit the Latin shape");
        assert.throws(() => geometricStarter(glyph, 100, 2450, 1100,
            250, "#243647"), /fit inside glyph cell/);
    }
    assert.throws(() => geometricStarter("ℍ", ...args),
        /supported exact geometric glyph/,
        "unauthored mathematical alphabet variants require manual ink");
});

test("indexed radicals preserve exact native Unicode paths", () => {
    const args = [1550, 2450, 1100, 250, "#243647"];
    for (const [glyph, count, paths] of [
        ["∛", 36, 2], ["∜", 7, 3],
    ]) {
        const ink = geometricStarter(glyph, ...args);
        assert.equal(ink.length, count);
        assert.equal(candidatePenPaths(ink, glyph).length, paths);
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(ink)), ink);
        assert.ok(ink.every((line) => line.grapheme === glyph));
        assert.throws(() => geometricStarter(glyph, 100, 2450, 1100,
            250, "#243647"), /fit inside glyph cell/);
        assert.notDeepEqual(ink, geometricStarter("√", ...args),
            "indexed radical cannot borrow square-root ink");
    }
    assert.notDeepEqual(geometricStarter("∛", ...args),
        geometricStarter("∜", ...args), "index number is independently inked");
    assert.throws(() => geometricStarter("∯", ...args),
        /supported exact geometric glyph/);
});

test("manual circled operators retain separate ring and interior ink", () => {
    const args = [1550, 2450, 1100, 250, "#243647"];
    for (const [glyph, count, paths] of [
        ["⊕", 66, 3], ["⊗", 66, 3], ["⊙", 65, 2],
    ]) {
        const ink = geometricStarter(glyph, ...args);
        assert.equal(ink.length, count, glyph + " native segments");
        assert.equal(candidatePenPaths(ink, glyph).length, paths,
            glyph + " joined outline and deliberate interior pen lifts");
        assert.deepEqual([ink[0].x1, ink[0].y1], [1550, 1548],
            "the closed ring starts at its top point");
        assert.deepEqual(
            [ink[63].x2, ink[63].y2], [ink[0].x1, ink[0].y1],
            "four sampled quadratic curves close the outer ring");
        assert.ok(ink.every((line) => line.grapheme === glyph));
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(ink)), ink);
        assert.throws(() => geometricStarter(glyph, 100, 2450, 1100,
            250, "#243647"), /fit inside glyph cell/);
        assert.throws(() => geometricStarter(glyph, 1550, 2450, 200,
            250, "#243647"), /pen-clearance/);
    }
    for (const [glyph, expected] of [
        ["⊕", [[1077, 2450, 2023, 2450],
            [1550, 1977, 1550, 2923]]],
        ["⊗", [[1198, 2098, 1902, 2802],
            [1198, 2802, 1902, 2098]]],
        ["⊙", [[1550, 2362, 1550, 2538]]],
    ]) {
        const ink = geometricStarter(glyph, ...args);
        assert.deepEqual(ink.slice(64).map(({ x1, y1, x2, y2 }) =>
            [x1, y1, x2, y2]), expected,
            glyph + " retains an independently directed interior mark");
    }
    assert.notDeepEqual(geometricStarter("⊕", ...args),
        geometricStarter("+", ...args),
        "circled plus must not be borrowed from ordinary plus");
    assert.notDeepEqual(geometricStarter("⊗", ...args),
        geometricStarter("×", ...args),
        "circled times must retain its own ring");
    assert.throws(() => geometricStarter("⊚", ...args),
        /supported exact geometric glyph/,
        "an unauthored circled lookalike is not an alias");
});

test("circled minus and divide keep separate inner ink", () => {
    const args = [1550, 2450, 1100, 250, "#243647"];
    for (const [glyph, interior] of [
        ["⊖", [1077, 2450, 2023, 2450]],
        ["⊘", [1110, 2890, 1990, 2010]],
    ]) {
        const ink = geometricStarter(glyph, ...args);
        assert.equal(ink.length, 65, glyph + " sampled ink chords");
        assert.deepEqual(candidatePenPaths(ink, glyph).map(
            ({ first, last }) => [first, last]),
        [[1, 64], [65, 65]], "ring and stroke retain one pen lift");
        assert.deepEqual([ink[0].x1, ink[0].y1,
            ink[63].x2, ink[63].y2], [1550, 1548, 1550, 1548]);
        assert.deepEqual([ink[64].x1, ink[64].y1,
            ink[64].x2, ink[64].y2], interior,
        "minus and slash are independently directed inner strokes");
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(ink)), ink);
        assert.throws(() => geometricStarter(glyph, 100, 2450,
            1100, 250, "#243647"), /fit inside glyph cell/);
        assert.throws(() => geometricStarter(glyph, 1550, 2450,
            200, 250, "#243647"), /pen-clearance/);
    }
    assert.notDeepEqual(geometricStarter("⊖", ...args),
        geometricStarter("-", ...args));
    assert.notDeepEqual(geometricStarter("⊘", ...args),
        geometricStarter("÷", ...args));
    assert.throws(() => geometricStarter("⦸", ...args),
        /supported exact geometric glyph/);
});

test("big set union and intersection have distinct joined contours", () => {
    const args = [1550, 2450, 1100, 250, "#243647"];
    for (const [glyph, small, first] of [
        ["⋂", "∩", [692, 3407]],
        ["⋃", "∪", [692, 1493]],
    ]) {
        const ink = geometricStarter(glyph, ...args);
        assert.equal(ink.length, 34, glyph + " native segments");
        assert.equal(candidatePenPaths(ink, glyph).length, 1,
            "both stems and bow form one connected pen path");
        assert.deepEqual([ink[0].x1, ink[0].y1], first);
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(ink)), ink);
        assert.ok(ink.every((line) => line.grapheme === glyph));
        assert.notDeepEqual(ink, geometricStarter(small, ...args),
            "the n-ary operator has separate Unicode and taller geometry");
        assert.throws(() => geometricStarter(glyph, 100, 2450,
            1100, 250, "#243647"), /fit inside glyph cell/);
    }
    assert.throws(() => geometricStarter("⋏", ...args),
        /supported exact geometric glyph/);
});

test("n-ary logical wedges retain exact wide joined strokes", () => {
    const args = [1550, 2450, 1100, 250, "#243647"];
    for (const [glyph, small, expected] of [
        ["⋀", "∧", [[626, 3495, 1550, 1405],
            [1550, 1405, 2474, 3495]]],
        ["⋁", "∨", [[626, 1405, 1550, 3495],
            [1550, 3495, 2474, 1405]]],
    ]) {
        const ink = geometricStarter(glyph, ...args);
        assert.deepEqual(ink.map(({ x1, y1, x2, y2 }) =>
            [x1, y1, x2, y2]), expected,
            "n-ary wedges use a wide, connected two-arm pen contour");
        assert.equal(candidatePenPaths(ink, glyph).length, 1);
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(ink)), ink);
        assert.ok(ink.every((line) => line.grapheme === glyph));
        assert.notDeepEqual(ink, geometricStarter(small, ...args),
            "regular logical wedge must not supply n-ary ink");
        assert.throws(() => geometricStarter(glyph, 100, 2450,
            1100, 250, "#243647"), /fit inside glyph cell/);
    }
    assert.throws(() => geometricStarter("⋏", ...args),
        /supported exact geometric glyph/);
});

test("square lattice meet and join retain three connected sides", () => {
    const args = [1550, 2450, 1100, 250, "#243647"];
    for (const [glyph, rounded, expected] of [
        ["⊓", "∩", [[725, 3242, 725, 1658],
            [725, 1658, 2375, 1658], [2375, 1658, 2375, 3242]]],
        ["⊔", "∪", [[725, 1658, 725, 3242],
            [725, 3242, 2375, 3242], [2375, 3242, 2375, 1658]]],
    ]) {
        const ink = geometricStarter(glyph, ...args);
        assert.deepEqual(ink.map(({ x1, y1, x2, y2 }) =>
            [x1, y1, x2, y2]), expected,
            "square lattice contour keeps three directed sides");
        assert.deepEqual(candidatePenPaths(ink, glyph).map(
            ({ first, last }) => [first, last]), [[1, 3]]);
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(ink)), ink);
        assert.ok(ink.every((line) => line.grapheme === glyph));
        assert.notDeepEqual(ink, geometricStarter(rounded, ...args));
        assert.throws(() => geometricStarter(glyph, 100, 2450,
            1100, 250, "#243647"), /fit inside glyph cell/);
    }
    assert.throws(() => geometricStarter("⊝", ...args),
        /supported exact geometric glyph/);
});

test("square subset and superset retain exact directed contours", () => {
    const args = [1550, 2450, 1100, 250, "#243647"];
    const cases = [
        ["⊏", [[2342, 1768, 758, 1768], [758, 1768, 758, 3132],
            [758, 3132, 2342, 3132]], [[1, 3]]],
        ["⊐", [[758, 1768, 2342, 1768], [2342, 1768, 2342, 3132],
            [2342, 3132, 758, 3132]], [[1, 3]]],
        ["⊑", [[2342, 1768, 758, 1768], [758, 1768, 758, 3132],
            [758, 3132, 2342, 3132], [758, 3495, 2342, 3495]],
        [[1, 3], [4, 4]]],
        ["⊒", [[758, 1768, 2342, 1768], [2342, 1768, 2342, 3132],
            [2342, 3132, 758, 3132], [2342, 3495, 758, 3495]],
        [[1, 3], [4, 4]]],
    ];
    for (const [glyph, expected, paths] of cases) {
        const ink = geometricStarter(glyph, ...args);
        assert.deepEqual(ink.map(({ x1, y1, x2, y2 }) =>
            [x1, y1, x2, y2]), expected);
        assert.deepEqual(candidatePenPaths(ink, glyph).map(
            ({ first, last }) => [first, last]), paths,
            "the optional underbar requires a deliberate pen lift");
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(ink)), ink);
        assert.ok(ink.every((line) => line.grapheme === glyph));
        assert.throws(() => geometricStarter(glyph, 100, 2450,
            1100, 250, "#243647"), /fit inside glyph cell/);
    }
    assert.notDeepEqual(geometricStarter("⊏", ...args),
        geometricStarter("⊂", ...args),
        "square subset cannot borrow rounded subset ink");
    assert.throws(() => geometricStarter("⊑", 1550, 2450,
        500, 250, "#243647"), /pen-clearance/);
    assert.throws(() => geometricStarter("⊝", ...args),
        /supported exact geometric glyph/);
});

test("square operators preserve closed border and lifted inner marks", () => {
    const args = [1550, 2450, 1100, 250, "#243647"];
    const border = [[692, 1592, 2408, 1592],
        [2408, 1592, 2408, 3308], [2408, 3308, 692, 3308],
        [692, 3308, 692, 1592]];
    const cases = [
        ["⊞", [[1110, 2450, 1990, 2450],
            [1550, 2010, 1550, 2890]]],
        ["⊟", [[1110, 2450, 1990, 2450]]],
        ["⊠", [[1110, 2010, 1990, 2890],
            [1990, 2010, 1110, 2890]]],
        ["⊡", [[1484, 2450, 1616, 2450]]],
    ];
    for (const [glyph, inner] of cases) {
        const ink = geometricStarter(glyph, ...args);
        assert.deepEqual(ink.map(({ x1, y1, x2, y2 }) =>
            [x1, y1, x2, y2]), [...border, ...inner]);
        const paths = candidatePenPaths(ink, glyph);
        assert.deepEqual(paths.map(({ first, last }) => [first, last]),
            [[1, 4], ...inner.map((_, i) => [i + 5, i + 5])],
            "border closes without lifting, inner strokes lift exactly");
        assert.deepEqual(paths[0].start, paths[0].end);
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(ink)), ink);
        assert.ok(ink.every((line) => line.grapheme === glyph));
        assert.throws(() => geometricStarter(glyph, 100, 2450,
            1100, 250, "#243647"), /fit inside glyph cell/);
        assert.throws(() => geometricStarter(glyph, 1550, 2450,
            500, 250, "#243647"), /pen-clearance/);
    }
    assert.notDeepEqual(geometricStarter("⊞", ...args),
        geometricStarter("⊕", ...args),
        "square enclosure must not borrow circled ink");
    assert.throws(() => geometricStarter("⊝", ...args),
        /supported exact geometric glyph/);
});

test("precedence geometry has smooth pen paths and lifted bars", () => {
    const args = [1550, 2450, 1100, 250, "#243647"];
    const cases = [
        ["≺", [2265, 1735, 758, 2450, 2265, 3165], 32],
        ["≻", [835, 1735, 2342, 2450, 835, 3165], 32],
        ["≼", [2265, 1735, 758, 2450, 2265, 3165], 33],
        ["≽", [835, 1735, 2342, 2450, 835, 3165], 33],
    ];
    for (const [glyph, endpoints, count] of cases) {
        const ink = geometricStarter(glyph, ...args);
        assert.equal(ink.length, count);
        assert.deepEqual([
            ink[0].x1, ink[0].y1,
            ink[15].x2, ink[15].y2,
            ink[31].x2, ink[31].y2,
        ], endpoints, "both quadratic arms meet at an exact vertex");
        assert.deepEqual(candidatePenPaths(ink, glyph).map(
            ({ first, last }) => [first, last]),
        count === 33 ? [[1, 32], [33, 33]] : [[1, 32]]);
        if (count === 33) {
            assert.deepEqual([
                ink[32].x1, ink[32].y1,
                ink[32].x2, ink[32].y2,
            ], [835, 3506, 2265, 3506]);
        }
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(ink)), ink);
        assert.ok(ink.every((segment) => segment.grapheme === glyph));
        assert.throws(() => geometricStarter(glyph, 100, 2450,
            1100, 250, "#243647"), /fit inside glyph cell/);
    }
    assert.notDeepEqual(geometricStarter("≺", ...args),
        geometricStarter("<", ...args),
        "curved precedence must not reuse a strict comparison glyph");
    assert.notDeepEqual(geometricStarter("≼", ...args),
        geometricStarter("⊆", ...args),
        "precedence-or-equal cannot alias subset-or-equal ink");
    assert.throws(() => geometricStarter("≼", 1550, 2450,
        500, 250, "#243647"), /pen-clearance/);
    assert.throws(() => geometricStarter("≽", 1550, 2450,
        500, 250, "#243647"), /pen-clearance/);
    assert.throws(() => geometricStarter("≾", ...args),
        /supported exact geometric glyph/);
});

test("normal subgroup relations keep closed triangles and lifted bars", () => {
    const args = [1550, 2450, 1100, 250, "#243647"];
    const cases = [
        ["⊲", [[2298, 1735, 758, 2450], [758, 2450, 2298, 3165],
            [2298, 3165, 2298, 1735]], false],
        ["⊳", [[802, 1735, 2342, 2450], [2342, 2450, 802, 3165],
            [802, 3165, 802, 1735]], false],
        ["⊴", [[2298, 1735, 758, 2450], [758, 2450, 2298, 3165],
            [2298, 3165, 2298, 1735]], true],
        ["⊵", [[802, 1735, 2342, 2450], [2342, 2450, 802, 3165],
            [802, 3165, 802, 1735]], true],
    ];
    for (const [glyph, sides, inclusive] of cases) {
        const ink = geometricStarter(glyph, ...args);
        assert.deepEqual(ink.slice(0, 3).map(({ x1, y1, x2, y2 }) =>
            [x1, y1, x2, y2]), sides);
        assert.equal(ink.length, inclusive ? 4 : 3);
        const paths = candidatePenPaths(ink, glyph);
        assert.deepEqual(paths.map(({ first, last }) => [first, last]),
            inclusive ? [[1, 3], [4, 4]] : [[1, 3]]);
        assert.deepEqual(paths[0].start, paths[0].end,
            "triangle is one closed pen-down contour");
        if (inclusive) {
            assert.deepEqual([ink[3].x1, ink[3].y1,
                ink[3].x2, ink[3].y2], [802, 3506, 2298, 3506]);
        }
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(ink)), ink);
        assert.ok(ink.every((segment) => segment.grapheme === glyph));
        assert.throws(() => geometricStarter(glyph, 100, 2450,
            1100, 250, "#243647"), /fit inside glyph cell/);
    }
    assert.notDeepEqual(geometricStarter("⊲", ...args),
        geometricStarter("≺", ...args));
    assert.notDeepEqual(geometricStarter("⊴", ...args),
        geometricStarter("⊑", ...args));
    assert.throws(() => geometricStarter("⊴", 1550, 2450,
        500, 250, "#243647"), /pen-clearance/);
    assert.throws(() => geometricStarter("⊵", 1550, 2450,
        500, 250, "#243647"), /pen-clearance/);
    assert.throws(() => geometricStarter("⋜", ...args),
        /supported exact geometric glyph/);
});

test("dotted and triple comparisons preserve mirrored lifted ink", () => {
    const args = [1550, 2450, 1100, 250, "#243647"];
    const cases = [
        ["⋖", [[2045, 1680, 978, 2450],
            [978, 2450, 2045, 3220], [2430, 2351, 2430, 2549]]],
        ["⋗", [[1055, 1680, 2122, 2450],
            [2122, 2450, 1055, 3220], [670, 2351, 670, 2549]]],
        ["⋘", [[1055, 1658, 725, 2450],
            [725, 2450, 1055, 3242],
            [1715, 1658, 1385, 2450],
            [1385, 2450, 1715, 3242],
            [2375, 1658, 2045, 2450],
            [2045, 2450, 2375, 3242]]],
        ["⋙", [[725, 1658, 1055, 2450],
            [1055, 2450, 725, 3242],
            [1385, 1658, 1715, 2450],
            [1715, 2450, 1385, 3242],
            [2045, 1658, 2375, 2450],
            [2375, 2450, 2045, 3242]]],
    ];
    for (const [glyph, expected] of cases) {
        const ink = geometricStarter(glyph, ...args);
        assert.deepEqual(ink.map(({ x1, y1, x2, y2 }) =>
            [x1, y1, x2, y2]), expected);
        assert.deepEqual(candidatePenPaths(ink, glyph).map(
            ({ first, last }) => [first, last]),
        glyph === "⋖" || glyph === "⋗"
            ? [[1, 2], [3, 3]] : [[1, 2], [3, 4], [5, 6]]);
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(ink)), ink);
        assert.ok(ink.every(({ grapheme }) => grapheme === glyph));
        assert.throws(() => geometricStarter(glyph, 100, 2450,
            1100, 250, "#243647"), /fit inside glyph cell/);
        assert.throws(() => geometricStarter(glyph, 1550, 2450,
            500, 250, "#243647"), /pen-clearance/);
    }
    assert.notDeepEqual(geometricStarter("⋖", ...args),
        geometricStarter("<", ...args));
    assert.notDeepEqual(geometricStarter("⋘", ...args),
        geometricStarter("≪", ...args));
    assert.throws(() => geometricStarter("⋜", ...args),
        /supported exact geometric glyph/);
});

test("stacked comparisons keep distinct pen paths and equality bar", () => {
    const args = [1550, 2450, 1100, 250, "#243647"];
    const lessOverGreater = [[2265, 1482, 1055, 1790],
        [1055, 1790, 2265, 2098], [835, 2802, 2045, 3110],
        [2045, 3110, 835, 3418]];
    const greaterOverLess = [[835, 1482, 2045, 1790],
        [2045, 1790, 835, 2098], [2265, 2802, 1055, 3110],
        [1055, 3110, 2265, 3418]];
    for (const [glyph, contour, equal] of [
        ["≶", lessOverGreater, false],
        ["≷", greaterOverLess, false],
        ["⋚", lessOverGreater, true],
        ["⋛", greaterOverLess, true],
    ]) {
        const ink = geometricStarter(glyph, ...args);
        assert.deepEqual(ink.map(({ x1, y1, x2, y2 }) =>
            [x1, y1, x2, y2]), equal
            ? [...contour, [945, 2450, 2155, 2450]] : contour);
        assert.deepEqual(candidatePenPaths(ink, glyph).map(
            ({ first, last }) => [first, last]), equal
            ? [[1, 2], [3, 4], [5, 5]] : [[1, 2], [3, 4]]);
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(ink)), ink);
        assert.ok(ink.every(({ grapheme }) => grapheme === glyph));
        assert.throws(() => geometricStarter(glyph, 100, 2450,
            1100, 250, "#243647"), /fit inside glyph cell/);
        assert.throws(() => geometricStarter(glyph, 1550, 2450,
            equal ? 500 : 350, 250, "#243647"), /pen-clearance/);
    }
    assert.notDeepEqual(geometricStarter("≶", ...args),
        geometricStarter("≪", ...args));
    assert.notDeepEqual(geometricStarter("⋚", ...args),
        geometricStarter("≤", ...args));
    assert.throws(() => geometricStarter("⋜", ...args),
        /supported exact geometric glyph/);
});

test("floor and ceiling brackets preserve connected corner pen paths", () => {
    const args = [1550, 2450, 1100, 250, "#243647"];
    const cases = [
        ["⌊", [[802, 1625, 802, 3275], [802, 3275, 1990, 3275]]],
        ["⌋", [[2298, 1625, 2298, 3275], [2298, 3275, 1110, 3275]]],
        ["⌈", [[802, 3275, 802, 1625], [802, 1625, 1990, 1625]]],
        ["⌉", [[2298, 3275, 2298, 1625], [2298, 1625, 1110, 1625]]],
    ];
    for (const [glyph, expected] of cases) {
        const ink = geometricStarter(glyph, ...args);
        assert.deepEqual(ink.map(({ x1, y1, x2, y2 }) =>
            [x1, y1, x2, y2]), expected);
        assert.deepEqual(candidatePenPaths(ink, glyph).map(
            ({ first, last }) => [first, last]), [[1, 2]],
        "each corner stays a single directed two-segment pen path");
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(ink)), ink);
        assert.throws(() => geometricStarter(glyph, 100, 2450, 1100,
            250, "#243647"), /fit inside glyph cell|clips glyph cell/);
    }
    assert.notDeepEqual(geometricStarter("⌊", ...args),
        geometricStarter("[", ...args),
        "floor must not borrow ASCII bracket ink");
    assert.notDeepEqual(geometricStarter("⌉", ...args),
        geometricStarter("]", ...args),
        "ceiling must not borrow ASCII bracket ink");
});

test("set difference and nonparallel keep exact lifted math ink", () => {
    const args = [1550, 2450, 1100, 250, "#243647"];
    const cases = [
        ["∖", [[780, 1625, 2320, 3275]], [[1, 1]]],
        ["∦", [[1154, 1515, 1154, 3385],
            [1946, 1515, 1946, 3385], [780, 3220, 2320, 1680]],
        [[1, 1], [2, 2], [3, 3]]],
    ];
    for (const [glyph, expected, paths] of cases) {
        const ink = geometricStarter(glyph, ...args);
        assert.deepEqual(ink.map(({ x1, y1, x2, y2 }) =>
            [x1, y1, x2, y2]), expected);
        assert.deepEqual(candidatePenPaths(ink, glyph).map(
            ({ first, last }) => [first, last]), paths);
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(ink)), ink);
        assert.throws(() => geometricStarter(glyph, 100, 2450, 1100,
            250, "#243647"), /fit inside glyph cell|clips glyph cell/);
    }
    const plain = geometricStarter("∥", ...args);
    const barred = geometricStarter("∦", ...args);
    assert.deepEqual(barred.slice(0, 2), plain.map((stroke) => ({
        ...stroke, grapheme: "∦",
    })), "nonparallel retains both independent native parallel stems");
    assert.notDeepEqual(geometricStarter("∖", ...args),
        geometricStarter("\\", ...args),
        "set difference is not an ASCII backslash glyph");
});

test("mapsto arrows keep directed shafts and distinct stop bars", () => {
    const args = [1550, 2450, 1100, 250, "#243647"];
    for (const [glyph, coords] of [
        ["↦", [[725, 1735, 725, 3165], [725, 2450, 2430, 2450],
            [1770, 1845, 2430, 2450], [2430, 2450, 1770, 3055]]],
        ["↤", [[2375, 1735, 2375, 3165], [2375, 2450, 670, 2450],
            [1330, 1845, 670, 2450], [670, 2450, 1330, 3055]]],
    ]) {
        const ink = geometricStarter(glyph, ...args);
        assert.deepEqual(ink.map(({ x1, y1, x2, y2 }) =>
            [x1, y1, x2, y2]), coords);
        assert.deepEqual(candidatePenPaths(ink, glyph).map(
            ({ first, last }) => [first, last]),
        [[1, 1], [2, 2], [3, 4]],
        "the vertical stop, shaft and arrowhead remain separate pen paths");
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(ink)), ink);
        assert.throws(() => geometricStarter(glyph, 120, 2450, 1100,
            250, "#243647"), /fit inside glyph cell|clips glyph cell/);
    }
    assert.notDeepEqual(geometricStarter("↦", ...args),
        geometricStarter("→", ...args),
        "the right mapsto bar is not a normal right arrow");
    assert.notDeepEqual(geometricStarter("↤", ...args),
        geometricStarter("←", ...args),
        "the left mapsto bar is not a normal left arrow");
});

test("right arrow and proportionality have distinct editable ink", () => {
    const arrow = geometricStarter("→", 1550, 2450, 1100,
        250, "#243647");
    assert.deepEqual(arrow.map(({ x1, y1, x2, y2 }) =>
        [x1, y1, x2, y2]), [
        [670, 2450, 2430, 2450],
        [1770, 1845, 2430, 2450],
        [2430, 2450, 1770, 3055],
    ]);
    assert.deepEqual(candidatePenPaths(arrow, "→").map(
        ({ first, last }) => [first, last]), [[1, 1], [2, 3]]);
    const proportional = geometricStarter("∝", 1550, 2450, 1100,
        250, "#243647");
    assert.equal(proportional.length, 64,
        "four ideal quadratics use 64 sampled native pen chords");
    assert.deepEqual([proportional[0].x1, proportional[0].y1],
        [2375, 1735]);
    assert.deepEqual([proportional.at(-1).x2,
        proportional.at(-1).y2], [2375, 3165]);
    assert.deepEqual(candidatePenPaths(proportional, "∝").map(
        ({ first, last }) => [first, last]), [[1, 64]],
    "proportionality is one continuous directed path");
    for (const glyph of [arrow, proportional]) {
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(glyph)),
            glyph, "separate Unicode symbols roundtrip exact v1 TSV");
    }
    assert.throws(() => geometricStarter("∝", 200, 2450, 1100,
        250, "#243647"), /fit inside glyph cell|clips glyph cell/);
});

test("left and double arrows have independent directed pen paths", () => {
    const options = [
        ["←", [
            [2430, 2450, 670, 2450],
            [1330, 1845, 670, 2450], [670, 2450, 1330, 3055],
        ], [[1, 1], [2, 3]]],
        ["↔", [
            [670, 2450, 2430, 2450],
            [1330, 1845, 670, 2450], [670, 2450, 1330, 3055],
            [1770, 1845, 2430, 2450], [2430, 2450, 1770, 3055],
        ], [[1, 1], [2, 3], [4, 5]]],
    ];
    for (const [glyph, expected, paths] of options) {
        const ink = geometricStarter(glyph, 1550, 2450, 1100,
            250, "#243647");
        assert.deepEqual(ink.map(({ x1, y1, x2, y2 }) =>
            [x1, y1, x2, y2]), expected);
        assert.deepEqual(candidatePenPaths(ink, glyph).map(
            ({ first, last }) => [first, last]), paths);
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(ink)), ink);
        assert.notDeepEqual(ink, geometricStarter("→", 1550, 2450, 1100,
            250, "#243647"), "arrow directions are never aliases");
    }
    assert.throws(() => geometricStarter("↔", 100, 2450, 1100,
        250, "#243647"), /fit inside glyph cell|clips glyph cell/);
});

test("vertical arrows preserve exact directions and separate pen heads",
    () => {
    const options = [
        ["↑", [
            [1550, 3330, 1550, 1570],
            [945, 2230, 1550, 1570], [1550, 1570, 2155, 2230],
        ], [[1, 1], [2, 3]]],
        ["↓", [
            [1550, 1570, 1550, 3330],
            [945, 2670, 1550, 3330], [1550, 3330, 2155, 2670],
        ], [[1, 1], [2, 3]]],
        ["↕", [
            [1550, 1570, 1550, 3330],
            [945, 2230, 1550, 1570], [1550, 1570, 2155, 2230],
            [945, 2670, 1550, 3330], [1550, 3330, 2155, 2670],
        ], [[1, 1], [2, 3], [4, 5]]],
    ];
    for (const [glyph, expected, paths] of options) {
        const ink = geometricStarter(glyph, 1550, 2450, 1100,
            250, "#243647");
        assert.deepEqual(ink.map(({ x1, y1, x2, y2 }) =>
            [x1, y1, x2, y2]), expected);
        assert.deepEqual(candidatePenPaths(ink, glyph).map(
            ({ first, last }) => [first, last]), paths);
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(ink)), ink);
    }
    assert.throws(() => geometricStarter("↕", 1550, 3800, 1100,
        250, "#243647"), /fit inside glyph cell|clips glyph cell/);
});

test("intersection and union have distinct continuous quadratic ink", () => {
    const shared = [1550, 2450, 1100, 250, "#243647"];
    for (const [glyph, start, end] of [
        ["∩", [835, 3165], [2265, 3165]],
        ["∪", [835, 1735], [2265, 1735]],
    ]) {
        const ink = geometricStarter(glyph, ...shared);
        assert.equal(ink.length, 32);
        assert.deepEqual([ink[0].x1, ink[0].y1], start);
        assert.deepEqual([ink.at(-1).x2, ink.at(-1).y2], end);
        assert.deepEqual(candidatePenPaths(ink, glyph).map(
            ({ first, last }) => [first, last]), [[1, 32]]);
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(ink)), ink);
        assert.notDeepEqual(ink, geometricStarter(
            glyph === "∩" ? "∧" : "∨", ...shared));
    }
    assert.throws(() => geometricStarter("∩", 120, 2450, 1100,
        250, "#243647"), /fit inside glyph cell|clips glyph cell/);
});

test("membership and nonmembership retain separate sampled ink", () => {
    const shared = [1550, 2450, 1100, 250, "#243647"];
    const member = geometricStarter("∈", ...shared);
    const nonmember = geometricStarter("∉", ...shared);
    assert.equal(member.length, 33);
    assert.equal(nonmember.length, 34);
    assert.deepEqual([member[0].x1, member[0].y1], [2265, 1735]);
    assert.deepEqual([member[31].x2, member[31].y2],
        [2265, 3165]);
    assert.deepEqual([member[32].x1, member[32].y1,
        member[32].x2, member[32].y2], [835, 2450, 2155, 2450]);
    assert.deepEqual([nonmember[33].x1, nonmember[33].y1,
        nonmember[33].x2, nonmember[33].y2],
        [615, 3385, 2485, 1515]);
    assert.deepEqual(candidatePenPaths(member, "∈").map(
        ({ first, last }) => [first, last]), [[1, 32], [33, 33]]);
    assert.deepEqual(candidatePenPaths(nonmember, "∉").map(
        ({ first, last }) => [first, last]),
        [[1, 32], [33, 33], [34, 34]]);
    const coordinates = (rows) => rows.map(
        ({ grapheme, ...stroke }) => stroke);
    assert.deepEqual(coordinates(nonmember.slice(0, 33)),
        coordinates(member), "nonmembership only adds its own slash");
    assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(member)), member);
    assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(nonmember)),
        nonmember);
    assert.throws(() => geometricStarter("∉", 120, 2450, 1100,
        250, "#243647"), /fit inside glyph cell|clips glyph cell/);
});

test("contains and not-contains use exact reverse membership paths", () => {
    const shared = [1550, 2450, 1100, 250, "#243647"];
    for (const [source, target, size, paths] of [
        ["∈", "∋", 33, [[1, 32], [33, 33]]],
        ["∉", "∌", 34, [[1, 32], [33, 33], [34, 34]]],
    ]) {
        const member = geometricStarter(source, ...shared);
        const contains = geometricStarter(target, ...shared);
        assert.equal(contains.length, size);
        assert.deepEqual(candidatePenPaths(contains, target).map(
            ({ first, last }) => [first, last]), paths);
        for (let i = 0; i < 32; i += 1) {
            const original = member[i];
            const reversed = contains[i];
            assert.deepEqual([reversed.y1, reversed.y2],
                [original.y1, original.y2]);
            assert.ok(Math.abs(reversed.x1 -
                (3100 - original.x1)) <= 1);
            assert.ok(Math.abs(reversed.x2 -
                (3100 - original.x2)) <= 1,
            "integer quadratic sampling may differ by one rounded unit");
        }
        assert.deepEqual([contains[32].x1, contains[32].y1,
            contains[32].x2, contains[32].y2],
        [945, 2450, 2265, 2450],
        "reverse membership keeps its own separately lifted crossbar");
        if (size === 34) assert.deepEqual([contains[33].x1,
            contains[33].y1, contains[33].x2, contains[33].y2],
        [615, 3385, 2485, 1515]);
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(contains)),
            contains);
        assert.throws(() => geometricStarter(target, 150, 2450, 1100,
            250, "#243647"), /fit inside glyph cell|clips glyph cell/);
    }
});

test("subset signs keep independent curved contour and equality bar", () => {
    const shared = [1550, 2450, 1100, 250, "#243647"];
    const proper = geometricStarter("⊂", ...shared);
    const inclusive = geometricStarter("⊆", ...shared);
    assert.equal(proper.length, 64, "four directed quadratic arcs");
    assert.equal(inclusive.length, 65, "equality is a lifted stroke");
    assert.deepEqual([proper[0].x1, proper[0].y1], [2265, 1790]);
    assert.deepEqual([proper.at(-1).x2, proper.at(-1).y2],
        [2265, 3110]);
    assert.deepEqual([inclusive[64].x1, inclusive[64].y1,
        inclusive[64].x2, inclusive[64].y2],
        [780, 3440, 2320, 3440]);
    assert.deepEqual(candidatePenPaths(proper, "⊂").map(
        ({ first, last }) => [first, last]), [[1, 64]]);
    assert.deepEqual(candidatePenPaths(inclusive, "⊆").map(
        ({ first, last }) => [first, last]), [[1, 64], [65, 65]]);
    const coordinates = (rows) => rows.map(
        ({ grapheme, ...stroke }) => stroke);
    assert.deepEqual(coordinates(proper),
        coordinates(inclusive.slice(0, 64)),
        "inclusive subset retains the same exact proper-subset path");
    assert.notDeepEqual(proper, geometricStarter("∈", ...shared),
        "subset is not the membership bow");
    assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(inclusive)),
        inclusive);
    assert.throws(() => geometricStarter("⊆", 120, 2450, 1100,
        250, "#243647"), /fit inside glyph cell|clips glyph cell/);
});

test("superset signs mirror subset curves but keep distinct ink keys", () => {
    const shared = [1550, 2450, 1100, 250, "#243647"];
    for (const [subset, superset, count, paths] of [
        ["⊂", "⊃", 64, [[1, 64]]],
        ["⊆", "⊇", 65, [[1, 64], [65, 65]]],
    ]) {
        const source = geometricStarter(subset, ...shared);
        const reverse = geometricStarter(superset, ...shared);
        assert.equal(reverse.length, count);
        assert.deepEqual(candidatePenPaths(reverse, superset).map(
            ({ first, last }) => [first, last]), paths);
        assert.deepEqual([reverse[0].x1, reverse[0].y1], [835, 1790]);
        assert.deepEqual([reverse[63].x2, reverse[63].y2],
            [835, 3110]);
        for (let index = 0; index < 64; index += 1) {
            const original = source[index];
            const reflected = reverse[index];
            assert.deepEqual([reflected.x1, reflected.y1,
                reflected.x2, reflected.y2],
            [3100 - original.x1, original.y1,
                3100 - original.x2, original.y2],
            "reflect only the curve geometry around its authored center");
        }
        if (count === 65) assert.deepEqual([
            reverse[64].x1, reverse[64].y1,
            reverse[64].x2, reverse[64].y2,
        ], [780, 3440, 2320, 3440],
        "inclusive superset has a separately lifted reading-order bar");
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(reverse)),
            reverse);
    }
    assert.throws(() => geometricStarter("⊇", 3200, 2450, 1100,
        250, "#243647"), /fit inside glyph cell|clips glyph cell/);
});

test("negated set relations add exact lifted strokes, never aliases", () => {
    const shared = [1550, 2450, 1100, 250, "#243647"];
    for (const [plain, negated, total, penPaths] of [
        ["⊂", "⊄", 65, [[1, 64], [65, 65]]],
        ["⊆", "⊈", 66, [[1, 64], [65, 65], [66, 66]]],
        ["⊃", "⊅", 65, [[1, 64], [65, 65]]],
        ["⊇", "⊉", 66, [[1, 64], [65, 65], [66, 66]]],
    ]) {
        const original = geometricStarter(plain, ...shared);
        const refused = geometricStarter(negated, ...shared);
        assert.equal(refused.length, total);
        assert.deepEqual(candidatePenPaths(refused, negated).map(
            ({ first, last }) => [first, last]), penPaths);
        const coordinates = (rows) => rows.map(
            ({ grapheme, ...stroke }) => stroke);
        assert.deepEqual(coordinates(refused.slice(0, -1)),
            coordinates(original),
            "negation does not silently mutate the underlying contour");
        const strike = refused.at(-1);
        assert.deepEqual([strike.x1, strike.y1, strike.x2, strike.y2],
            [615, 3385, 2485, 1515]);
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(refused)),
            refused);
        assert.throws(() => geometricStarter(negated, 200, 2450, 1100,
            250, "#243647"), /fit inside glyph cell|clips glyph cell/);
    }
});

test("empty set, nabla and identity operators have independent pen ink", () => {
    const shared = [1550, 2450, 1100, 250, "#243647"];
    const options = [
        ["∅", 65, [[1, 64], [65, 65]]],
        ["∇", 3, [[1, 3]]],
        ["≡", 3, [[1, 1], [2, 2], [3, 3]]],
        ["≢", 4, [[1, 1], [2, 2], [3, 3], [4, 4]]],
    ];
    for (const [glyph, count, paths] of options) {
        const ink = geometricStarter(glyph, ...shared);
        assert.equal(ink.length, count);
        assert.deepEqual(candidatePenPaths(ink, glyph).map(
            ({ first, last }) => [first, last]), paths);
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(ink)), ink);
        assert.throws(() => geometricStarter(glyph, 100, 2450, 1100,
            250, "#243647"), /fit inside glyph cell|clips glyph cell/);
    }
    const empty = geometricStarter("∅", ...shared);
    assert.deepEqual([empty[0].x1, empty[0].y1], [1550, 1570]);
    assert.deepEqual([empty[63].x2, empty[63].y2], [1550, 1570],
        "the empty-set ellipse is a closed sampled path");
    assert.deepEqual([empty[64].x1, empty[64].y1,
        empty[64].x2, empty[64].y2], [670, 3440, 2430, 1460],
        "the diagonal slash is lifted from the empty-set contour");
    const gradient = geometricStarter("∇", ...shared);
    assert.deepEqual(gradient.map(({ x1, y1, x2, y2 }) =>
        [x1, y1, x2, y2]), [
        [670, 1460, 2430, 1460], [2430, 1460, 1550, 3440],
        [1550, 3440, 670, 1460],
    ], "nabla is an inverted continuous triangle, not delta");
    const identity = geometricStarter("≡", ...shared);
    const distinct = geometricStarter("≢", ...shared);
    assert.deepEqual(identity.map(({ x1, y1, x2, y2 }) =>
        [x1, y1, x2, y2]), [
        [780, 1735, 2320, 1735], [780, 2450, 2320, 2450],
        [780, 3165, 2320, 3165],
    ]);
    assert.deepEqual(distinct.slice(0, 3).map(({ grapheme, ...line }) =>
        line), identity.map(({ grapheme, ...line }) => line));
    assert.deepEqual([distinct[3].x1, distinct[3].y1,
        distinct[3].x2, distinct[3].y2], [725, 3330, 2375, 1570]);
    assert.notDeepEqual(gradient, geometricStarter("Δ", ...shared));
});

test("school geometry symbols preserve directed and lifted pen paths", () => {
    const shared = [1550, 2450, 1100, 250, "#243647"];
    const parallel = geometricStarter("∥", ...shared);
    const perpendicular = geometricStarter("⊥", ...shared);
    const similar = geometricStarter("∼", ...shared);
    const congruent = geometricStarter("≅", ...shared);
    assert.deepEqual(parallel.map(({ x1, y1, x2, y2 }) =>
        [x1, y1, x2, y2]), [
        [1154, 1515, 1154, 3385], [1946, 1515, 1946, 3385],
    ]);
    assert.deepEqual(perpendicular.map(({ x1, y1, x2, y2 }) =>
        [x1, y1, x2, y2]), [
        [1550, 1570, 1550, 3220], [780, 3220, 2320, 3220],
    ]);
    assert.equal(similar.length, 32);
    assert.equal(congruent.length, 34);
    assert.deepEqual([similar[0].x1, similar[0].y1,
        similar.at(-1).x2, similar.at(-1).y2],
    [670, 2670, 2430, 2230]);
    assert.deepEqual(congruent.slice(-2).map(({ x1, y1, x2, y2 }) =>
        [x1, y1, x2, y2]), [
        [780, 2560, 2320, 2560], [780, 3330, 2320, 3330],
    ]);
    for (const [glyph, ink, paths] of [
        ["∥", parallel, [[1, 1], [2, 2]]],
        ["⊥", perpendicular, [[1, 1], [2, 2]]],
        ["∼", similar, [[1, 32]]],
        ["≅", congruent, [[1, 32], [33, 33], [34, 34]]],
    ]) {
        assert.deepEqual(candidatePenPaths(ink, glyph).map(
            ({ first, last }) => [first, last]), paths);
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(ink)), ink);
        assert.throws(() => geometricStarter(glyph, 120, 2450, 1100,
            250, "#243647"), /fit inside glyph cell|clips glyph cell/);
    }
    assert.notDeepEqual(similar, geometricStarter("≈", ...shared));
    assert.notDeepEqual(congruent, geometricStarter("=", ...shared));
});

test("quantifier and therefore symbols retain exact separated ink", () => {
    const shared = [1550, 2450, 1100, 250, "#243647"];
    const cases = [
        ["∀", 3, [[1, 2], [3, 3]]],
        ["∃", 4, [[1, 1], [2, 2], [3, 3], [4, 4]]],
        ["∄", 5, [[1, 1], [2, 2], [3, 3], [4, 4], [5, 5]]],
        ["∴", 3, [[1, 1], [2, 2], [3, 3]]],
    ];
    for (const [glyph, size, paths] of cases) {
        const ink = geometricStarter(glyph, ...shared);
        assert.equal(ink.length, size);
        assert.deepEqual(candidatePenPaths(ink, glyph).map(
            ({ first, last }) => [first, last]), paths);
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(ink)), ink);
        assert.throws(() => geometricStarter(glyph, 100, 2450, 1100,
            250, "#243647"), /fit inside glyph cell|clips glyph cell/);
    }
    const universal = geometricStarter("∀", ...shared);
    assert.deepEqual(universal.map(({ x1, y1, x2, y2 }) =>
        [x1, y1, x2, y2]), [
        [780, 1570, 1550, 3385], [1550, 3385, 2320, 1570],
        [1088, 2615, 2012, 2615],
    ]);
    const exists = geometricStarter("∃", ...shared);
    assert.deepEqual(exists.map(({ x1, y1, x2, y2 }) =>
        [x1, y1, x2, y2]), [
        [2320, 1625, 2320, 3275], [890, 1625, 2320, 1625],
        [890, 2450, 2320, 2450], [890, 3275, 2320, 3275],
    ]);
    const notExists = geometricStarter("∄", ...shared);
    assert.deepEqual(notExists.slice(0, 4).map(
        ({ grapheme, ...line }) => line), exists.map(
        ({ grapheme, ...line }) => line));
    assert.deepEqual([notExists[4].x1, notExists[4].y1,
        notExists[4].x2, notExists[4].y2], [725, 3385, 2375, 1515]);
    const therefore = geometricStarter("∴", ...shared);
    assert.deepEqual(therefore.map(({ x1, y1, x2, y2 }) =>
        [x1, y1, x2, y2]), [
        [1550, 1658, 1550, 1812], [945, 2978, 945, 3132],
        [2155, 2978, 2155, 3132],
    ]);
    assert.notDeepEqual(universal, geometricStarter("Δ", ...shared));
});

test("because and logic turnstiles are exact independently lifted ink", () => {
    const shared = [1550, 2450, 1100, 250, "#243647"];
    const examples = [
        ["∵", [[945, 1768, 945, 1922],
            [2155, 1768, 2155, 1922], [1550, 3088, 1550, 3242]]],
        ["⊤", [[780, 1680, 2320, 1680],
            [1550, 1680, 1550, 3220]]],
        ["⊢", [[835, 1625, 835, 3275],
            [835, 2450, 2375, 2450]]],
        ["⊨", [[648, 1625, 648, 3275],
            [945, 1625, 945, 3275], [648, 2450, 2375, 2450]]],
    ];
    for (const [glyph, expected] of examples) {
        const ink = geometricStarter(glyph, ...shared);
        assert.deepEqual(ink.map(({ x1, y1, x2, y2 }) =>
            [x1, y1, x2, y2]), expected);
        assert.deepEqual(candidatePenPaths(ink, glyph).map(
            ({ first, last }) => [first, last]), expected.map((_, i) =>
                [i + 1, i + 1]));
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(ink)), ink);
        assert.throws(() => geometricStarter(glyph, 100, 2450, 1100,
            250, "#243647"), /fit inside glyph cell|clips glyph cell/);
    }
    assert.notDeepEqual(geometricStarter("∵", ...shared),
        geometricStarter("∴", ...shared));
    assert.notDeepEqual(geometricStarter("⊢", ...shared),
        geometricStarter("⊨", ...shared));
});

test("angles and negated logic entailment have exact independent ink", () => {
    const shared = [1550, 2450, 1100, 250, "#243647"];
    const cases = [
        ["∠", [[2408, 3220, 802, 3220],
            [802, 3220, 2155, 1680]], [[1, 2]]],
        ["∟", [[802, 1592, 802, 3220],
            [802, 3220, 2375, 3220]], [[1, 2]]],
        ["⊬", [[835, 1625, 835, 3275],
            [835, 2450, 2375, 2450],
            [626, 3352, 2496, 1548]], [[1, 1], [2, 2], [3, 3]]],
        ["⊭", [[648, 1625, 648, 3275],
            [945, 1625, 945, 3275], [648, 2450, 2375, 2450],
            [626, 3352, 2496, 1548]],
        [[1, 1], [2, 2], [3, 3], [4, 4]]],
    ];
    for (const [glyph, expected, paths] of cases) {
        const ink = geometricStarter(glyph, ...shared);
        assert.deepEqual(ink.map(({ x1, y1, x2, y2 }) =>
            [x1, y1, x2, y2]), expected);
        assert.deepEqual(candidatePenPaths(ink, glyph).map(
            ({ first, last }) => [first, last]), paths);
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(ink)), ink);
        assert.throws(() => geometricStarter(glyph, 100, 2450, 1100,
            250, "#243647"), /fit inside glyph cell|clips glyph cell/);
    }
    for (const [negative, positive] of [["⊬", "⊢"], ["⊭", "⊨"]]) {
        const no = geometricStarter(negative, ...shared);
        const yes = geometricStarter(positive, ...shared);
        assert.deepEqual(no.slice(0, yes.length).map(
            ({ grapheme, ...line }) => line), yes.map(
                ({ grapheme, ...line }) => line),
        "negation keeps exact positive geometry and adds a lifted slash");
        assert.equal(no.length, yes.length + 1);
    }
    assert.notDeepEqual(geometricStarter("∠", ...shared),
        geometricStarter("∟", ...shared));
});

test("negation and implication keep directed double-shaft pen paths", () => {
    const shared = [1550, 2450, 1100, 250, "#243647"];
    const cases = [
        ["¬", [[725, 2065, 2320, 2065],
            [2320, 2065, 2320, 3055]], [[1, 2]]],
        ["⇒", [[670, 2274, 2045, 2274],
            [670, 2626, 2045, 2626],
            [1770, 1845, 2430, 2450],
            [2430, 2450, 1770, 3055]],
        [[1, 1], [2, 2], [3, 4]]],
        ["⇔", [[1110, 2274, 1990, 2274],
            [1110, 2626, 1990, 2626],
            [1330, 1845, 670, 2450],
            [670, 2450, 1330, 3055],
            [1770, 1845, 2430, 2450],
            [2430, 2450, 1770, 3055]],
        [[1, 1], [2, 2], [3, 4], [5, 6]]],
    ];
    for (const [glyph, expected, paths] of cases) {
        const ink = geometricStarter(glyph, ...shared);
        assert.deepEqual(ink.map(({ x1, y1, x2, y2 }) =>
            [x1, y1, x2, y2]), expected);
        assert.deepEqual(candidatePenPaths(ink, glyph).map(
            ({ first, last }) => [first, last]), paths);
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(ink)), ink);
        assert.throws(() => geometricStarter(glyph, 150, 2450, 1100,
            250, "#243647"), /fit inside glyph cell|clips glyph cell/);
    }
    assert.notDeepEqual(geometricStarter("⇒", ...shared),
        geometricStarter("→", ...shared));
    assert.notDeepEqual(geometricStarter("⇔", ...shared),
        geometricStarter("↔", ...shared));
});

test("calculus and comparison math SVG preserves ideal curve controls",
    async () => {
        const ui = mountEditor();
        try {
            const { editor, get, downloads } = ui;
            editor.start();
            for (const [glyph, quadratics, lifts] of [
                ["∑", 0, 1], ["∫", 3, 1],
                ["∂", 4, 1], ["Δ", 0, 1], ["≈", 4, 2],
                ["→", 0, 2], ["∝", 4, 1],
                ["<", 0, 1], [">", 0, 1],
                ["←", 0, 2], ["↔", 0, 3],
                ["↑", 0, 2], ["↓", 0, 2], ["↕", 0, 3],
                ["∧", 0, 1], ["∨", 0, 1],
                ["∩", 2, 1], ["∪", 2, 1],
                ["∈", 2, 2], ["∉", 2, 3],
                ["∋", 2, 2], ["∌", 2, 3],
                ["⊂", 4, 1], ["⊆", 4, 2],
                ["⊃", 4, 1], ["⊇", 4, 2],
                ["⊄", 4, 2], ["⊈", 4, 3],
                ["⊅", 4, 2], ["⊉", 4, 3],
                ["∅", 4, 2], ["∇", 0, 1],
                ["≡", 0, 3], ["≢", 0, 4],
                ["∥", 0, 2], ["⊥", 0, 2],
                ["∼", 2, 1], ["≅", 2, 3],
                ["∀", 0, 2], ["∃", 0, 4],
                ["∄", 0, 5], ["∴", 0, 3],
                ["∵", 0, 3], ["⊤", 0, 2],
                ["⊢", 0, 2], ["⊨", 0, 3],
                ["∠", 0, 1], ["∟", 0, 1],
                ["⊬", 0, 3], ["⊭", 0, 4],
                ["¬", 0, 1], ["⇒", 0, 3], ["⇔", 0, 4],
                ["≪", 0, 2], ["≫", 0, 2],
                ["≮", 0, 2], ["≯", 0, 2],
                ["≰", 0, 3], ["≱", 0, 3],
                ["↗", 0, 2], ["↘", 0, 2],
                ["↖", 0, 2], ["↙", 0, 2], ["⇐", 0, 3],
                ["⟨", 0, 1], ["⟩", 0, 1],
            ]) {
                get("form").value = glyph;
                get("character").value = glyph;
                get("form").dispatch("input");
                const guide = get("ideal-rounded");
                const path = guide.attributes.get("d");
                assert.equal((path.match(/ Q /g) ?? []).length, quadratics);
                assert.equal((path.match(/M /g) ?? []).length, lifts);
                assert.equal(guide.attributes.get("visibility"), "visible");
                get("construct").dispatch("click");
                get("save").dispatch("click");
                const actual = parseGlyphLibrary(await downloads.at(-1).text());
                assert.deepEqual(actual.filter((line) => line.grapheme
                    === glyph), geometricStarter(glyph, 1550, 2450,
                    1100, 250, "#243647"));
            }
            const accepted = parseGlyphLibrary(await downloads.at(-1).text());
            get("form").value = "∫";
            get("character").value = "∫";
            get("form").dispatch("input");
            get("circle-x").value = "150";
            get("circle-x").dispatch("input");
            assert.equal(get("ideal-rounded").attributes.get("visibility"),
                "hidden", "clipped integral must hide ideal guide");
            get("construct").dispatch("click");
            assert.match(get("status").textContent, /fit inside glyph cell/);
            get("save").dispatch("click");
            assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
                accepted, "invalid calculus geometry never mutates ink");
        } finally {
            ui.restore();
        }
    });

test("degree and percent preserve ideal circular guides and pen lifts",
    async () => {
        const degree = geometricStarter("°", 1550, 2450, 1100,
            250, "#243647");
        const percent = geometricStarter("%", 1550, 2450, 1100,
            250, "#243647");
        assert.equal(degree.length, 48);
        assert.equal(percent.length, 97);
        assert.deepEqual(candidatePenPaths(degree, "°")
            .map(({ first, last }) => [first, last]), [[1, 48]]);
        assert.deepEqual(candidatePenPaths(percent, "%")
            .map(({ first, last }) => [first, last]),
        [[1, 48], [49, 96], [97, 97]]);
        for (const glyph of [degree, percent]) {
            const paths = candidatePenPaths(glyph, glyph[0].grapheme);
            for (const path of paths.filter(
                (entry) => entry.last > entry.first)) {
                assert.deepEqual(path.start, path.end,
                    "each ideal circular ring is a closed pen path");
            }
            assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(glyph)),
                glyph, "native integer TSV retains the exact grapheme");
        }
        const ui = mountEditor();
        try {
            const { editor, get, downloads } = ui;
            editor.start();
            for (const [glyph, arcs, counts] of [
                ["°", 2, 48], ["%", 4, 97],
            ]) {
                get("form").value = glyph;
                get("character").value = glyph;
                get("form").dispatch("input");
                const guide = get("ideal-rounded");
                assert.equal((guide.attributes.get("d").match(/ A /g)
                    ?? []).length, arcs, "SVG uses real circular arcs");
                assert.equal(guide.attributes.get("visibility"), "visible");
                for (const stem of ["ideal-rounded-left",
                    "ideal-rounded-right"]) {
                    assert.equal(get(stem).attributes.get("visibility"),
                        "hidden", "circle marks need no unrelated stems");
                }
                get("construct").dispatch("click");
                get("save").dispatch("click");
                const authored = parseGlyphLibrary(
                    await downloads.at(-1).text());
                assert.equal(authored.filter((s) => s.grapheme === glyph)
                    .length, counts);
            }
            const original = await downloads.at(-1).text();
            get("circle-x").value = "150";
            get("circle-x").dispatch("input");
            assert.equal(get("ideal-rounded").attributes.get("visibility"),
                "hidden", "clipped circles hide their ideal preview");
            get("construct").dispatch("click");
            assert.match(get("status").textContent,
                /fit inside glyph cell|clips glyph cell/);
            get("save").dispatch("click");
            assert.equal(await downloads.at(-1).text(), original,
                "invalid edited rings preserve existing authored strokes");
        } finally {
            ui.restore();
        }
        assert.throws(() => geometricStarter("%", 1550, 2450,
            200, 250, "#243647"), /pen-clearance/);
    });

test("peso/dollar and euro have distinct directed currency geometry",
    async () => {
        const args = [1550, 2450, 1100, 250, "#243647"];
        const dollar = geometricStarter("$", ...args);
        const euro = geometricStarter("€", ...args);
        const simpleS = geometricStarter("s", ...args);
        const openC = geometricStarter("c", ...args);
        assert.equal(dollar.length, 65);
        assert.equal(euro.length, 38);
        assert.deepEqual(dollar.slice(0, 64), simpleS.map((line) =>
            ({ ...line, grapheme: "$" })),
        "dollar upper/lower S maintains the four authored quadratics");
        assert.deepEqual(euro.slice(0, 36), openC.map((line) =>
            ({ ...line, grapheme: "€" })),
        "euro reuses the exact open-circle C ink path");
        assert.deepEqual(candidatePenPaths(dollar, "$").map(
            ({ first, last }) => [first, last]), [[1, 64], [65, 65]]);
        assert.deepEqual(candidatePenPaths(euro, "€").map(
            ({ first, last }) => [first, last]),
        [[1, 36], [37, 37], [38, 38]]);
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary([
            ...dollar, ...euro,
        ])), [...dollar, ...euro]);

        const ui = mountEditor();
        try {
            const { editor, get, downloads } = ui;
            editor.start();
            for (const [glyph, curves, count] of [
                ["$", 4, 65], ["€", 0, 38],
            ]) {
                get("form").value = glyph;
                get("character").value = glyph;
                get("form").dispatch("input");
                const guide = get("ideal-rounded");
                assert.equal(guide.attributes.get("visibility"), "visible");
                assert.equal((guide.attributes.get("d").match(/ Q /g)
                    ?? []).length, curves);
                assert.equal(get("ideal-c-loop").attributes.get("visibility"),
                    glyph === "€" ? "visible" : "hidden");
                if (glyph === "€") {
                    assert.match(get("ideal-c-loop").attributes.get("d"),
                        / A /, "euro guide retains the true circular arc");
                    assert.equal((guide.attributes.get("d").match(/ L /g)
                        ?? []).length, 2, "two separate euro crossbars");
                }
                get("construct").dispatch("click");
                get("save").dispatch("click");
                const saved = parseGlyphLibrary(
                    await downloads.at(-1).text());
                assert.equal(saved.filter((line) => line.grapheme === glyph)
                    .length, count);
            }
            const accepted = await downloads.at(-1).text();
            get("circle-x").value = "150";
            get("circle-x").dispatch("input");
            assert.equal(get("ideal-rounded").attributes.get("visibility"),
                "hidden", "clipped currency hides its ideal geometry");
            get("construct").dispatch("click");
            assert.match(get("status").textContent,
                /fit inside glyph cell|clips glyph cell/);
            get("save").dispatch("click");
            assert.equal(await downloads.at(-1).text(), accepted,
                "refused currency geometry never erases saved draft ink");
        } finally {
            ui.restore();
        }
    });

test("math fill keeps custom ink and refuses over-cap", async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        assert.equal(get("seed-math").disabled, true);
        editor.start();
        const originals = [stroke("−"), stroke("∞")];
        const bytes = new TextEncoder().encode(encodeGlyphLibrary(originals));
        get("load").files = [{
            size: bytes.length, arrayBuffer: async () => bytes.buffer,
        }];
        await get("load").dispatch("change");
        get("seed-math").dispatch("click");
        assert.match(get("status").textContent, /Filled 91 missing math/);
        assert.equal(downloads.length, 0);
        get("save").dispatch("click");
        const saved = parseGlyphLibrary(await downloads.at(-1).text());
        const missing = ["+", "-", "±", "∓", "=", "×", "÷", "√", "π",
            "≠", "<", ">", "≤", "≥", "∧", "∨", "∩", "∪", "∈", "∉",
            "∋", "∌",
            "⊂", "⊆", "⊃", "⊇", "⊄", "⊈", "⊅", "⊉",
            "∑", "∫", "∂",
            "Δ", "≈",
            "→", "←", "↔", "↑", "↓", "↕", "∝",
            "∅", "∇", "≡", "≢", "∥", "⊥", "∼", "≅",
            "∀", "∃", "∄", "∴", "∵", "⊤", "⊢", "⊨",
            "∠", "∟", "⊬", "⊭", "¬", "⇒", "⇔",
            "≪", "≫", "≮", "≯", "≰", "≱", "↗", "↘",
            "↖", "↙", "⇐", "↦", "↤", "∣", "∤",
            "′", "″", "‴", "⋅", "∗", "∖", "∦",
            "⌊", "⌋", "⌈", "⌉"];
        assert.deepEqual(saved, [...originals, ...missing.flatMap(
            (glyph) => geometricStarter(glyph, 1550, 2450,
                1100, 250, "#243647"))]);
        assert.ok(!saved.some(({ grapheme }) =>
            grapheme === "⟨" || grapheme === "⟩"),
        "inner-product angle brackets require explicit manual selection");
        get("seed-math").dispatch("click");
        assert.match(get("status").textContent, /All 93 math/);
        get("save").dispatch("click");
        assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
            saved, "custom Unicode minus remains untouched");
        const full = Array.from({ length: 4090 }, () => stroke("∞"));
        const big = new TextEncoder().encode(encodeGlyphLibrary(full));
        get("load").files = [{
            size: big.length, arrayBuffer: async () => big.buffer,
        }];
        await get("load").dispatch("change");
        get("seed-math").dispatch("click");
        assert.match(get("status").textContent, /Save requires 1..4096/);
        get("save").dispatch("click");
        assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
            full, "invalid math batch is atomic");
        editor.reset();
        assert.equal(get("seed-math").disabled, true);
    } finally {
        ui.restore();
    }
});

test("math SVG respects exact paths and clears invalid preview", async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        editor.start();
        get("form").value = "×";
        get("character").value = "×";
        get("form").dispatch("input");
        assert.equal(get("ideal-rounded").attributes.get("d"),
            "M 890 1790 L 2210 3110 M 2210 1790 L 890 3110");
        assert.equal(get("ideal-rounded-left").attributes.get("visibility"),
            "hidden");
        get("construct").dispatch("click");
        get("save").dispatch("click");
        const saved = parseGlyphLibrary(await downloads.at(-1).text());
        assert.deepEqual(saved, geometricStarter("×", 1550, 2450,
            1100, 250, "#243647"));
        get("circle-x").value = "150";
        get("circle-x").dispatch("input");
        assert.equal(get("ideal-rounded").attributes.get("visibility"),
            "hidden");
        get("construct").dispatch("click");
        assert.match(get("status").textContent, /fit inside glyph cell/);
        get("save").dispatch("click");
        assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
            saved, "invalid math geometry keeps authored ink");
    } finally {
        ui.restore();
    }
});

test("infinity SVG guide is quadratic and never mutates custom ink",
    async () => {
        const ui = mountEditor();
        try {
            const { editor, get, downloads } = ui;
            editor.start();
            get("form").value = "∞";
            get("character").value = "∞";
            get("form").dispatch("input");
            const guide = get("ideal-rounded");
            const path = guide.attributes.get("d");
            assert.match(path, /^M 1550 2450 Q 945 1515 670 2450/);
            assert.equal(path.match(/ Q /g).length, 4,
                "SVG retains four exact mathematical quadratics");
            assert.equal(guide.attributes.get("visibility"), "visible");
            get("construct").dispatch("click");
            get("save").dispatch("click");
            const original = parseGlyphLibrary(await downloads.at(-1).text());
            assert.deepEqual(original, geometricStarter("∞", 1550, 2450,
                1100, 250, "#243647"));
            get("circle-x").value = "150";
            get("circle-x").dispatch("input");
            assert.equal(guide.attributes.get("visibility"), "hidden");
            get("construct").dispatch("click");
            assert.match(get("status").textContent, /fit inside glyph cell/);
            get("save").dispatch("click");
            assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
                original, "invalid radical and infinity ink must not replace");
        } finally {
            ui.restore();
        }
    });

test("numeral 0-9 use distinct exact ellipse and ordered stroke guides", () => {
    const counts = [48, 3, 34, 64, 3, 34, 64, 2, 96, 64];
    const args = [1550, 2450, 1100, 250, "#243647"];
    const all = [..."0123456789"].map((g, index) => {
        const lines = geometricStarter(g, ...args);
        assert.equal(lines.length, counts[index]);
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(lines)), lines);
        return lines;
    });
    const zero = all[0];
    const circularO = geometricStarter("o", ...args);
    assert.equal(zero[0].x1, 2320, "ellipse has 770 µm X radius");
    assert.equal(zero[0].y1, 2450);
    assert.equal(zero[12].x1, 1550);
    assert.equal(zero[12].y1, 3550, "ellipse has 1100 µm Y radius");
    assert.equal(circularO[0].x1, 2650, "lowercase o is circular");
    assert.equal(zero[47].x2, zero[0].x1);
    assert.equal(zero[47].y2, zero[0].y1);
    const eight = all[8];
    for (const start of [0, 48]) {
        assert.equal(eight[start].x1, eight[start + 47].x2);
        assert.equal(eight[start].y1, eight[start + 47].y2);
    }
    assert.deepEqual(all[1].map((line) =>
        [line.x1, line.y1, line.x2, line.y2]), [
        [1110, 1790, 1550, 1350],
        [1550, 1350, 1550, 3550],
        [1022, 3550, 2078, 3550],
    ]);
    assert.deepEqual(all[7].map((line) =>
        [line.x1, line.y1, line.x2, line.y2]), [
        [780, 1350, 2320, 1350],
        [2320, 1350, 1275, 3550],
    ]);
    for (const glyph of ["2", "3", "5", "6", "9"]) {
        const lines = all[Number(glyph)];
        assert.ok(lines.some((line, index) => index > 0
            && line.x1 === lines[index - 1].x2
            && line.y1 === lines[index - 1].y2));
    }
    assert.throws(() => geometricStarter("0", 150, 2450,
        1100, 250, "#243647"), /fit inside glyph cell/);
    assert.throws(() => geometricStarter("8", 1550, 3650,
        1100, 250, "#243647"), /fit inside glyph cell/);
});

test("digit SVG guide preserves ellipses and native ink", async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        editor.start();
        get("form").value = "0";
        get("character").value = "0";
        get("form").dispatch("input");
        assert.equal(get("ideal-circle").attributes.get("visibility"),
            "hidden", "numeric zero has its own ellipse, not the o circle");
        assert.match(get("ideal-rounded").attributes.get("d"),
            /^M 2320 2450 A 770 1100 0 1 1 780 2450/);
        assert.equal(get("ideal-rounded-left").attributes.get("visibility"),
            "hidden");
        get("construct").dispatch("click");
        get("form").value = "8";
        get("character").value = "8";
        get("form").dispatch("input");
        const guide = get("ideal-rounded").attributes.get("d");
        assert.match(guide, /A 627 550 0 1 1/);
        assert.match(guide, /A 770 550 0 1 1/);
        assert.equal(guide.split("M ").length - 1, 2,
            "two disconnected loops remain editable as ordered pen strokes");
        get("construct").dispatch("click");
        get("save").dispatch("click");
        const saved = parseGlyphLibrary(await downloads.at(-1).text());
        assert.deepEqual(saved, [
            ...geometricStarter("0", 1550, 2450, 1100, 250, "#243647"),
            ...geometricStarter("8", 1550, 2450, 1100, 250, "#243647"),
        ]);
        get("circle-y").value = "4350";
        get("circle-y").dispatch("input");
        assert.equal(get("ideal-rounded").attributes.get("visibility"),
            "hidden");
        get("construct").dispatch("click");
        assert.match(get("status").textContent, /fit inside glyph cell/);
        get("save").dispatch("click");
        assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
            saved, "refused change does not replace an authored numeral");
    } finally {
        ui.restore();
    }
});

test("digit fill preserves custom ink and refuses over-cap", async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        assert.equal(get("seed-digits").disabled, true);
        editor.start();
        const originals = [stroke("0"), stroke("∞")];
        const bytes = new TextEncoder().encode(encodeGlyphLibrary(originals));
        get("load").files = [{ size: bytes.length,
            arrayBuffer: async () => bytes.buffer }];
        await get("load").dispatch("change");
        get("seed-digits").dispatch("click");
        assert.match(get("status").textContent, /Filled 9 missing numeral/);
        assert.equal(downloads.length, 0);
        get("save").dispatch("click");
        const saved = parseGlyphLibrary(await downloads.at(-1).text());
        assert.deepEqual(saved, [...originals, ...[..."123456789"]
            .flatMap((g) => geometricStarter(g, 1550, 2450,
                1100, 250, "#243647"))]);
        get("seed-digits").dispatch("click");
        assert.match(get("status").textContent, /All ten numeral glyphs/);
        const full = Array.from({ length: 4080 }, () => stroke("∞"));
        const big = new TextEncoder().encode(encodeGlyphLibrary(full));
        get("load").files = [{ size: big.length,
            arrayBuffer: async () => big.buffer }];
        await get("load").dispatch("change");
        get("seed-digits").dispatch("click");
        assert.match(get("status").textContent, /Save requires 1..4096/);
        get("save").dispatch("click");
        assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
            full, "over-cap batch does not change any previously drawn glyph");
        editor.reset();
        assert.equal(get("seed-digits").disabled, true);
    } finally {
        ui.restore();
    }
});

test("uppercase A-Z have distinct sampled geometry and exact circles", () => {
    const counts = [3, 65, 36, 33, 4, 3, 38, 3, 3, 34, 3, 2, 4,
        3, 48, 33, 49, 34, 64, 2, 34, 2, 4, 2, 3, 3];
    const args = [1550, 2450, 1100, 250, "#243647"];
    const alphabet = [..."ABCDEFGHIJKLMNOPQRSTUVWXYZ"].map((g, index) => {
        const lines = geometricStarter(g, ...args);
        assert.equal(lines.length, counts[index], g);
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(lines)), lines);
        return lines;
    });
    const upperO = alphabet[14];
    const lowerO = geometricStarter("o", ...args);
    assert.deepEqual(upperO.map(({ x1, y1, x2, y2 }) =>
        [x1, y1, x2, y2]), lowerO.map(({ x1, y1, x2, y2 }) =>
        [x1, y1, x2, y2]), "uppercase O shares exact o circle");
    assert.deepEqual(alphabet[16].slice(0, 48).map((line) =>
        [line.x1, line.y1, line.x2, line.y2]),
        upperO.map((line) => [line.x1, line.y1, line.x2, line.y2]),
        "Q retains the same unmodified circle before its pen-lift tail");
    assert.deepEqual(alphabet[2].map(({ x1, y1, x2, y2 }) =>
        [x1, y1, x2, y2]), geometricStarter("c", ...args).map(
        ({ x1, y1, x2, y2 }) => [x1, y1, x2, y2]));
    assert.deepEqual(alphabet[6].slice(0, 36).map((line) =>
        [line.x1, line.y1, line.x2, line.y2]),
        alphabet[2].map((line) =>
            [line.x1, line.y1, line.x2, line.y2]));
    assert.deepEqual(alphabet[18].map(({ x1, y1, x2, y2 }) =>
        [x1, y1, x2, y2]), geometricStarter("s", ...args).map(
        ({ x1, y1, x2, y2 }) => [x1, y1, x2, y2]));
    assert.deepEqual(alphabet[0].map((line) =>
        [line.x1, line.y1, line.x2, line.y2]), [
        [725, 3550, 1550, 1350], [1550, 1350, 2375, 3550],
        [1121, 2505, 1979, 2505],
    ]);
    assert.throws(() => geometricStarter("A", 1550, 1050, 1100,
        250, "#243647"), /fit inside glyph cell/);
});

test("uppercase SVG guides show circles, open arcs and curves", async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        editor.start();
        get("form").value = "O";
        get("character").value = "O";
        get("form").dispatch("input");
        assert.equal(get("ideal-circle").attributes.get("visibility"),
            "visible");
        assert.equal(get("ideal-rounded-left").attributes.get("visibility"),
            "hidden");
        get("construct").dispatch("click");
        get("form").value = "G";
        get("character").value = "G";
        get("form").dispatch("input");
        assert.equal(get("ideal-c-loop").attributes.get("visibility"),
            "visible");
        assert.match(get("ideal-rounded").attributes.get("d"),
            /M 1660 2835 L 2375 2835/);
        get("construct").dispatch("click");
        get("form").value = "B";
        get("character").value = "B";
        get("form").dispatch("input");
        assert.match(get("ideal-rounded").attributes.get("d"), /Q /);
        assert.equal(get("ideal-circle").attributes.get("visibility"),
            "hidden");
        get("construct").dispatch("click");
        get("save").dispatch("click");
        const saved = parseGlyphLibrary(await downloads.at(-1).text());
        assert.deepEqual(saved, ["O", "G", "B"].flatMap((g) =>
            geometricStarter(g, 1550, 2450, 1100, 250, "#243647")));
        get("circle-x").value = "200";
        get("circle-x").dispatch("input");
        assert.equal(get("ideal-rounded").attributes.get("visibility"),
            "hidden");
        get("construct").dispatch("click");
        assert.match(get("status").textContent, /fit inside glyph cell/);
        get("save").dispatch("click");
        assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
            saved, "clipped capital must not overwrite authored ink");
    } finally {
        ui.restore();
    }
});

test("uppercase fill preserves ink and refuses over-cap", async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        assert.equal(get("seed-capitals").disabled, true);
        editor.start();
        const originals = [stroke("Q"), stroke("∞")];
        const bytes = new TextEncoder().encode(encodeGlyphLibrary(originals));
        get("load").files = [{ size: bytes.length,
            arrayBuffer: async () => bytes.buffer }];
        await get("load").dispatch("change");
        get("seed-capitals").dispatch("click");
        assert.match(get("status").textContent, /Filled 25 missing uppercase/);
        assert.equal(downloads.length, 0);
        get("save").dispatch("click");
        const saved = parseGlyphLibrary(await downloads.at(-1).text());
        assert.deepEqual(saved, [...originals,
            ...[..."ABCDEFGHIJKLMNOPQRSTUVWXYZ"].filter((g) => g !== "Q")
                .flatMap((g) => geometricStarter(g, 1550, 2450,
                    1100, 250, "#243647"))]);
        get("seed-capitals").dispatch("click");
        assert.match(get("status").textContent, /All 26 uppercase glyphs/);
        const full = Array.from({ length: 4000 }, () => stroke("∞"));
        const big = new TextEncoder().encode(encodeGlyphLibrary(full));
        get("load").files = [{ size: big.length,
            arrayBuffer: async () => big.buffer }];
        await get("load").dispatch("change");
        get("seed-capitals").dispatch("click");
        assert.match(get("status").textContent, /Save requires 1..4096/);
        get("save").dispatch("click");
        assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
            full, "over-cap fill must be atomic");
        editor.reset();
        assert.equal(get("seed-capitals").disabled, true);
    } finally {
        ui.restore();
    }
});

test("14 direct NFD accents keep exact keys and canonical starter ink", () => {
    const args = [1550, 2450, 1100, 250, "#243647"];
    const composed = ["á", "é", "í", "ó", "ú", "ñ", "ü",
        "Á", "É", "Í", "Ó", "Ú", "Ñ", "Ü"];
    const all = [];
    for (const nfc of composed) {
        const nfd = nfc.normalize("NFD");
        assert.notEqual(nfd, nfc);
        const canonical = geometricStarter(nfc, ...args);
        const authored = geometricStarter(nfd, ...args);
        assert.deepEqual(authored, canonical.map((line) =>
            ({ ...line, grapheme: nfd })), `${nfd} geometry`);
        assert.ok(authored.every((line) => line.grapheme === nfd));
        all.push(...canonical, ...authored);
        assert.deepEqual(candidatePenPaths(authored, nfc), []);
        assert.notEqual(candidatePenPaths(authored, nfd).length, 0);
    }
    assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(all)), all);
    for (const [nfc, variant] of [["á", "two-storey"],
        ["á", "two-storey-italic"], ["ó", "italic"],
        ["ó", "cursive"]]) {
        const nfd = nfc.normalize("NFD");
        const chosen = variant.startsWith("two-storey")
            ? [1550, 2450, 700, 250, "#243647"] : args;
        assert.deepEqual(geometricStarter(nfd, ...chosen, undefined, variant),
            geometricStarter(nfc, ...chosen, undefined, variant)
                .map((line) => ({ ...line, grapheme: nfd })));
    }
    assert.throws(() => geometricStarter("o\u0308", ...args),
        /supported exact geometric/);
    assert.throws(() => geometricStarter("a\u0301", 200, 2450,
        1100, 250, "#243647"), /fit inside glyph cell/);
    assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(all)), all,
        "refused construction cannot mutate previously authored geometry");
});

test("NFD geometric choices preview canonical shape but save exact ink",
    async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        editor.start();
        for (const [nfd, guide] of [["o\u0301", "ideal-circle"],
            ["n\u0303", "ideal-diacritic"],
            ["N\u0303", "ideal-diacritic"]]) {
            get("form").value = nfd;
            get("character").value = nfd;
            get("form").dispatch("input");
            assert.equal(get(guide).attributes.get("visibility"),
                "visible", `NFD guide for ${nfd}`);
            assert.match(get("ideal-preview").attributes.get("aria-label"),
                /Analytic geometric/);
            get("construct").dispatch("click");
            assert.match(get("status").textContent, /Constructed .*segments/);
        }
        get("save").dispatch("click");
        const first = parseGlyphLibrary(await downloads.at(-1).text());
        assert.deepEqual(first, ["o\u0301", "n\u0303", "N\u0303"]
            .flatMap((glyph) => geometricStarter(glyph, 1550, 2450,
                1100, 250, "#243647")));
        get("form").value = "Ñ";
        get("character").value = "Ñ";
        get("form").dispatch("input");
        get("construct").dispatch("click");
        get("save").dispatch("click");
        const withNfc = parseGlyphLibrary(await downloads.at(-1).text());
        assert.deepEqual(withNfc.slice(0, first.length), first);
        assert.deepEqual(withNfc.slice(first.length),
            geometricStarter("Ñ", 1550, 2450,
                1100, 250, "#243647"));
        get("form").value = "N\u0303";
        get("character").value = "N\u0303";
        get("circle-x").value = "200";
        get("circle-x").dispatch("input");
        assert.equal(get("ideal-diacritic").attributes.get("visibility"),
            "hidden", "invalid guide does not show stale marks");
        get("construct").dispatch("click");
        assert.match(get("status").textContent, /fit inside glyph cell/);
        get("save").dispatch("click");
        assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
            withNfc, "rejected NFD change preserves both NFC and NFD ink");
    } finally {
        ui.restore();
    }
});

test("NFD acute a/o variants use exact menu form and editable TSV",
    async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        editor.start();
        const nfdA = "a\u0301";
        const nfdO = "o\u0301";
        const samples = [
            [nfdA + "-italic", nfdA, "italic", 1100],
            [nfdA + "-cursive", nfdA, "cursive", 1100],
            [nfdA + "-two", nfdA, "two-storey", 700],
            [nfdA + "-two-italic", nfdA, "two-storey-italic", 700],
            [nfdO + "-italic", nfdO, "italic", 1100],
            [nfdO + "-cursive", nfdO, "cursive", 1100],
        ];
        let accumulated = [];
        for (const [form, glyph, kind, radius] of samples) {
            get("form").value = form;
            get("character").value = glyph;
            get("circle-radius").value = String(radius);
            get("form").dispatch("input");
            assert.equal(get("ideal-arch").attributes.get("visibility"),
                kind.startsWith("two-storey") ? "visible" : "hidden");
            assert.equal(get("ideal-entry").attributes.get("visibility"),
                kind === "cursive" ? "visible" : "hidden");
            assert.equal(get("italic-angle").disabled,
                !kind.endsWith("italic"));
            get("construct").dispatch("click");
            assert.match(get("status").textContent,
                /Constructed \d+ sampled segments/);
            accumulated = [...accumulated.filter((s) => s.grapheme !== glyph),
                ...geometricStarter(glyph, 1550, 2450, radius,
                    250, "#243647", undefined, kind)];
            get("save").dispatch("click");
            assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
                accumulated, `${form} retains independent exact Unicode`);
        }
        const intact = accumulated;
        get("form").value = nfdA + "-two";
        get("character").value = nfdA;
        get("circle-radius").value = "1100";
        get("form").dispatch("input");
        assert.equal(get("ideal-arch").attributes.get("visibility"),
            "hidden", "clipped NFD two-storey guide cannot show old ink");
        get("construct").dispatch("click");
        assert.match(get("status").textContent, /fit inside glyph cell/);
        get("save").dispatch("click");
        assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
            intact, "clipped NFD variant leaves earlier drafts unchanged");
    } finally {
        ui.restore();
    }
});

test("uppercase NFC and NFD preserve authored ink", async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        assert.equal(get("seed-upper-spanish").disabled, true);
        assert.equal(get("seed-upper-spanish-nfd").disabled, true);
        editor.start();
        get("seed-upper-spanish").dispatch("click");
        assert.match(get("status").textContent,
            /Author uppercase base glyphs first/);
        get("seed-capitals").dispatch("click");
        get("save").dispatch("click");
        const original = parseGlyphLibrary(await downloads.at(-1).text());
        const custom = { ...stroke("Ñ"), color: "#ABCDEF" };
        const before = [...original, custom];
        const bytes = new TextEncoder().encode(encodeGlyphLibrary(before));
        get("load").files = [{ size: bytes.length,
            arrayBuffer: async () => bytes.buffer }];
        await get("load").dispatch("change");
        get("seed-upper-spanish").dispatch("click");
        assert.match(get("status").textContent,
            /Filled 6 missing Spanish uppercase NFC accents/);
        get("seed-upper-spanish-nfd").dispatch("click");
        assert.match(get("status").textContent,
            /Filled 7 missing Spanish uppercase NFD accents/);
        assert.equal(downloads.length, 1, "no implicit download");
        get("save").dispatch("click");
        const saved = parseGlyphLibrary(await downloads.at(-1).text());
        const pairs = [["A", "Á"], ["E", "É"], ["I", "Í"],
            ["O", "Ó"], ["U", "Ú"], ["N", "Ñ"], ["U", "Ü"]];
        const nfc = pairs.filter(([, target]) => target !== "Ñ")
            .flatMap(([base, target]) =>
                composeAccentedGlyph(before, base, target, "#243647"));
        const nfd = pairs.flatMap(([base, target]) =>
            composeAccentedGlyph(before, base, target.normalize("NFD"),
                "#243647"));
        assert.deepEqual(saved, [...before, ...nfc, ...nfd]);
        assert.deepEqual(saved.filter((line) => line.grapheme === "Ñ"),
            [custom], "existing composed target is preserved");
        assert.deepEqual(saved.slice(0, before.length), before,
            "every authored uppercase source remains byte-identical");
        get("seed-upper-spanish").dispatch("click");
        get("seed-upper-spanish-nfd").dispatch("click");
        assert.match(get("status").textContent,
            /All seven Spanish uppercase NFD accents/);
        get("save").dispatch("click");
        assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
            saved, "repeat action never replaces exact authored ink");
        editor.reset();
        assert.equal(get("seed-upper-spanish").disabled, true);
        assert.equal(get("seed-upper-spanish-nfd").disabled, true);
    } finally {
        ui.restore();
    }
});

test("uppercase accent batches refuse clipped marks and over-cap", async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        editor.start();
        get("seed-capitals").dispatch("click");
        get("accent-lift").value = "500";
        get("accent-lift").dispatch("input");
        get("seed-upper-spanish").dispatch("click");
        assert.match(get("status").textContent, /fit inside glyph cell/);
        get("save").dispatch("click");
        const before = parseGlyphLibrary(await downloads.at(-1).text());
        assert.equal(before.length, 512);
        get("accent-lift").value = "0";
    get("italic-angle").value = "12";
        const full = [...before, ...Array.from({ length: 3540 },
            () => stroke("∞"))];
        const bytes = new TextEncoder().encode(encodeGlyphLibrary(full));
        get("load").files = [{ size: bytes.length,
            arrayBuffer: async () => bytes.buffer }];
        await get("load").dispatch("change");
        get("seed-upper-spanish-nfd").dispatch("click");
        assert.match(get("status").textContent, /Save requires 1..4096/);
        get("save").dispatch("click");
        assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
            full, "refused batch leaves every authored source unchanged");
    } finally {
        ui.restore();
    }
});

test("eleven text punctuation shapes remain distinct and bounded", () => {
    const forms = [";", ":", "\"", "'", "…", "–", "—",
        "(", ")", "[", "]"];
    const counts = [2, 2, 2, 1, 3, 1, 1, 32, 32, 3, 3];
    const args = [1550, 2450, 1100, 250, "#243647"];
    for (const [index, glyph] of forms.entries()) {
        const lines = geometricStarter(glyph, ...args);
        assert.equal(lines.length, counts[index], glyph);
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(lines)), lines);
    }
    assert.deepEqual(geometricStarter("–", ...args)[0].grapheme, "–");
    assert.notDeepEqual(geometricStarter("–", ...args),
        geometricStarter("—", ...args));
    assert.notDeepEqual(geometricStarter("'", ...args),
        geometricStarter("’", ...args));
    assert.throws(() => geometricStarter(":", 1550, 2450, 200,
        250, "#243647"), /pen-clearance/);
    assert.throws(() => geometricStarter("(", 150, 2450, 1100,
        250, "#243647"), /fit inside glyph cell|clips glyph cell/);
});

test("manual notation strokes keep distinct slash and mirrored curly ink",
    () => {
    const args = [1550, 2450, 1100, 250, "#123456"];
    const forms = ["/", "\\", "{", "}"];
    const counts = [1, 1, 64, 64];
    const lines = forms.map((glyph) => geometricStarter(glyph, ...args));
    for (const [index, glyph] of forms.entries()) {
        const strokes = lines[index];
        assert.equal(strokes.length, counts[index], glyph);
        assert.ok(strokes.every((stroke) => stroke.grapheme === glyph));
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(strokes)),
            strokes, "exact ASCII punctuation survives native TSV");
        assert.equal(candidatePenPaths(strokes, glyph).length, 1,
            "each notation mark is one directed pen path");
    }
    assert.deepEqual(lines[0][0], {
        grapheme: "/", x1: 2265, y1: 1405, x2: 835, y2: 3495,
        width: 250, color: "#123456",
    });
    assert.deepEqual(lines[1][0], {
        grapheme: "\\", x1: 835, y1: 1405, x2: 2265, y2: 3495,
        width: 250, color: "#123456",
    });
    const left = lines[2];
    const right = lines[3];
    for (let index = 0; index < left.length; index += 1) {
        assert.ok(Math.abs(left[index].x1 + right[index].x1 - 3100) <= 1,
            "rounded native micrometres preserve the mirrored ideal");
        assert.ok(Math.abs(left[index].x2 + right[index].x2 - 3100) <= 1);
        assert.equal(left[index].y1, right[index].y1);
        assert.equal(left[index].y2, right[index].y2);
        if (index > 0) {
            assert.deepEqual([left[index].x1, left[index].y1],
                [left[index - 1].x2, left[index - 1].y2],
                "one continuous four-curve brace path");
        }
    }
    assert.throws(() => geometricStarter("{", 250, 2450, 1100,
        250, "#123456"), /Shape and pen/);
    assert.throws(() => geometricStarter("\\", 1550, 2450, 1100,
        1600, "#123456"), /outside native bounds/);
    assert.throws(() => geometricStarter("/", 1550, 2450, 1100,
        250, "#GGGGGG"), /Ink color/);
});

test("manual notation guide, save and clipped edit preserve authored draft",
    async () => {
        const ui = mountEditor();
        try {
            const { editor, get, downloads } = ui;
            editor.start();
            for (const glyph of ["/", "\\", "{", "}"]) {
                get("character").value = glyph;
                get("character").dispatch("input");
                get("form").value = glyph;
                get("form").dispatch("change");
                const guide = get("ideal-rounded");
                assert.equal(guide.attributes.get("visibility"), "visible");
                const path = guide.attributes.get("d");
                assert.match(path, /M [0-9]+ [0-9]+ /);
                if (glyph === "{" || glyph === "}") {
                    assert.equal((path.match(/Q /g) ?? []).length, 4,
                        "each brace retains four exact SVG quadratic guides");
                } else {
                    assert.match(path, / L /, "slash guide stays straight");
                }
                get("construct").dispatch("click");
                assert.match(get("status").textContent, /Constructed/);
            }
            get("save").dispatch("click");
            const saved = parseGlyphLibrary(await downloads.at(-1).text());
            assert.deepEqual(saved, ["/", "\\", "{", "}"].flatMap(
                (glyph) => geometricStarter(glyph, 1550, 2450, 1100,
                    250, "#243647")));
            get("circle-x").value = "150";
            get("circle-x").dispatch("input");
            assert.equal(get("ideal-rounded").attributes.get("visibility"),
                "hidden", "clipped notation has no misleading valid guide");
            get("construct").dispatch("click");
            assert.match(get("status").textContent, /Shape and pen/);
            get("save").dispatch("click");
            assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
                saved, "refusal must never change prior caller-authored ink");
        } finally {
            ui.restore();
        }
    });

test("text marks fill keeps custom ink and rejects over-cap", async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        assert.equal(get("seed-text-marks").disabled, true);
        editor.start();
        const initial = [stroke(";"), stroke("∞")];
        const bytes = new TextEncoder().encode(encodeGlyphLibrary(initial));
        get("load").files = [{ size: bytes.length,
            arrayBuffer: async () => bytes.buffer }];
        await get("load").dispatch("change");
        get("form").value = "(";
        get("character").value = "(";
        get("form").dispatch("input");
        assert.match(get("ideal-rounded").attributes.get("d"), /Q /);
        get("seed-text-marks").dispatch("click");
        assert.match(get("status").textContent,
            /Filled 10 missing text punctuation/);
        assert.equal(downloads.length, 0);
        get("save").dispatch("click");
        const saved = parseGlyphLibrary(await downloads.at(-1).text());
        const missing = [":", "\"", "'", "…", "–", "—",
            "(", ")", "[", "]"];
        assert.deepEqual(saved, [...initial, ...missing.flatMap((glyph) =>
            geometricStarter(glyph, 1550, 2450, 1100, 250, "#243647"))]);
        get("seed-text-marks").dispatch("click");
        assert.match(get("status").textContent,
            /All eleven text punctuation/);
        const full = Array.from({ length: 4050 }, () => stroke("∞"));
        const raw = new TextEncoder().encode(encodeGlyphLibrary(full));
        get("load").files = [{ size: raw.length,
            arrayBuffer: async () => raw.buffer }];
        await get("load").dispatch("change");
        get("seed-text-marks").dispatch("click");
        assert.match(get("status").textContent, /Save requires 1..4096/);
        get("save").dispatch("click");
        assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
            full);
        editor.reset();
        assert.equal(get("seed-text-marks").disabled, true);
    } finally {
        ui.restore();
    }
});

test("next-missing navigation cycles exact NFC/NFD and never draws ink",
    async () => {
        const ui = mountEditor();
        try {
            const { editor, get, downloads } = ui;
            assert.equal(get("focus-missing").disabled, true);
            editor.start();
            assert.equal(get("focus-missing").disabled, false);
            get("character").value = "ü";
            get("character").dispatch("input");
            get("focus-missing").dispatch("click");
            assert.equal(get("character").value, "a\u0301");
            assert.equal(get("form").value, "a\u0301",
                "selected NFD has its own exact geometry guide");
            assert.match(get("status").textContent,
                /Inspecting missing glyph á \(NFD\); draft unchanged/);
            get("focus-missing").dispatch("click");
            assert.equal(get("character").value, "e\u0301");
            get("character").value = "U\u0308";
            get("character").dispatch("input");
            get("focus-missing").dispatch("click");
            assert.equal(get("character").value, "a", "wrap to first key");
            assert.equal(downloads.length, 0);
            assert.match(get("baseline-coverage").textContent,
                /0\/114 exact glyphs; 114 missing/);
            get("seed-baseline").dispatch("click");
            assert.equal(get("focus-missing").disabled, true);
            editor.reset();
            assert.equal(get("focus-missing").disabled, true);
        } finally {
            ui.restore();
        }
    });

test("baseline coverage is read-only and tracks separate Unicode keys",
    () => {
        const source = [stroke("á"), stroke("n\u0303"), stroke("∞"),
            stroke("á")];
        const saved = encodeGlyphLibrary(source);
        const missing = missingBilingualBaseline(source);
        assert.equal(missing.length, 112);
        assert.equal(new Set(missing).size, 112);
        assert.equal(missing.includes("á"), false);
        assert.equal(missing.includes("a\u0301"), true);
        assert.equal(missing.includes("n\u0303"), false);
        assert.equal(missing.includes("ñ"), true);
        assert.ok(missing.includes("-"));
        assert.equal(encodeGlyphLibrary(source), saved);
        assert.equal(missingBilingualBaseline(fillMissingBilingualBaseline(
            source, 1550, 2450, 1100, 250, "#243647",
            { dx: 0, lift: 0 })).length, 0);
    });

test("one atomic fill authors only the 114 exact bilingual baseline", () => {
    const existing = [
        { ...stroke("ó"), color: "#102030" },
        { ...stroke("o\u0301"), color: "#405060" },
        stroke("∞"),
    ];
    const original = encodeGlyphLibrary(existing);
    const filled = fillMissingBilingualBaseline(existing,
        1550, 2450, 1100, 250, "#243647", { dx: 0, lift: 0 });
    assert.equal(new Set(filled.map((line) => line.grapheme)).size, 115,
        "the 114 text glyphs and one separate custom math glyph");
    assert.deepEqual(filled.slice(0, existing.length), existing,
        "custom Unicode stroke sequence and colors retain original order");
    assert.equal(encodeGlyphLibrary(existing), original,
        "the batch never mutates the source library");
    const authored = new Set(filled.map((line) => line.grapheme));
    for (const glyph of ["-", "¿", "¡", "«", "»", "‘", "’", "“", "”",
        "Á", "ü", "n\u0303", "U\u0308", "a\u0301", "0"]) {
        assert.ok(authored.has(glyph), `exact baseline glyph ${glyph}`);
    }
    for (const optional of ["+", "∂", "€", "%"]) {
        assert.ok(!authored.has(optional), `${optional} is not added`);
    }
    assert.equal(filled.filter((line) => line.grapheme === "ó").length, 1);
    assert.equal(filled.filter((line) => line.grapheme === "o\u0301").length,
        1, "existing NFD has independent authored ink");
    assert.strictEqual(fillMissingBilingualBaseline(filled,
        1550, 2450, 1100, 250, "#243647", { dx: 0, lift: 0 }), filled,
    "complete baseline is a no-op, including unchanged object identity");
    assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(filled)), filled);
});

test("connected bilingual fill admits 114 exact keys with continuous bases",
    () => {
        const existing = [stroke("∞"), ...geometricStarter("a",
            1550, 2450, 1100, 250, "#243647")];
        const before = encodeGlyphLibrary(existing);
        const filled = fillMissingBilingualBaseline(existing,
            1550, 2450, 1100, 250, "#243647",
            { dx: 0, lift: 0 }, "cursive");
        assert.equal(missingBilingualBaseline(filled).length, 0);
        assert.equal(new Set(filled.map((line) => line.grapheme)).size, 115);
        assert.deepEqual(filled.slice(0, existing.length), existing,
            "custom printed a and unrelated math ink stay untouched");
        assert.equal(encodeGlyphLibrary(existing), before);
        for (const base of "bcdefghijklmnopqrstuvwxyz") {
            assert.deepEqual(filled.filter((line) => line.grapheme === base),
                geometricStarter(base, 1550, 2450, 1100, 250,
                    "#243647", undefined, "cursive"));
        }
        for (const target of ["í", "i\u0301", "ñ", "n\u0303", "Á",
            "A\u0301", "Ü", "U\u0308"]) {
            assert.ok(filled.some((line) => line.grapheme === target));
        }
        assert.strictEqual(fillMissingBilingualBaseline(filled,
            1550, 2450, 1100, 250, "#243647",
            { dx: 0, lift: 0 }, "cursive"), filled);
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(filled)),
            filled);
    });

test("connected baseline fill is atomic on ambiguous i, clipping and cap",
    () => {
        for (const [existing, pos, error] of [
            [[stroke("i"), stroke("∞")], { dx: 0, lift: 0 },
                /Acute i requires one stem and a clear upper dot/],
            [[stroke("∞")], { dx: 0, lift: 500 },
                /must fit inside glyph cell/],
            [Array.from({ length: 4090 }, () => stroke("∞")),
                { dx: 0, lift: 0 }, /Save requires 1..4096/],
        ]) {
            const before = encodeGlyphLibrary(existing);
            assert.throws(() => fillMissingBilingualBaseline(existing,
                1550, 2450, 1100, 250, "#243647", pos, "cursive"),
            error);
            assert.equal(encodeGlyphLibrary(existing), before);
        }
    });

test("connected baseline button is session-gated, unsaved and idempotent",
    async () => {
        const ui = mountEditor();
        try {
            const { editor, get, downloads } = ui;
            assert.equal(get("seed-connected-baseline").disabled, true);
            get("seed-connected-baseline").dispatch("click");
            assert.equal(downloads.length, 0);
            editor.start();
            get("seed-connected-baseline").dispatch("click");
            assert.match(get("status").textContent,
                /Filled 114 missing connected bilingual baseline glyphs/);
            assert.match(get("baseline-coverage").textContent,
                /114\/114 exact glyphs; complete/);
            assert.equal(downloads.length, 0);
            get("character").value = "g";
            get("character").dispatch("input");
            assert.equal(get("preview").children.length, 91);
            get("save").dispatch("click");
            const bytes = await downloads.at(-1).text();
            const filled = parseGlyphLibrary(bytes);
            assert.equal(filled.length, 3091);
            assert.equal(new Set(filled.map((line) => line.grapheme)).size,
                114);
            get("seed-connected-baseline").dispatch("click");
            assert.match(get("status").textContent,
                /All 114 bilingual text glyphs already have ink/);
            get("save").dispatch("click");
            assert.equal(await downloads.at(-1).text(), bytes);
            editor.reset();
            assert.equal(get("seed-connected-baseline").disabled, true);
        } finally {
            ui.restore();
        }
    });

test("atomic bilingual fill refuses ambiguous i, clipped marks and caps",
    () => {
        const customI = [stroke("i"), stroke("∞")];
        const original = encodeGlyphLibrary(customI);
        assert.throws(() => fillMissingBilingualBaseline(customI,
            1550, 2450, 1100, 250, "#243647", { dx: 0, lift: 0 }),
        /Acute i requires one stem and a clear upper dot/);
        assert.equal(encodeGlyphLibrary(customI), original);
        const custom = [stroke("∞")];
        const before = encodeGlyphLibrary(custom);
        assert.throws(() => fillMissingBilingualBaseline(custom,
            1550, 2450, 1100, 250, "#243647", { dx: 0, lift: 500 }),
        /must fit inside glyph cell/);
        assert.equal(encodeGlyphLibrary(custom), before);
        const nearlyFull = Array.from({ length: 4090 }, () => stroke("∞"));
        const source = encodeGlyphLibrary(nearlyFull);
        assert.throws(() => fillMissingBilingualBaseline(nearlyFull,
            1550, 2450, 1100, 250, "#243647", { dx: 0, lift: 0 }),
        /Save requires 1..4096/);
        assert.equal(encodeGlyphLibrary(nearlyFull), source);
        // A separated short mark far from the i's upper stem is not its
        // dot; never discard it during an atomic NFC/NFD baseline fill.
        for (const form of ["single", "cursive"]) {
            const misaligned = geometricStarter("i", 1550, 2450,
                1100, 250, "#243647").map((line, index) => index === 1
                ? { ...line, x1: 2750, x2: 2750 } : line);
            const input = [...misaligned, stroke("∞")];
            const before = encodeGlyphLibrary(input);
            assert.throws(() => fillMissingBilingualBaseline(input,
                1550, 2450, 1100, 250, "#243647",
                { dx: 0, lift: 0 }, form), /aligned upper dot/);
            assert.equal(encodeGlyphLibrary(input), before);
        }
        // A manually imported centerline can be TSV-valid but physically
        // clipped: the bulk fill must not commit any preceding new glyphs.
        for (const form of ["single", "cursive"]) {
            const edge = [{ ...stroke("e"), x1: 0,
                y1: 1900, x2: 2300, y2: 3300 }, stroke("∞")];
            const input = encodeGlyphLibrary(edge);
            assert.throws(() => fillMissingBilingualBaseline(edge,
                1550, 2450, 1100, 250, "#243647",
                { dx: 0, lift: 0 }, form), /Source pen.*glyph cell/);
            assert.equal(encodeGlyphLibrary(edge), input);
        }
    });

test("baseline browser button is explicit, idempotent and session-gated",
    async () => {
        const ui = mountEditor();
        try {
            const { editor, get, downloads } = ui;
            assert.equal(get("seed-baseline").disabled, true);
            assert.match(get("baseline-coverage").textContent,
                /requires a compatible backend session/);
            get("seed-baseline").dispatch("click");
            assert.equal(downloads.length, 0);
            editor.start();
            assert.match(get("baseline-coverage").textContent,
                /0\/114 exact glyphs; 114 missing/);
            get("seed-baseline").dispatch("click");
            assert.match(get("status").textContent,
                /Filled 114 missing bilingual baseline glyphs/);
            assert.equal(downloads.length, 0, "fill is an unsaved draft");
            assert.match(get("baseline-coverage").textContent,
                /114\/114 exact glyphs; complete/);
            get("save").dispatch("click");
            const original = await downloads.at(-1).text();
            const filled = parseGlyphLibrary(original);
            assert.equal(filled.length, 2728);
            assert.equal(new Set(filled.map((line) => line.grapheme)).size,
                114);
            get("seed-baseline").dispatch("click");
            assert.match(get("status").textContent,
                /All 114 bilingual text glyphs already have ink/);
            get("save").dispatch("click");
            assert.equal(await downloads.at(-1).text(), original);
            editor.reset();
            assert.equal(get("seed-baseline").disabled, true);
            assert.match(get("baseline-coverage").textContent,
                /requires a compatible backend session/);
        } finally {
            ui.restore();
        }
    });

test("refused baseline browser fill preserves an imported custom i", async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        editor.start();
        const strokes = [stroke("i"), stroke("∞")];
        const saved = encodeGlyphLibrary(strokes);
        const bytes = new TextEncoder().encode(saved);
        get("load").files = [{
            size: bytes.length, arrayBuffer: async () => bytes.buffer,
        }];
        await get("load").dispatch("change");
        assert.match(get("baseline-coverage").textContent,
            /1\/114 exact glyphs; 113 missing/);
        get("seed-baseline").dispatch("click");
        assert.match(get("status").textContent,
            /Acute i requires one stem and a clear upper dot/);
        assert.match(get("baseline-coverage").textContent,
            /1\/114 exact glyphs; 113 missing/);
        assert.equal(downloads.length, 0,
            "refusal does not automatically download an incomplete draft");
        get("save").dispatch("click");
        assert.equal(await downloads.at(-1).text(), saved,
            "refusal never replaces caller-authored i or custom math ink");
    } finally {
        ui.restore();
    }
});

test("eleven explicit fills author 206 exact glyphs", async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        editor.start();
        for (const action of ["seed-lowercase", "seed-spanish",
            "seed-spanish-nfd", "seed-punctuation", "seed-quotes",
            "seed-math", "seed-digits", "seed-capitals",
            "seed-upper-spanish", "seed-upper-spanish-nfd",
            "seed-text-marks"]) {
            get(action).dispatch("click");
            assert.match(get("status").textContent, /Filled/);
        }
        assert.equal(downloads.length, 0);
        get("save").dispatch("click");
        const all = parseGlyphLibrary(await downloads.at(-1).text());
        assert.equal(all.length, 4084);
        assert.equal(new Set(all.map((line) => line.grapheme)).size, 206);
        const differentMinuses = ["-", "−"].map((glyph) =>
            all.filter((line) => line.grapheme === glyph));
        assert.equal(differentMinuses[0].length, 1);
        assert.equal(differentMinuses[1].length, 1);
        assert.notEqual(differentMinuses[0][0].grapheme,
            differentMinuses[1][0].grapheme);
        for (const glyph of ["⟨", "⟩"]) {
            get("form").value = glyph;
            get("character").value = glyph;
            get("form").dispatch("input");
            get("construct").dispatch("click");
            assert.match(get("status").textContent,
                /Constructed 2 sampled segments/);
        }
        get("save").dispatch("click");
        const manual = parseGlyphLibrary(await downloads.at(-1).text());
        assert.equal(manual.length, 4088);
        assert.equal(new Set(manual.map((line) => line.grapheme)).size, 208);
        assert.deepEqual(manual.slice(0, all.length), all,
            "manual glyphs never alter preexisting mass-filled strokes");
        get("seed-math").dispatch("click");
        assert.match(get("status").textContent, /All 93 math/);
        get("save").dispatch("click");
        assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
            manual, "automatic math fill does not overwrite manual angles");
        // The combined 4088-stroke library cannot admit a 65-stroke Greek
        // outline; the editor must refuse without changing saved TSV.
        get("form").value = "α";
        get("character").value = "α";
        get("form").dispatch("input");
        get("construct").dispatch("click");
        assert.match(get("status").textContent, /4096/);
        get("save").dispatch("click");
        assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
            manual, "over-cap manual Greek cannot corrupt prior TSV");
        editor.reset();
        assert.equal(get("seed-math").disabled, true);
        assert.equal(get("seed-digits").disabled, true);
        assert.equal(get("seed-capitals").disabled, true);
        assert.equal(get("seed-upper-spanish").disabled, true);
        assert.equal(get("seed-upper-spanish-nfd").disabled, true);
        assert.equal(get("seed-text-marks").disabled, true);
    } finally {
        ui.restore();
    }
});

test("all geometric starters respect pen margins over a boundary grid", () => {
    const forms = ["o", "a", "ó", "á", "b", "d", "p", "q",
        "g", "n", "u", "h", "m", "r", "s", "l", "t", "f", "v", "w",
        ".", ",", "!", "¡", "?", "¿", "«", "»", "‘", "’", "“", "”",
        "+", "-", "−", "±", "∓", "=", "×", "÷",
        "√", "π", "∞", "≠", "<", ">", "≤", "≥", "∧", "∨",
        "∩", "∪", "∈", "∉", "∋", "∌", "⊂", "⊆", "⊃", "⊇",
        "⊄", "⊈", "⊅", "⊉", "∑", "∫", "∂", "∮", "∬", "∭",
        "ℕ", "ℤ", "ℚ", "ℝ", "ℂ", "∛", "∜",
        "Δ", "≈", "→", "←", "↔", "↑", "↓", "↕", "∝",
        "∅", "∇", "≡", "≢", "∥", "⊥", "∼", "≅",
        "∀", "∃", "∄", "∴", "∵", "⊤", "⊢", "⊨",
        "∠", "∟", "⊬", "⊭", "¬", "⇒", "⇔",
        "≪", "≫", "≮", "≯", "≰", "≱", "↗", "↘",
        "↖", "↙", "⇐", "↦", "↤", "∣", "∤", "′", "″", "‴",
        "⋅", "∗", "∖", "∦", "⌊", "⌋", "⌈", "⌉",
        "⟨", "⟩", "α", "β", "θ", "λ", "μ", "γ",
        "δ", "σ", "ρ", "τ", "ε", "ω", "κ", "χ",
        "φ", "ψ", "ζ", "η", "ξ", "ι", "ο", "υ", "ν", "ς",
        "Γ", "Ω",
        "°", "%", "$", "€",
        ..."0123456789",
        ..."ABCDEFGHIJKLMNOPQRSTUVWXYZ",
        ";", ":", "\"", "'", "…", "–", "—", "(", ")", "[", "]",
        "/", "\\", "{", "}",
        "k", "x", "y", "z", "c", "e", "i", "í", "j"];
    let accepted = 0;
    let checked = 0;
    for (const glyph of forms) {
        for (const x of [1000, 1550, 2300]) {
            for (const y of [1000, 1700, 2450, 3150]) {
                for (const radius of [200, 750, 1100, 1450]) {
                    for (const width of [100, 250, 750]) {
                        checked += 1;
                        let lines;
                        try {
                            lines = geometricStarter(glyph, x, y, radius,
                                width, "#123456");
                        } catch {
                            continue;
                        }
                        accepted += 1;
                        assert.ok(lines.length > 0);
                        for (const line of lines) {
                            assert.equal(line.grapheme, glyph);
                            assert.equal(line.width, width);
                            for (const [px, py] of [
                                [line.x1, line.y1], [line.x2, line.y2],
                            ]) {
                                assert.ok(Number.isInteger(px)
                                    && Number.isInteger(py));
                                assert.ok(px - width / 2 >= 0
                                    && py - width / 2 >= 0
                                    && px + width / 2 <= 3500
                                    && py + width / 2 <= 4500,
                                `clipped ${glyph} at ${x},${y},${radius}`);
                            }
                        }
                    }
                }
            }
        }
    }
    assert.equal(checked, forms.length * 3 * 4 * 4 * 3);
    assert.ok(accepted > 1500, "boundary grid must test valid shapes");
});

test("g reuses exact o circle with a separate quadratic hook", () => {
    const args = [1550, 2450, 1100, 250, "#123456"];
    const o = geometricStarter("o", ...args);
    const g = geometricStarter("g", ...args);
    assert.equal(g.length, 65);
    assert.deepEqual(g.slice(0, 48), o.map((stroke) => ({
        ...stroke, grapheme: "g",
    })));
    assert.deepEqual(g[48], {
        grapheme: "g", x1: 2650, y1: 2450,
        x2: 2650, y2: 3165, width: 250, color: "#123456",
    });
    assert.deepEqual([g[49].x1, g[49].y1], [2650, 3165]);
    assert.deepEqual([g[64].x2, g[64].y2], [2100, 4045]);
    for (let index = 50; index <= 64; index += 1) {
        assert.deepEqual([g[index].x1, g[index].y1],
            [g[index - 1].x2, g[index - 1].y2]);
    }
    assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(g)), g);
    assert.throws(() => geometricStarter("g", 1550, 3150, 1100,
        250, "#123456"), /Loop hook and pen/);
    assert.throws(() => geometricStarter("g", ...args,
        undefined, "two-storey"), /Two-storey/);
});

test("g ideal circle and hook preserve authored TSV on clipping", async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        editor.start();
        get("character").value = "g";
        get("character").dispatch("input");
        get("form").value = "g";
        get("form").dispatch("change");
        assert.equal(get("ideal-circle").attributes.get("visibility"),
            "visible");
        assert.equal(get("ideal-stem").attributes.get("visibility"),
            "visible");
        assert.equal(get("ideal-stem").attributes.get("y2"), "3165");
        assert.equal(get("ideal-rounded").attributes.get("d"),
            "M 2650 3165 Q 2650 4045 2100 4045");
        assert.equal(get("ideal-rounded").attributes.get("visibility"),
            "visible");
        for (const id of ["ideal-rounded-left", "ideal-rounded-right",
            "ideal-e-bar", "ideal-i-stem", "ideal-i-dot"]) {
            assert.equal(get(id).attributes.get("visibility"), "hidden");
        }
        get("construct").dispatch("click");
        assert.equal(get("preview").children.length, 65);
        get("save").dispatch("click");
        const saved = parseGlyphLibrary(await downloads.at(-1).text());
        assert.deepEqual(saved, geometricStarter("g", 1550, 2450, 1100,
            250, "#243647"));
        get("circle-y").value = "3150";
        get("circle-y").dispatch("input");
        assert.equal(get("ideal-circle").attributes.get("visibility"),
            "hidden");
        get("construct").dispatch("click");
        assert.match(get("status").textContent, /Loop hook and pen/);
        get("save").dispatch("click");
        assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
            saved, "clipped g must preserve previous ink");
    } finally {
        ui.restore();
    }
});

test("dotted j has separate pen dot and 16 connected hook chords", () => {
    const args = [1550, 2450, 1100, 250, "#123456"];
    const j = geometricStarter("j", ...args);
    assert.equal(j.length, 18);
    assert.deepEqual(j[0], {
        grapheme: "j", x1: 1550, y1: 2010,
        x2: 1550, y2: 3220, width: 250, color: "#123456",
    });
    assert.deepEqual([j[1].x1, j[1].y1], [1550, 3220]);
    assert.deepEqual([j[16].x2, j[16].y2], [1000, 3990]);
    assert.deepEqual(j[17], {
        grapheme: "j", x1: 1550, y1: 1350,
        x2: 1550, y2: 1430, width: 250, color: "#123456",
    });
    for (let index = 2; index <= 16; index += 1) {
        assert.deepEqual([j[index].x1, j[index].y1],
            [j[index - 1].x2, j[index - 1].y2]);
    }
    assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(j)), j);
    assert.throws(() => geometricStarter("j", 1550, 3150, 1100,
        250, "#123456"), /Hooked descender and pen/);
    assert.throws(() => geometricStarter("j", ...args,
        undefined, "two-storey"), /Two-storey/);
});

test("j SVG and TSV agree; clipping refuses edits", async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        editor.start();
        get("character").value = "j";
        get("character").dispatch("input");
        get("form").value = "j";
        get("form").dispatch("change");
        assert.equal(get("ideal-rounded").attributes.get("d"),
            "M 1550 3220 Q 1550 3990 1000 3990");
        assert.equal(get("ideal-rounded").attributes.get("visibility"),
            "visible");
        assert.equal(get("ideal-i-stem").attributes.get("visibility"),
            "visible");
        assert.equal(get("ideal-i-stem").attributes.get("y2"), "3220");
        assert.equal(get("ideal-i-dot").attributes.get("visibility"),
            "visible");
        for (const hidden of ["ideal-rounded-left", "ideal-rounded-right",
            "ideal-circle", "ideal-e-bar"]) {
            assert.equal(get(hidden).attributes.get("visibility"), "hidden");
        }
        get("construct").dispatch("click");
        assert.equal(get("preview").children.length, 18);
        get("save").dispatch("click");
        const saved = parseGlyphLibrary(await downloads.at(-1).text());
        assert.deepEqual(saved, geometricStarter("j", 1550, 2450, 1100,
            250, "#243647"));
        get("circle-y").value = "3150";
        get("circle-y").dispatch("input");
        assert.equal(get("ideal-rounded").attributes.get("visibility"),
            "hidden");
        get("construct").dispatch("click");
        assert.match(get("status").textContent, /Hooked descender and pen/);
        get("save").dispatch("click");
        assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
            saved, "invalid j must not replace authored ink");
    } finally {
        ui.restore();
    }
});

test("f is taller and r has an n shoulder without right stem", () => {
    const args = [1550, 2450, 1100, 250, "#123456"];
    const f = geometricStarter("f", ...args);
    const r = geometricStarter("r", ...args);
    const n = geometricStarter("n", ...args);
    assert.deepEqual(f, [
        { grapheme: "f", x1: 1550, y1: 800, x2: 1550,
            y2: 3550, width: 250, color: "#123456" },
        { grapheme: "f", x1: 450, y1: 2450, x2: 2155,
            y2: 2450, width: 250, color: "#123456" },
    ]);
    assert.equal(r.length, 33);
    assert.deepEqual(r, n.slice(0, -1).map((stroke) => ({
        ...stroke, grapheme: "r",
    })), "r has a true quadratic shoulder but no dangling n right stem");
    assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary([...f, ...r])),
        [...f, ...r]);
    assert.throws(() => geometricStarter("f", 1550, 1450, 1100,
        250, "#123456"), /Tall stem and pen/);
    assert.throws(() => geometricStarter("r", 1550, 1100, 1100,
        250, "#123456"), /Shape and pen/);
    assert.throws(() => geometricStarter("r", ...args,
        undefined, "two-storey"), /Two-storey/);
});

test("f/r SVG previews match TSV and refuse clipped replacement", async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        editor.start();
        get("character").value = "f";
        get("character").dispatch("input");
        get("form").value = "f";
        get("form").dispatch("change");
        assert.equal(get("ideal-i-stem").attributes.get("visibility"),
            "visible");
        assert.equal(get("ideal-i-stem").attributes.get("y1"), "800");
        assert.equal(get("ideal-e-bar").attributes.get("visibility"),
            "visible");
        assert.equal(get("ideal-rounded").attributes.get("visibility"),
            "hidden");
        get("construct").dispatch("click");
        assert.equal(get("preview").children.length, 2);
        get("character").value = "r";
        get("character").dispatch("input");
        get("form").value = "r";
        get("form").dispatch("change");
        assert.equal(get("ideal-rounded").attributes.get("d"),
            "M 450 2450 Q 450 1350 1550 1350 Q 2650 1350 2650 2450");
        assert.equal(get("ideal-rounded").attributes.get("visibility"),
            "visible");
        assert.equal(get("ideal-rounded-left").attributes.get("visibility"),
            "visible");
        assert.equal(get("ideal-rounded-right").attributes.get("visibility"),
            "hidden");
        assert.equal(get("ideal-i-stem").attributes.get("visibility"),
            "hidden");
        get("construct").dispatch("click");
        assert.equal(get("preview").children.length, 33);
        get("save").dispatch("click");
        const saved = parseGlyphLibrary(await downloads.at(-1).text());
        assert.deepEqual(saved, [
            ...geometricStarter("f", 1550, 2450, 1100, 250,
                "#243647"),
            ...geometricStarter("r", 1550, 2450, 1100, 250,
                "#243647"),
        ]);
        get("character").value = "f";
        get("character").dispatch("input");
        get("form").value = "f";
        get("form").dispatch("change");
        get("circle-y").value = "1450";
        get("circle-y").dispatch("input");
        assert.equal(get("ideal-i-stem").attributes.get("visibility"),
            "hidden");
        get("construct").dispatch("click");
        assert.match(get("status").textContent, /Tall stem and pen/);
        get("save").dispatch("click");
        assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
            saved, "refused f must not overwrite either glyph");
    } finally {
        ui.restore();
    }
});

test("k/x/y/z straight templates retain independent pen paths", () => {
    const args = [1550, 2450, 1100, 250, "#123456"];
    const endpoints = {
        k: [
            [450, 800, 450, 3550], [2650, 1350, 450, 2450],
            [450, 2450, 2650, 3550],
        ],
        x: [
            [450, 1350, 2650, 3550], [2650, 1350, 450, 3550],
        ],
        y: [
            [450, 1350, 1550, 2450], [2650, 1350, 1550, 2450],
            [1550, 2450, 1000, 4100],
        ],
        z: [
            [450, 1350, 2650, 1350], [2650, 1350, 450, 3550],
            [450, 3550, 2650, 3550],
        ],
    };
    const library = [];
    for (const [glyph, expected] of Object.entries(endpoints)) {
        const lines = geometricStarter(glyph, ...args);
        assert.deepEqual(lines.map(({ x1, y1, x2, y2 }) =>
            [x1, y1, x2, y2]), expected);
        assert.deepEqual(lines.map(({ grapheme, width, color }) =>
            [grapheme, width, color]),
        expected.map(() => [glyph, 250, "#123456"]));
        library.push(...lines);
        assert.throws(() => geometricStarter(glyph, ...args,
            undefined, "two-storey"), /Two-storey/);
    }
    assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(library)), library);
    assert.throws(() => geometricStarter("k", 1550, 1350, 1100,
        250, "#123456"), /Straight strokes and pen/);
    assert.throws(() => geometricStarter("y", 1550, 3150, 1100,
        250, "#123456"), /Straight strokes and pen/);
    assert.throws(() => geometricStarter("y", 1550, 2450, 1100,
        1500, "#123456"), /Shape and pen/);
});

test("k/x/y/z SVG pen-up guides and native TSV stay synchronized", async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        editor.start();
        const paths = [
            ["k", "M 450 800 L 450 3550 M 2650 1350"
                + " L 450 2450 L 2650 3550", 3],
            ["x", "M 450 1350 L 2650 3550 M 2650 1350"
                + " L 450 3550", 2],
            ["y", "M 450 1350 L 1550 2450 M 2650 1350"
                + " L 1550 2450 L 1000 4100", 3],
            ["z", "M 450 1350 L 2650 1350 L 450 3550"
                + " L 2650 3550", 3],
        ];
        for (const [glyph, d, count] of paths) {
            get("character").value = glyph;
            get("character").dispatch("input");
            get("form").value = glyph;
            get("form").dispatch("change");
            assert.equal(get("ideal-rounded").attributes.get("d"), d);
            assert.equal(get("ideal-preview").attributes.get("aria-label"),
                `Exact straight ${glyph} guide, retained in TSV`);
            assert.equal(get("ideal-rounded").attributes.get("visibility"),
                "visible");
            for (const other of ["ideal-rounded-left", "ideal-rounded-right",
                "ideal-circle", "ideal-i-stem", "ideal-e-bar"]) {
                assert.equal(get(other).attributes.get("visibility"), "hidden");
            }
            get("construct").dispatch("click");
            assert.equal(get("preview").children.length, count);
        }
        get("save").dispatch("click");
        const accepted = parseGlyphLibrary(await downloads.at(-1).text());
        assert.deepEqual(accepted, paths.flatMap(([glyph]) =>
            geometricStarter(glyph, 1550, 2450, 1100, 250, "#243647")));
        for (const [glyph, y] of [["k", "1350"], ["y", "3150"]]) {
            get("character").value = glyph;
            get("character").dispatch("input");
            get("form").value = glyph;
            get("form").dispatch("change");
            get("circle-y").value = y;
            get("circle-y").dispatch("input");
            assert.equal(get("ideal-rounded").attributes.get("visibility"),
                "hidden");
            get("construct").dispatch("click");
            assert.match(get("status").textContent, /Straight strokes and pen/);
            get("save").dispatch("click");
            assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
                accepted, `clipped ${glyph} must not replace saved ink`);
            get("circle-y").value = "2450";
            get("circle-y").dispatch("input");
        }
    } finally {
        ui.restore();
    }
});

test("l and t preserve exact straight ink and bounded ascenders", () => {
    const args = [1550, 2450, 1100, 250, "#123456"];
    const l = geometricStarter("l", ...args);
    const t = geometricStarter("t", ...args);
    assert.deepEqual(l, [{
        grapheme: "l", x1: 1550, y1: 800, x2: 1550, y2: 3550,
        width: 250, color: "#123456",
    }]);
    assert.deepEqual(t, [
        { grapheme: "t", x1: 1550, y1: 1350,
            x2: 1550, y2: 3550, width: 250, color: "#123456" },
        { grapheme: "t", x1: 450, y1: 2450,
            x2: 2155, y2: 2450, width: 250, color: "#123456" },
    ]);
    assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary([...l, ...t])),
        [...l, ...t]);
    assert.throws(() => geometricStarter("l", 1550, 1450, 1100,
        250, "#123456"), /Tall stem and pen/);
    assert.throws(() => geometricStarter("l", ...args,
        undefined, "two-storey"), /Two-storey/);
});

test("l/t exact SVG guides and atomic explicit replacement", async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        editor.start();
        for (const [glyph, y, lines] of [
            ["l", "800", 1], ["t", "1350", 2],
        ]) {
            get("character").value = glyph;
            get("character").dispatch("input");
            get("form").value = glyph;
            get("form").dispatch("change");
            assert.equal(get("ideal-i-stem").attributes.get("visibility"),
                "visible");
            assert.equal(get("ideal-i-stem").attributes.get("y1"), y);
            assert.equal(get("ideal-i-stem").attributes.get("y2"),
                "3550");
            assert.equal(get("ideal-i-dot").attributes.get("visibility"),
                "hidden");
            assert.equal(get("ideal-e-bar").attributes.get("visibility"),
                glyph === "t" ? "visible" : "hidden");
            assert.equal(get("ideal-circle").attributes.get("visibility"),
                "hidden");
            get("construct").dispatch("click");
            assert.equal(get("preview").children.length, lines);
        }
        get("save").dispatch("click");
        const saved = parseGlyphLibrary(await downloads.at(-1).text());
        assert.deepEqual(saved, [
            ...geometricStarter("l", 1550, 2450, 1100, 250,
                "#243647"),
            ...geometricStarter("t", 1550, 2450, 1100, 250,
                "#243647"),
        ]);
        get("character").value = "l";
        get("character").dispatch("input");
        get("form").value = "l";
        get("form").dispatch("change");
        get("circle-y").value = "1450";
        get("circle-y").dispatch("input");
        assert.equal(get("ideal-i-stem").attributes.get("visibility"),
            "hidden");
        get("construct").dispatch("click");
        assert.match(get("status").textContent, /Tall stem and pen/);
        get("save").dispatch("click");
        assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
            saved, "clipped l must not change prior authored strokes");
    } finally {
        ui.restore();
    }
});

test("c and e open-circle arcs and dotted i have ordered ink", () => {
    const args = [1550, 2450, 1100, 250, "#123456"];
    const c = geometricStarter("c", ...args);
    const e = geometricStarter("e", ...args);
    const i = geometricStarter("i", ...args);
    const acuteI = geometricStarter("í", ...args);
    assert.equal(c.length, 36);
    assert.equal(e.length, 37);
    assert.deepEqual(e.slice(0, 36), c.map((line) => ({
        ...line, grapheme: "e",
    })));
    assert.deepEqual(e.at(-1), {
        grapheme: "e", x1: 450, y1: 2450,
        x2: 2155, y2: 2450, width: 250, color: "#123456",
    });
    assert.deepEqual(acuteI, [
        { grapheme: "í", x1: 1550, y1: 2010,
            x2: 1550, y2: 3550, width: 250, color: "#123456" },
        { grapheme: "í", x1: 1165, y1: 980,
            x2: 1825, y2: 485, width: 250, color: "#123456" },
    ], "the accented i uses an acute instead of a dot");
    assert.deepEqual(i, [
        { grapheme: "i", x1: 1550, y1: 2010,
            x2: 1550, y2: 3550, width: 250, color: "#123456" },
        { grapheme: "i", x1: 1550, y1: 1350,
            x2: 1550, y2: 1430, width: 250, color: "#123456" },
    ]);
    for (let index = 0; index < c.length; index += 1) {
        const line = c[index];
        if (index > 0) {
            assert.deepEqual([line.x1, line.y1],
                [c[index - 1].x2, c[index - 1].y2]);
        }
        for (const [x, y] of [
            [line.x1, line.y1], [line.x2, line.y2],
        ]) assert.ok(Math.abs(Math.hypot(x - 1550, y - 2450) - 1100) < 1);
    }
    assert.notDeepEqual([c[0].x1, c[0].y1],
        [c.at(-1).x2, c.at(-1).y2], "C-shaped arc remains open");
    const acuteE = composeAccentedGlyph(e, "e", "é", "#123456");
    assert.equal(acuteE.length, 38);
    assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary([
        ...c, ...e, ...i, ...acuteE, ...acuteI,
    ])).length, 115);
    assert.throws(() => geometricStarter("i", 1550, 2450, 200, 250,
        "#123456"), /Dotted i requires visible pen-clearance/);
    assert.throws(() => geometricStarter("e", 150, 2450, 500, 250,
        "#123456"), /Shape and pen/);
});

test("open arc and dotted stem guides replace exact glyphs only", async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        editor.start();
        for (const [glyph, total] of [
            ["c", 36], ["e", 37], ["i", 2], ["í", 2],
        ]) {
            get("character").value = glyph;
            get("character").dispatch("input");
            get("form").value = glyph;
            get("form").dispatch("change");
            assert.equal(get("ideal-circle").attributes.get("visibility"),
                "hidden");
            assert.equal(get("ideal-c-loop").attributes.get("visibility"),
                ["i", "í"].includes(glyph) ? "hidden" : "visible");
            assert.equal(get("ideal-e-bar").attributes.get("visibility"),
                glyph === "e" ? "visible" : "hidden");
            assert.equal(get("ideal-i-dot").attributes.get("visibility"),
                glyph === "i" ? "visible" : "hidden");
            assert.equal(get("ideal-i-stem").attributes.get("visibility"),
                ["i", "í"].includes(glyph) ? "visible" : "hidden");
            assert.equal(get("ideal-accent").attributes.get("visibility"),
                glyph === "í" ? "visible" : "hidden");
            get("construct").dispatch("click");
            assert.equal(get("preview").children.length, total);
        }
        assert.match(get("ideal-c-loop").attributes.get("d"),
            / A 1100 1100 0 1 1 /);
        get("character").value = "é";
        get("character").dispatch("input");
        get("copy-from").value = "e";
        get("compose").dispatch("click");
        assert.equal(get("preview").children.length, 38);
        get("save").dispatch("click");
        const accepted = parseGlyphLibrary(await downloads.at(-1).text());
        assert.equal(accepted.length, 115);
        get("form").value = "i";
        get("form").dispatch("change");
        get("character").value = "i";
        get("character").dispatch("input");
        get("circle-radius").value = "200";
        get("construct").dispatch("click");
        assert.match(get("status").textContent, /Dotted i requires/);
        get("save").dispatch("click");
        assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
            accepted, "invalid dot clearance leaves all templates intact");
    } finally {
        ui.restore();
    }
});

test("geometric replacement preserves glyph identity and bounds", async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        editor.start();
        const original = [stroke("ñ"), stroke("o"), stroke("∞")];
        const bytes = new TextEncoder().encode(encodeGlyphLibrary(original));
        get("load").files = [{
            size: bytes.length, arrayBuffer: async () => bytes.buffer,
        }];
        await get("load").dispatch("change");
        get("construct").dispatch("click");
        assert.match(get("status").textContent,
            /Choose the matching geometric starter glyph/);
        get("character").value = "o";
        get("character").dispatch("input");
        get("circle-radius").value = "3200";
        get("construct").dispatch("click");
        assert.match(get("status").textContent, /outside native bounds/);
        assert.equal(get("preview").children.length, 1,
            "invalid construction leaves previous glyph intact");
        get("circle-radius").value = "1100";
        get("construct").dispatch("click");
        assert.equal(get("preview").children.length, 48);
        get("character").value = "a";
        get("character").dispatch("input");
        get("form").value = "a";
        get("construct").dispatch("click");
        assert.equal(get("preview").children.length, 49);
        for (const [grapheme, expected] of [["ó", 49], ["á", 50]]) {
            get("character").value = grapheme;
            get("character").dispatch("input");
            get("form").value = grapheme;
            get("form").dispatch("change");
            get("accent-shift").value = grapheme === "ó" ? "100" : "0";
            get("accent-lift").value = grapheme === "ó" ? "100" : "0";
            get("construct").dispatch("click");
            assert.equal(get("preview").children.length, expected);
        }
        get("character").value = "a";
        get("character").dispatch("input");
        get("form").value = "a-two";
        get("form").dispatch("change");
        get("construct").dispatch("click");
        assert.equal(get("preview").children.length, 81);
        get("character").value = "á";
        get("character").dispatch("input");
        get("form").value = "á-two";
        get("form").dispatch("change");
        get("construct").dispatch("click");
        assert.match(get("status").textContent, /Acute accent/);
        assert.equal(get("preview").children.length, 50,
            "invalid accented upper story leaves previous á intact");
        get("circle-radius").value = "600";
        get("construct").dispatch("click");
        assert.equal(get("preview").children.length, 82);
        get("circle-radius").value = "1100";
        get("character").value = "ó";
        get("character").dispatch("input");
        get("form").value = "ó";
        get("form").dispatch("change");
        get("accent-shift").value = "600";
        get("construct").dispatch("click");
        assert.match(get("status").textContent, /Acute shift/);
        assert.equal(get("preview").children.length, 49);
        get("accent-shift").value = "100";
        get("circle-y").value = "1600";
        get("construct").dispatch("click");
        assert.match(get("status").textContent, /Acute accent/);
        assert.equal(get("preview").children.length, 49,
            "failed acute replacement preserves the existing ó ink");
        get("circle-y").value = "2450";
        get("save").dispatch("click");
        const saved = parseGlyphLibrary(await downloads[0].text());
        assert.deepEqual(saved.slice(0, 2), [original[0], original[2]]);
        for (const [grapheme, lines] of [
            ["o", geometricStarter(
                "o", 1550, 2450, 1100, 250, "#243647")],
            ["a", geometricStarter(
                "a", 1550, 2450, 1100, 250, "#243647",
                undefined, "two-storey")],
            ["ó", geometricStarter(
                "ó", 1550, 2450, 1100, 250, "#243647",
                { dx: 100, lift: 100 })],
            ["á", geometricStarter(
                "á", 1550, 2450, 600, 250, "#243647",
                { dx: 0, lift: 0 }, "two-storey")],
        ]) {
            assert.deepEqual(saved.filter((item) => item.grapheme ===
                grapheme), lines);
        }
        editor.reset();
        assert.equal(get("construct").disabled, true);
    } finally {
        ui.restore();
    }
});


test("exact SVG circle guide tracks parameters without saving", async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        assert.equal(get("ideal-circle").attributes.get("visibility"),
            "hidden");
        editor.start();
        assert.equal(get("ideal-circle").attributes.get("visibility"),
            "visible");
        assert.equal(get("ideal-stem").attributes.get("visibility"),
            "hidden");
        get("circle-x").value = "1600";
        get("circle-x").dispatch("input");
        assert.equal(get("ideal-circle").attributes.get("cx"), "1600");
        get("form").value = "a";
        get("form").dispatch("change");
        assert.equal(get("ideal-stem").attributes.get("visibility"),
            "visible");
        assert.equal(get("ideal-stem").attributes.get("x1"), "2700");
        get("form").value = "a-two";
        get("form").dispatch("change");
        assert.equal(get("ideal-arch").attributes.get("visibility"),
            "visible");
        assert.equal(get("ideal-arch").attributes.get("d"),
            "M 822 1672 Q 500 250 1600 250 Q 2700 250 2700 1350");
        get("form").value = "ó";
        get("form").dispatch("change");
        assert.equal(get("ideal-arch").attributes.get("visibility"),
            "hidden");
        assert.equal(get("ideal-stem").attributes.get("visibility"),
            "hidden");
        assert.equal(get("ideal-accent").attributes.get("visibility"),
            "visible");
        assert.equal(get("ideal-accent").attributes.get("x1"), "1215");
        get("accent-shift").value = "100";
        get("accent-shift").dispatch("input");
        get("accent-lift").value = "120";
        get("accent-lift").dispatch("input");
        assert.equal(get("ideal-accent").attributes.get("x1"), "1315");
        assert.equal(get("ideal-accent").attributes.get("y1"), "860");
        get("form").value = "á";
        get("form").dispatch("change");
        assert.equal(get("ideal-stem").attributes.get("visibility"),
            "visible");
        get("circle-y").value = "1600";
        get("circle-y").dispatch("input");
        assert.equal(get("ideal-circle").attributes.get("visibility"),
            "hidden", "invalid acute clearance hides the entire guide");
        assert.equal(get("ideal-accent").attributes.get("visibility"),
            "hidden");
        assert.equal(get("ideal-arch").attributes.get("visibility"),
            "hidden");
        get("circle-y").value = "2450";
        get("circle-y").dispatch("input");
        get("circle-radius").value = "1900";
        get("circle-radius").dispatch("input");
        assert.equal(get("ideal-circle").attributes.get("visibility"),
            "hidden", "invalid guide cannot display stale accepted geometry");
        assert.equal(get("ideal-stem").attributes.get("visibility"),
            "hidden");
        assert.equal(get("ideal-accent").attributes.get("visibility"),
            "hidden");
        get("save").dispatch("click");
        assert.equal(downloads.length, 0, "preview never persists ink");
        get("circle-radius").value = "1100";
        get("circle-radius").dispatch("input");
        assert.equal(get("ideal-circle").attributes.get("visibility"),
            "visible");
        editor.reset();
        assert.equal(get("ideal-arch").attributes.get("visibility"),
            "hidden");
        assert.equal(get("ideal-circle").attributes.get("visibility"),
            "hidden");
        assert.equal(get("ideal-stem").attributes.get("visibility"),
            "hidden");
        assert.equal(get("ideal-accent").attributes.get("visibility"),
            "hidden");
    } finally {
        ui.restore();
    }
});

test("cursive o/a starters keep directed entry, loop and exit", () => {
    const values = [1550, 2450, 700, 250, "#123456"];
    for (const [glyph, count, paths] of [
        ["o", 74, 1], ["a", 76, 1],
        ["ó", 75, 2], ["á", 77, 2],
    ]) {
        const base = geometricStarter(glyph, ...values);
        const cursive = geometricStarter(glyph, ...values,
            undefined, "cursive");
        assert.equal(cursive.length, count, `exact ${glyph} pen segments`);
        assert.equal(candidatePenPaths(cursive, glyph).length, paths,
            `${glyph} loop and connectors share one directed pen path`);
        const loop = [...base.slice(24, 48), ...base.slice(0, 24)];
        assert.deepEqual(cursive.slice(1, 49), loop,
            "cursive first loop preserves all 48 exact base circle chords");
        assert.deepEqual(cursive.slice(49, 73), base.slice(24, 48),
            "second half-loop carries connected pen from left to right");
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(cursive)),
            cursive, `native exact Unicode TSV for ${glyph}`);
        assert.deepEqual(geometricStarter(glyph, ...values), base,
            "choosing cursive cannot mutate its regular form");
    }
    for (const invalid of ["0", "A", "∞"]) {
        assert.throws(() => geometricStarter(invalid, ...values,
            undefined, "cursive"), /Cursive variant/);
    }
    assert.throws(() => geometricStarter("o", 1550, 2450,
        1300, 250, "#123456", undefined, "cursive"),
    /Cursive connector and pen clip/);
});

test("browser cursive o/a guide and replacement stay deliberate", async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        editor.start();
        get("form").value = "o-cursive";
        get("form").dispatch("change");
        assert.equal(get("ideal-entry").attributes.get("visibility"),
            "visible");
        assert.equal(get("ideal-exit").attributes.get("visibility"),
            "visible");
        assert.equal(get("ideal-approach").attributes.get("visibility"),
            "hidden");
        assert.match(get("ideal-preview").attributes.get("aria-label"),
            /no adjacent joining/);
        get("character").value = "o";
        get("character").dispatch("input");
        get("construct").dispatch("click");
        get("save").dispatch("click");
        assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
            geometricStarter("o", 1550, 2450, 1100, 250, "#243647",
                undefined, "cursive"));
        get("form").value = "a-cursive";
        get("form").dispatch("change");
        assert.equal(get("ideal-approach").attributes.get("visibility"),
            "visible");
        get("circle-radius").value = "1300";
        get("circle-radius").dispatch("input");
        assert.equal(get("ideal-entry").attributes.get("visibility"),
            "hidden", "invalid geometry hides stale connector guides");
        get("circle-radius").value = "700";
        get("circle-radius").dispatch("input");
        assert.equal(get("ideal-entry").attributes.get("visibility"),
            "visible");
        editor.reset();
        assert.equal(get("ideal-exit").attributes.get("visibility"),
            "hidden");
    } finally {
        ui.restore();
    }
});

test("italic o/a variants shear the ideal base around a fixed baseline", () => {
    const values = [1550, 2450, 700, 250, "#123456"];
    const baseline = values[1] + values[2];
    for (const [glyph, regular, italic, count] of [
        ["o", "single", "italic", 48],
        ["a", "single", "italic", 49],
        ["ó", "single", "italic", 49],
        ["á", "single", "italic", 50],
        ["a", "two-storey", "two-storey-italic", 81],
        ["á", "two-storey", "two-storey-italic", 82],
    ]) {
        const base = geometricStarter(glyph, ...values, undefined, regular);
        const saved = encodeGlyphLibrary(base);
        const position = { dx: 0, lift: 0, italicDegrees: 12 };
        const result = geometricStarter(glyph, ...values, position, italic);
        const slope = Math.tan(12 * Math.PI / 180);
        assert.equal(result.length, count);
        assert.deepEqual(result, base.map((stroke) => ({
            ...stroke,
            x1: Math.round(stroke.x1 + slope * (baseline - stroke.y1)),
            x2: Math.round(stroke.x2 + slope * (baseline - stroke.y2)),
        })), `${glyph} retains each stroke, color and order`);
        assert.notDeepEqual(result, base, `${glyph} italic is distinct`);
        assert.deepEqual(geometricStarter(glyph, ...values,
            { ...position, italicDegrees: 0 }, italic), base);
        assert.equal(encodeGlyphLibrary(base), saved,
            "regular outlines remain unchanged by italic derivation");
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(result)),
            result, "sampled italic ink has native v1 byte parity");
    }
    for (const invalid of [-1, 21, 1.5, NaN, Infinity]) {
        assert.throws(() => geometricStarter("o", ...values,
            { dx: 0, lift: 0, italicDegrees: invalid }, "italic"),
        /Italic angle/);
    }
    assert.throws(() => geometricStarter("n", ...values,
        undefined, "italic"), /Italic variant/);
    assert.throws(() => geometricStarter("o", ...values,
        undefined, "two-storey-italic"), /Two-storey/);
    assert.throws(() => geometricStarter("a", 2900, 2450, 400,
        250, "#123456", { dx: 0, lift: 0, italicDegrees: 20 },
        "italic"), /Italic ink and pen clip/);
});

test("italic SVG guide and replacement are session-safe", async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads, links } = ui;
        assert.equal(get("italic-angle").disabled, true);
        editor.start();
        assert.equal(get("italic-angle").disabled, true,
            "regular geometry does not enable italic control");
        get("form").value = "a-italic";
        get("form").dispatch("change");
        assert.equal(get("italic-angle").disabled, false);
        assert.equal(get("ideal-circle").attributes.get("visibility"),
            "visible");
        assert.equal(get("ideal-stem").attributes.get("visibility"),
            "visible");
        assert.match(get("ideal-geometry").attributes.get("transform"),
            /^matrix\(1 0 -0\.2125.+ 1 .+ 0\)$/);
        get("character").value = "a";
        get("character").dispatch("input");
        get("construct").dispatch("click");
        get("save").dispatch("click");
        assert.equal(links.at(-1).name, "my-glyph-strokes.tsv");
        const first = parseGlyphLibrary(await downloads.at(-1).text());
        assert.deepEqual(first, geometricStarter("a", 1550, 2450,
            1100, 250, "#243647",
            { dx: 0, lift: 0, italicDegrees: 12 }, "italic"));
        get("form").value = "a-two-italic";
        get("form").dispatch("change");
        assert.equal(get("ideal-arch").attributes.get("visibility"),
            "visible", "printed italic arch uses the same ideal bowl");
        get("circle-radius").value = "800";
        get("circle-radius").dispatch("input");
        assert.equal(get("ideal-arch").attributes.get("visibility"),
            "visible");
        get("italic-angle").value = "21";
        get("italic-angle").dispatch("input");
        assert.equal(get("ideal-circle").attributes.get("visibility"),
            "hidden", "invalid angle hides the whole stale guide");
        const count = downloads.length;
        get("construct").dispatch("click");
        get("save").dispatch("click");
        assert.equal(downloads.length, count + 1,
            "only explicit Save downloads the unchanged accepted draft");
        assert.deepEqual(parseGlyphLibrary(
            await downloads.at(-1).text()), first);
        get("italic-angle").value = "8";
        get("italic-angle").dispatch("input");
        assert.equal(get("ideal-arch").attributes.get("visibility"),
            "visible");
        get("construct").dispatch("click");
        get("save").dispatch("click");
        assert.deepEqual(parseGlyphLibrary(
            await downloads.at(-1).text()),
        geometricStarter("a", 1550, 2450, 800, 250, "#243647",
            { dx: 0, lift: 0, italicDegrees: 8 }, "two-storey-italic"));
        get("form").value = "o";
        get("form").dispatch("change");
        assert.equal(get("italic-angle").disabled, true);
        assert.equal(get("ideal-geometry").attributes.get("transform"),
            "", "the regular perfect-circle SVG is not sheared");
        editor.reset();
        assert.equal(get("italic-angle").disabled, true);
        assert.equal(get("ideal-circle").attributes.get("visibility"),
            "hidden");
    } finally {
        ui.restore();
    }
});

test("authored italic shear preserves unrelated Unicode and pen paths", () => {
    const nfd = "n\u0303";
    const first = { ...stroke(nfd), x1: 450, y1: 1000,
        x2: 850, y2: 1600, width: 250 };
    const next = { ...first, x1: 850, y1: 1600,
        x2: 1300, y2: 2200 };
    const accent = { ...first, x1: 1100, y1: 200,
        x2: 1700, y2: 600, color: "#112233" };
    const composed = { ...stroke("ñ"), x1: 300, y1: 350,
        x2: 500, y2: 550 };
    const math = { ...stroke("∞"), x1: 2700, y1: 3300,
        x2: 2800, y2: 3450 };
    const library = [first, composed, next, math, accent];
    const before = encodeGlyphLibrary(library);
    const slope = Math.tan(12 * Math.PI / 180);
    const expected = library.map((line) => line.grapheme !== nfd
        ? line : {
            ...line,
            x1: Math.round(line.x1 + slope * (3550 - line.y1)),
            x2: Math.round(line.x2 + slope * (3550 - line.y2)),
        });
    const actual = italicizeGlyphStrokes(library, nfd, 12, 3550);
    assert.deepEqual(actual, expected);
    assert.equal(actual[1], composed);
    assert.equal(actual[3], math);
    assert.equal(encodeGlyphLibrary(library), before,
        "shearing is a copy, not an in-place edit");
    assert.deepEqual(candidatePenPaths(actual, nfd).map((path) =>
        [path.first, path.last]), [[1, 2], [3, 3]],
    "transformed touching endpoints preserve the original pen lifts");
    assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(actual)), actual);
    assert.deepEqual(italicizeGlyphStrokes(library, nfd, 0, 3550),
        library, "zero slant is byte-identical to the original");
    assert.deepEqual(italicizeGlyphStrokes(library, "ñ", 12, 3550)
        .filter((line) => line.grapheme === nfd),
    library.filter((line) => line.grapheme === nfd));
    for (const angle of [-1, 21, 12.5, NaN, Infinity]) {
        assert.throws(() => italicizeGlyphStrokes(library, nfd,
            angle, 3550), /Italic angle/);
    }
    for (const pivot of [-1, 4501, 1.5, NaN]) {
        assert.throws(() => italicizeGlyphStrokes(library, nfd,
            12, pivot), /italic baseline/);
    }
    assert.throws(() => italicizeGlyphStrokes(library, "ab", 12,
        3550), /exact visible/);
    assert.throws(() => italicizeGlyphStrokes(library, "á", 12,
        3550), /no authored ink/);
    const clipped = [{ ...stroke("a"), x1: 3100, y1: 300,
        x2: 3200, y2: 900, width: 250 }];
    assert.throws(() => italicizeGlyphStrokes(clipped, "a",
        20, 3550), /Italic ink and pen clip/);
    assert.deepEqual(clipped[0].x1, 3100,
        "clip refusal leaves the caller-owned ink unchanged");
    const full = Array.from({ length: 4096 }, (_, index) => ({
        ...first, x1: 450 + index % 100, x2: 850 + index % 100,
    }));
    assert.equal(italicizeGlyphStrokes(full, nfd, 12, 3550).length,
        4096, "same native capacity remains supported");
});

test("authored italic UI refuses invalid edits without saving", async () => {
    const ui = mountEditor();
    try {
        const { get, editor, downloads } = ui;
        const nfd = "n\u0303";
        assert.equal(get("slant-authored").disabled, true);
        editor.start();
        assert.equal(get("slant-authored").disabled, true,
            "missing glyph ink cannot be transformed");
        const add = (glyph, coords) => {
            get("character").value = glyph;
            get("character").dispatch("input");
            for (const [key, value] of Object.entries({
                x1: coords[0], y1: coords[1],
                x2: coords[2], y2: coords[3], width: 250,
            })) get(key).value = String(value);
            get("add").dispatch("click");
        };
        add(nfd, [450, 1000, 850, 1600]);
        add("ñ", [300, 350, 500, 550]);
        add(nfd, [850, 1600, 1300, 2200]);
        assert.equal(get("slant-authored").disabled, false);
        const original = [
            { grapheme: nfd, x1: 450, y1: 1000,
                x2: 850, y2: 1600, width: 250, color: "#243647" },
            { grapheme: "ñ", x1: 300, y1: 350,
                x2: 500, y2: 550, width: 250, color: "#243647" },
            { grapheme: nfd, x1: 850, y1: 1600,
                x2: 1300, y2: 2200, width: 250, color: "#243647" },
        ];
        get("slant-authored").dispatch("click");
        assert.match(get("status").textContent,
            /Slanted all authored ñ ink; unsaved/);
        assert.equal(downloads.length, 0,
            "explicit italic edit is only an unsaved draft");
        get("save").dispatch("click");
        const accepted = parseGlyphLibrary(await downloads.at(-1).text());
        assert.deepEqual(accepted,
            italicizeGlyphStrokes(original, nfd, 12, 3550));
        get("authored-angle").value = "21";
        get("slant-authored").dispatch("click");
        assert.match(get("status").textContent, /Italic angle/);
        get("save").dispatch("click");
        assert.deepEqual(parseGlyphLibrary(
            await downloads.at(-1).text()), accepted);
        get("authored-angle").value = "12";
        get("authored-baseline").value = "";
        get("slant-authored").dispatch("click");
        assert.match(get("status").textContent, /Choose an italic angle/);
        get("authored-baseline").value = "3550";
        get("character").value = "ó";
        get("character").dispatch("input");
        assert.equal(get("slant-authored").disabled, true);
        get("character").value = nfd;
        get("character").dispatch("input");
        assert.equal(get("slant-authored").disabled, false);
        editor.reset();
        assert.equal(get("slant-authored").disabled, true);
        assert.equal(get("authored-angle").disabled, true);
    } finally {
        ui.restore();
    }
});

test("last authored slant undo/redo restores exact Unicode without writes",
    async () => {
        const ui = mountEditor();
        try {
            const { get, editor, downloads } = ui;
            const nfd = "a\u0301";
            const undo = get("slant-undo");
            const redo = get("slant-redo");
            assert.equal(undo.disabled, true);
            assert.equal(redo.disabled, true);
            editor.start();
            const append = (grapheme, coordinates) => {
                get("character").value = grapheme;
                get("character").dispatch("input");
                for (const [key, value] of Object.entries({
                    x1: coordinates[0], y1: coordinates[1],
                    x2: coordinates[2], y2: coordinates[3], width: 250,
                })) get(key).value = String(value);
                get("add").dispatch("click");
            };
            append(nfd, [450, 1000, 850, 1600]);
            append("á", [600, 1200, 900, 1550]);
            append(nfd, [850, 1600, 1300, 2200]);
            const original = [
                { grapheme: nfd, x1: 450, y1: 1000,
                    x2: 850, y2: 1600, width: 250, color: "#243647" },
                { grapheme: "á", x1: 600, y1: 1200,
                    x2: 900, y2: 1550, width: 250, color: "#243647" },
                { grapheme: nfd, x1: 850, y1: 1600,
                    x2: 1300, y2: 2200, width: 250, color: "#243647" },
            ];
            get("slant-authored").dispatch("click");
            const italic = italicizeGlyphStrokes(original, nfd, 12, 3550);
            assert.equal(undo.disabled, false);
            assert.equal(redo.disabled, true);
            assert.equal(downloads.length, 0,
                "slant and undo history never download automatically");
            get("character").value = "á";
            get("character").dispatch("input");
            assert.equal(undo.disabled, true,
                "undo cannot replace a different selected glyph");
            get("character").value = nfd;
            get("character").dispatch("input");
            assert.equal(undo.disabled, false);
            undo.dispatch("click");
            assert.equal(undo.disabled, false,
                "earlier accepted pen additions remain recoverable");
            assert.equal(undo.textContent, "Undo last stroke addition");
            assert.equal(redo.disabled, false);
            assert.match(get("status").textContent,
                /Undid last glyph slant; unsaved/);
            get("save").dispatch("click");
            assert.equal(await downloads.at(-1).text(),
                encodeGlyphLibrary(original), "undo is byte-exact");
            redo.dispatch("click");
            assert.equal(redo.disabled, true);
            assert.equal(undo.disabled, false);
            get("save").dispatch("click");
            assert.equal(await downloads.at(-1).text(),
                encodeGlyphLibrary(italic), "redo is byte-exact");
            // A refused slant leaves valid undo/redo and accepted ink intact.
            get("authored-angle").value = "21";
            get("slant-authored").dispatch("click");
            assert.match(get("status").textContent, /Italic angle/);
            assert.equal(undo.disabled, false);
            undo.dispatch("click");
            // Mutating an unrelated exact Unicode row after undo must
            // invalidate redo, never silently discard that new edit.
            append("∞", [700, 200, 900, 500]);
            assert.equal(redo.disabled, true);
            redo.dispatch("click");
            get("save").dispatch("click");
            assert.equal(await downloads.at(-1).text(),
                encodeGlyphLibrary([...original, {
                    grapheme: "∞", x1: 700, y1: 200,
                    x2: 900, y2: 500, width: 250,
                    color: "#243647",
                }]));
            get("authored-angle").value = "0";
            get("character").value = nfd;
            get("character").dispatch("input");
            get("slant-authored").dispatch("click");
            assert.equal(undo.disabled, true,
                "identity slant has no meaningful history");
            editor.reset();
            assert.equal(undo.disabled, true);
            assert.equal(redo.disabled, true);
            editor.start();
            assert.equal(undo.disabled, true,
                "a later session cannot recover previous ink");
        } finally {
            ui.restore();
        }
    });

test("one-step stroke undo restores empty drafts and directed edits",
    async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        const nfd = "n\u{303}";
        const undo = get("slant-undo");
        const redo = get("slant-redo");
        const save = async () => {
            get("save").dispatch("click");
            return downloads.at(-1).text();
        };
        editor.start();
        get("character").value = nfd;
        get("character").dispatch("input");
        for (const [axis, value] of Object.entries({
            x1: 400, y1: 600, x2: 1000, y2: 1400, width: 250,
        })) get(axis).value = String(value);
        get("add").dispatch("click");
        const original = [{
            grapheme: nfd, x1: 400, y1: 600, x2: 1000, y2: 1400,
            width: 250, color: "#243647",
        }];
        assert.equal(await save(), encodeGlyphLibrary(original));
        assert.equal(undo.textContent, "Undo last stroke addition");
        assert.equal(undo.disabled, false);
        undo.dispatch("click");
        assert.equal(get("preview").children.length, 0,
            "first authored segment can be undone to an empty draft");
        assert.equal(undo.disabled, true);
        assert.equal(redo.disabled, false);
        assert.equal(get("save").disabled, false,
            "empty draft remains available for guarded explicit save");
        const downloaded = downloads.length;
        get("save").dispatch("click");
        assert.match(get("status").textContent, /Save requires/);
        assert.equal(downloads.length, downloaded,
            "empty history state cannot export an invalid TSV");
        redo.dispatch("click");
        assert.equal(get("selected").value, "0");
        assert.equal(await save(), encodeGlyphLibrary(original));
        get("reverse").dispatch("click");
        const reversed = [{ ...original[0],
            x1: 1000, y1: 1400, x2: 400, y2: 600 }];
        assert.equal(await save(), encodeGlyphLibrary(reversed));
        assert.equal(undo.textContent, "Undo last stroke direction");
        undo.dispatch("click");
        assert.equal(get("selected").value, "0",
            "selected physical stroke remains selected across undo");
        assert.equal(get("x1").value, "400");
        assert.equal(await save(), encodeGlyphLibrary(original));
        redo.dispatch("click");
        assert.equal(await save(), encodeGlyphLibrary(reversed));
        get("x1").value = "1100";
        get("update").dispatch("click");
        const revised = [{ ...reversed[0], x1: 1100 }];
        assert.equal(undo.textContent, "Undo last stroke edit");
        assert.equal(await save(), encodeGlyphLibrary(revised));
        get("x2").value = "1100";
        get("y2").value = "1400";
        get("update").dispatch("click");
        assert.match(get("status").textContent, /endpoints must differ/);
        assert.equal(undo.disabled, false,
            "invalid proposal preserves last successful edit recovery");
        undo.dispatch("click");
        assert.equal(await save(), encodeGlyphLibrary(reversed));
        get("remove").dispatch("click");
        assert.equal(get("preview").children.length, 0);
        assert.equal(undo.textContent, "Undo last stroke removal");
        undo.dispatch("click");
        assert.equal(await save(), encodeGlyphLibrary(reversed));
        redo.dispatch("click");
        assert.equal(get("preview").children.length, 0);
        editor.reset();
        assert.equal(undo.disabled, true);
        assert.equal(redo.disabled, true);
    } finally {
        ui.restore();
    }
});

test("bulk handwriting fills undo as one atomic exact-library change",
    async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        const original = [stroke("ñ"), stroke("∞")];
        const before = encodeGlyphLibrary(original);
        const importInk = async () => {
            const raw = new TextEncoder().encode(before);
            get("load").files = [{ size: raw.length,
                arrayBuffer: async () => raw.buffer }];
            await get("load").dispatch("change");
            get("character").value = "ñ";
            get("character").dispatch("input");
        };
        const save = async () => {
            get("save").dispatch("click");
            return downloads.at(-1).text();
        };
        editor.start();
        await importInk();
        get("seed-lowercase").dispatch("click");
        assert.match(get("status").textContent, /Filled 26 missing/);
        assert.equal(get("slant-undo").textContent,
            "Undo last lowercase fill");
        const lowercaseBytes = await save();
        const lower = parseGlyphLibrary(lowercaseBytes);
        assert.equal(new Set(lower.map((item) => item.grapheme)).size, 28);
        assert.deepEqual(lower.slice(0, 2), original,
            "batch preserves custom input and exact Unicode source slots");
        get("character").value = "a";
        get("character").dispatch("input");
        assert.equal(get("slant-undo").disabled, true,
            "batch recovery is gated to original selected glyph");
        get("character").value = "ñ";
        get("character").dispatch("input");
        get("slant-undo").dispatch("click");
        assert.equal(await save(), before,
            "one undo removes all 26 new glyphs, not just one shape");
        assert.equal(get("slant-redo").disabled, false);
        get("seed-lowercase").dispatch("click");
        assert.equal(get("slant-redo").disabled, true,
            "a fresh batch replaces previous redo instead of stacking");
        assert.equal(await save(), lowercaseBytes);
        get("seed-punctuation").dispatch("click");
        assert.equal(get("slant-undo").textContent,
            "Undo last punctuation fill");
        const punctuatedBytes = await save();
        assert.notEqual(punctuatedBytes, lowercaseBytes);
        get("slant-undo").dispatch("click");
        assert.equal(await save(), lowercaseBytes,
            "later batch undo restores prior batch intact");
        get("slant-redo").dispatch("click");
        assert.equal(await save(), punctuatedBytes);
        await importInk();
        get("seed-baseline").dispatch("click");
        assert.match(get("status").textContent, /bilingual baseline/);
        assert.equal(get("slant-undo").textContent,
            "Undo last bilingual fill");
        const fullBaseline = await save();
        assert.ok(fullBaseline.length > before.length);
        get("slant-undo").dispatch("click");
        assert.equal(await save(), before,
            "large bilingual fill remains a single exact history step");
        get("slant-redo").dispatch("click");
        assert.equal(await save(), fullBaseline);
        editor.reset();
        assert.equal(get("slant-undo").disabled, true);
    } finally {
        ui.restore();
    }
});

test("starter replacement, exact copy and accent composition are undoable",
    async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        editor.start();
        const nfd = "e\u{301}";
        const line = (grapheme, x1, y1, x2, y2) => ({
            grapheme, x1, y1, x2, y2, width: 250,
            color: "#243647",
        });
        const importInk = async (ink) => {
            const source = new TextEncoder().encode(encodeGlyphLibrary(ink));
            get("load").files = [{ size: source.length,
                arrayBuffer: async () => source.buffer }];
            await get("load").dispatch("change");
        };
        const save = async () => {
            get("save").dispatch("click");
            return downloads.at(-1).text();
        };
        const recover = async (before, after, label) => {
            assert.equal(get("slant-undo").textContent, `Undo last ${label}`);
            assert.equal(await save(), encodeGlyphLibrary(after));
            get("slant-undo").dispatch("click");
            assert.equal(await save(), encodeGlyphLibrary(before),
                `${label} undo restores original authored ink`);
            get("slant-redo").dispatch("click");
            assert.equal(await save(), encodeGlyphLibrary(after),
                `${label} redo preserves exact Unicode slots`);
        };
        const original = [line("o", 700, 1300, 1200, 1900),
            line("ñ", 500, 2300, 1100, 2700)];
        await importInk(original);
        get("character").value = "o";
        get("character").dispatch("input");
        get("selected").value = "0";
        get("selected").dispatch("change");
        get("form").value = "o";
        get("form").dispatch("change");
        get("construct").dispatch("click");
        const starter = geometricStarter("o", 1550, 2450, 1100,
            250, "#243647");
        const replaced = [original[1], ...starter];
        await recover(original, replaced, "starter replacement");
        assert.equal(get("slant-undo").disabled, false);
        const accents = [line("e", 600, 1500, 1100, 2400),
            line("é", 1200, 900, 1500, 1200),
            line(nfd, 2000, 1500, 2300, 1800),
            line("∞", 2600, 2500, 3000, 2900)];
        await importInk(accents);
        get("character").value = "é";
        get("character").dispatch("input");
        get("copy-from").value = "e";
        get("copy").dispatch("click");
        const copied = accents.filter((item) => item.grapheme !== "é")
            .concat({ ...accents[0], grapheme: "é" });
        await recover(accents, copied, "glyph copy");
        get("character").value = nfd;
        get("character").dispatch("input");
        get("compose").dispatch("click");
        const composed = copied.filter((item) => item.grapheme !== nfd)
            .concat(composeAccentedGlyph(copied, "e", nfd, "#243647"));
        await recover(copied, composed, "accent composition");
        assert.deepEqual(parseGlyphLibrary(await save()), composed);
        get("character").value = "é";
        get("character").dispatch("input");
        assert.equal(get("slant-undo").disabled, true,
            "another exact spelling cannot undo the composed NFD glyph");
        get("character").value = nfd;
        get("character").dispatch("input");
        assert.equal(get("slant-undo").disabled, false);
        editor.reset();
        assert.equal(get("slant-undo").disabled, true);
    } finally {
        ui.restore();
    }
});

test("manual pen trace, path reorder and join recover in selected TSV",
    async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        const nfd = "n\u{303}";
        const line = (grapheme, x1, y1, x2, y2) => ({
            grapheme, x1, y1, x2, y2, width: 250,
            color: "#243647",
        });
        const a = line(nfd, 400, 500, 1000, 500);
        const b = line(nfd, 1000, 500, 1300, 900);
        const other = line("ñ", 800, 2000, 1100, 2200);
        const c = line(nfd, 1800, 1200, 2200, 1500);
        const source = [a, other, b, c];
        const save = async () => {
            get("save").dispatch("click");
            return downloads.at(-1).text();
        };
        const load = async (ink) => {
            const raw = new TextEncoder().encode(encodeGlyphLibrary(ink));
            get("load").files = [{ size: raw.length,
                arrayBuffer: async () => raw.buffer }];
            await get("load").dispatch("change");
            get("character").value = nfd;
            get("character").dispatch("input");
        };
        const select = (index) => {
            get("selected").value = String(index);
            get("selected").dispatch("change");
        };
        const roundtrip = async (accepted, original, label) => {
            assert.equal(await save(), encodeGlyphLibrary(accepted),
                `${label} accepted exact TSV`);
            assert.equal(get("slant-undo").disabled, false);
            get("slant-undo").dispatch("click");
            assert.equal(await save(), encodeGlyphLibrary(original),
                `${label} undo exact bytes`);
            get("slant-redo").dispatch("click");
            assert.equal(await save(), encodeGlyphLibrary(accepted),
                `${label} redo exact bytes`);
        };
        editor.start();
        await load(source);
        const preview = get("preview");
        const pointer = (x, y) => ({ clientX: x, clientY: y,
            pointerId: 19, isPrimary: true, button: 0 });
        preview.dispatch("pointerdown", pointer(230, 200));
        preview.dispatch("pointermove", pointer(240, 210));
        preview.dispatch("pointerup", pointer(250, 215));
        const traced = [...source,
            line(nfd, 2300, 2000, 2400, 2100),
            line(nfd, 2400, 2100, 2500, 2150)];
        assert.equal(get("slant-undo").textContent,
            "Undo last pen trace");
        await roundtrip(traced, source, "multi-segment trace");
        assert.equal(get("selected").value, "5",
            "redo restores selected final sampled pen segment");
        await load(source);
        select(0);
        get("path-later").dispatch("click");
        const moved = moveCandidatePath(source, nfd, 0, 1);
        assert.equal(get("slant-undo").textContent,
            "Undo last pen path order");
        await roundtrip(moved.strokes, source, "path reorder");
        assert.equal(get("selected").value,
            String(moved.selectedIndex));
        await load([a, other, c]);
        select(0);
        get("join").dispatch("click");
        const bridge = line(nfd, 1000, 500, 1800, 1200);
        const connected = [a, bridge, other, c];
        assert.equal(get("slant-undo").textContent,
            "Undo last pen connection");
        await roundtrip(connected, [a, other, c], "pen bridge");
        get("join").dispatch("click");
        assert.match(get("status").textContent, /already continuous/);
        assert.equal(get("slant-undo").disabled, false,
            "refused repeated join cannot discard accepted recovery");
        assert.equal(await save(), encodeGlyphLibrary(connected));
    } finally {
        ui.restore();
    }
});

test("one-step path undo preserves Unicode slots and imported source",
    async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        editor.start();
        const nfd = "o\u{301}";
        const line = (grapheme, x1, y1, x2, y2) => ({
            grapheme, x1, y1, x2, y2, width: 250,
            color: "#243647",
        });
        const source = [
            line(nfd, 400, 500, 1000, 500),
            line("ó", 500, 1200, 800, 1400),
            line(nfd, 1000, 500, 1000, 1300),
            line(nfd, 1000, 1300, 600, 1300),
        ];
        const bytes = encodeGlyphLibrary(source);
        const load = async (text) => {
            const raw = new TextEncoder().encode(text);
            get("load").files = [{
                size: raw.length, arrayBuffer: async () => raw.buffer,
            }];
            await get("load").dispatch("change");
        };
        const save = async () => {
            get("save").dispatch("click");
            return downloads.at(-1).text();
        };
        await load(bytes);
        get("character").value = nfd;
        get("character").dispatch("input");
        get("selected").value = "0";
        get("selected").dispatch("change");
        get("reverse-path").dispatch("click");
        const reversed = reverseCandidatePath(source, nfd, 0);
        assert.equal(await save(), encodeGlyphLibrary(reversed.strokes));
        assert.equal(get("slant-undo").textContent,
            "Undo last pen path direction");
        get("slant-undo").dispatch("click");
        assert.equal(get("selected").value, "0");
        assert.equal(await save(), bytes);
        get("slant-redo").dispatch("click");
        assert.equal(get("selected").value,
            String(reversed.selectedIndex));
        assert.equal(await save(), encodeGlyphLibrary(reversed.strokes));
        await load(bytes);
        assert.equal(get("slant-undo").disabled, true,
            "loading a selected source erases previous recovery");
        get("selected").value = "2";
        get("selected").dispatch("change");
        get("path-close").dispatch("click");
        const closed = closeCandidatePath(source, nfd, 2);
        assert.equal(await save(), encodeGlyphLibrary(closed.strokes));
        get("slant-undo").dispatch("click");
        assert.equal(await save(), bytes);
        get("slant-redo").dispatch("click");
        assert.equal(await save(), encodeGlyphLibrary(closed.strokes));
        get("character").value = "ó";
        get("character").dispatch("input");
        assert.equal(get("slant-undo").disabled, true,
            "a different exact Unicode key cannot undo the path");
        get("character").value = nfd;
        get("character").dispatch("input");
        assert.equal(get("slant-undo").disabled, false);
    } finally {
        ui.restore();
    }
});

test("slant history walks multiple steps and clears after import",
    async () => {
    const ui = mountEditor();
    try {
        const { get, editor, downloads } = ui;
        editor.start();
        get("character").value = "o";
        get("character").dispatch("input");
        for (const [key, value] of Object.entries({
            x1: 450, y1: 1000, x2: 850, y2: 1600, width: 250,
        })) get(key).value = String(value);
        get("add").dispatch("click");
        const original = [{
            grapheme: "o", x1: 450, y1: 1000,
            x2: 850, y2: 1600, width: 250, color: "#243647",
        }];
        const first = italicizeGlyphStrokes(original, "o", 12, 3550);
        const second = italicizeGlyphStrokes(first, "o", 12, 3550);
        get("slant-authored").dispatch("click");
        get("slant-authored").dispatch("click");
        get("slant-undo").dispatch("click");
        get("save").dispatch("click");
        assert.equal(await downloads.at(-1).text(),
            encodeGlyphLibrary(first), "one undo reverses one slant only");
        assert.equal(get("slant-undo").disabled, false);
        get("slant-undo").dispatch("click");
        get("save").dispatch("click");
        assert.equal(await downloads.at(-1).text(),
            encodeGlyphLibrary(original), "two undos recover both slants");
        get("slant-redo").dispatch("click");
        get("save").dispatch("click");
        assert.equal(await downloads.at(-1).text(),
            encodeGlyphLibrary(first), "first redo reapplies first slant");
        get("slant-redo").dispatch("click");
        get("save").dispatch("click");
        assert.equal(await downloads.at(-1).text(),
            encodeGlyphLibrary(second), "second redo reapplies latest slant");
        // A successful load invalidates history even for identical bytes.
        const bytes = encodeGlyphLibrary(second);
        get("load").files = [{
            size: bytes.length,
            arrayBuffer: async () => new TextEncoder().encode(bytes).buffer,
        }];
        await get("load").dispatch("change");
        assert.equal(get("slant-undo").disabled, true);
        assert.equal(get("slant-redo").disabled, true);
        // A different in-place mutation cannot be destroyed by stale undo.
        get("authored-angle").value = "0";
        get("slant-authored").dispatch("click");
        assert.equal(get("slant-undo").disabled, true);
        get("authored-angle").value = "12";
        get("slant-authored").dispatch("click");
        assert.equal(get("slant-undo").disabled, false);
        get("selected").value = "0";
        get("selected").dispatch("change");
        get("remove").dispatch("click");
        assert.equal(get("slant-undo").disabled, false);
        assert.equal(get("slant-undo").textContent,
            "Undo last stroke removal");
        assert.equal(get("slant-redo").disabled, true);
        get("slant-undo").dispatch("click");
        assert.equal(get("selected").value, "0",
            "manual undo restores the previous selected stroke");
        get("save").dispatch("click");
        assert.equal(await downloads.at(-1).text(),
            encodeGlyphLibrary(italicizeGlyphStrokes(second, "o", 12,
                3550)), "undo restores the removed glyph exactly");
        assert.equal(get("slant-redo").disabled, false);
        get("slant-redo").dispatch("click");
        assert.equal(get("slant-undo").disabled, false,
            "even an empty library can undo the last deletion");
    } finally {
        ui.restore();
    }
});

test("eight-step history evicts only oldest edits and truncates redo tail",
    async () => {
        const ui = mountEditor();
        try {
            const { editor, get, downloads } = ui;
            const nfd = "n\u0303";
            const undo = get("slant-undo");
            const redo = get("slant-redo");
            const visit = (glyph) => {
                get("character").value = glyph;
                get("character").dispatch("input");
            };
            const append = (glyph, i) => {
                visit(glyph);
                for (const [key, value] of Object.entries({
                    x1: 350 + i * 25, y1: 1000,
                    x2: 650 + i * 25, y2: 1400, width: 250,
                })) get(key).value = String(value);
                get("add").dispatch("click");
                return {
                    grapheme: glyph, x1: 350 + i * 25, y1: 1000,
                    x2: 650 + i * 25, y2: 1400, width: 250,
                    color: "#243647",
                };
            };
            const saved = async () => {
                get("save").dispatch("click");
                return parseGlyphLibrary(await downloads.at(-1).text());
            };
            editor.start();
            const all = Array.from({ length: 10 }, (_, i) => append(nfd, i));
            assert.deepEqual(await saved(), all);
            for (let i = 0; i < 8; i += 1) {
                assert.equal(undo.disabled, false);
                undo.dispatch("click");
            }
            assert.equal(undo.disabled, true,
                "oldest two edits are now the unrecoverable base state");
            assert.deepEqual(await saved(), all.slice(0, 2));
            for (let i = 0; i < 8; i += 1) {
                assert.equal(redo.disabled, false);
                redo.dispatch("click");
            }
            assert.equal(redo.disabled, true);
            assert.deepEqual(await saved(), all);

            undo.dispatch("click");
            undo.dispatch("click");
            assert.deepEqual(await saved(), all.slice(0, 8));
            get("authored-angle").value = "21";
            get("slant-authored").dispatch("click");
            assert.match(get("status").textContent, /Italic angle/);
            assert.equal(redo.disabled, false,
                "refused edit cannot truncate a valid redo branch");
            get("authored-angle").value = "0";
            get("slant-authored").dispatch("click");
            assert.equal(redo.disabled, false,
                "identity edit cannot truncate a valid redo branch");

            const branched = append("ñ", 15);
            assert.equal(redo.disabled, true);
            visit(nfd);
            assert.equal(redo.disabled, true,
                "discarded future cannot return on its old Unicode key");
            assert.equal(undo.disabled, true,
                "history currently belongs to the new exact glyph");
            visit("ñ");
            undo.dispatch("click");
            assert.deepEqual(await saved(), all.slice(0, 8));
            assert.equal(redo.disabled, false);
            redo.dispatch("click");
            assert.deepEqual(await saved(), [...all.slice(0, 8), branched]);
            editor.reset();
            assert.equal(undo.disabled, true);
            editor.start();
            visit("ñ");
            assert.equal(redo.disabled, true);
        } finally {
            ui.restore();
        }
    });

test("multi-step history replays Unicode-specific ink and bulk fill exactly",
    async () => {
        const ui = mountEditor();
        try {
            const { editor, get, downloads } = ui;
            const nfd = "a\u0301";
            const undo = get("slant-undo");
            const redo = get("slant-redo");
            const select = (glyph) => {
                get("character").value = glyph;
                get("character").dispatch("input");
            };
            const append = (glyph, offset) => {
                select(glyph);
                for (const [key, value] of Object.entries({
                    x1: 500 + offset, y1: 1200,
                    x2: 850 + offset, y2: 1600, width: 250,
                })) get(key).value = String(value);
                get("add").dispatch("click");
                return { grapheme: glyph, x1: 500 + offset, y1: 1200,
                    x2: 850 + offset, y2: 1600, width: 250,
                    color: "#243647" };
            };
            const saved = async () => {
                get("save").dispatch("click");
                return await downloads.at(-1).text();
            };
            editor.start();
            const first = append(nfd, 0);
            const second = append("á", 50);
            const beforeFill = encodeGlyphLibrary([first, second]);
            get("seed-digits").dispatch("click");
            const withFill = await saved();
            assert.ok(parseGlyphLibrary(withFill).length > 20);
            assert.equal(undo.textContent, "Undo last digit fill");
            undo.dispatch("click");
            assert.equal(await saved(), beforeFill,
                "undo of one transaction clears all ten added numeral keys");
            assert.equal(undo.textContent, "Undo last stroke addition");
            undo.dispatch("click");
            assert.equal(await saved(), encodeGlyphLibrary([first]));
            select("á");
            assert.equal(undo.disabled, true,
                "NFC is not the previous NFD history owner");
            select(nfd);
            assert.equal(undo.disabled, false);
            undo.dispatch("click");
            assert.equal(get("preview").children.length, 0);
            get("save").dispatch("click");
            assert.match(get("status").textContent, /Save requires 1\.\.4096/,
                "empty in-memory undo state cannot download invalid TSV");
            redo.dispatch("click");
            assert.equal(await saved(), encodeGlyphLibrary([first]));
            select("á");
            redo.dispatch("click");
            assert.equal(await saved(), beforeFill);
            redo.dispatch("click");
            assert.equal(await saved(), withFill);
            assert.equal(redo.disabled, true);
            editor.reset();
        } finally {
            ui.restore();
        }
    });

test("history navigation finds exact NFC/NFD undo and redo owners",
    async () => {
        const ui = mountEditor();
        try {
            const { editor, get, downloads } = ui;
            const nfd = "a\u0301";
            const goUndo = get("recovery-undo-select");
            const goRedo = get("recovery-redo-select");
            const undo = get("slant-undo");
            const redo = get("slant-redo");
            const visit = (glyph) => {
                get("character").value = glyph;
                get("character").dispatch("input");
            };
            const add = (glyph, offset) => {
                visit(glyph);
                for (const [key, value] of Object.entries({
                    x1: 400 + offset, y1: 1200, x2: 800 + offset,
                    y2: 1600, width: 250,
                })) get(key).value = String(value);
                get("add").dispatch("click");
                return { grapheme: glyph, x1: 400 + offset,
                    y1: 1200, x2: 800 + offset, y2: 1600,
                    width: 250, color: "#243647" };
            };
            const save = async () => {
                get("save").dispatch("click");
                return await downloads.at(-1).text();
            };
            assert.equal(goUndo.disabled, true);
            assert.equal(goRedo.disabled, true);
            editor.start();
            const first = add(nfd, 0);
            const second = add("á", 100);
            const third = add("∞", 200);
            const original = encodeGlyphLibrary([first, second, third]);
            assert.equal(get("recovery-status").textContent.includes(
                "3/3 applied"), true);
            assert.equal(goUndo.disabled, true,
                "current Unicode glyph already owns next undo");
            assert.equal(goRedo.disabled, true);
            undo.dispatch("click");
            assert.equal(goUndo.disabled, false);
            assert.equal(goUndo.textContent, "Select á to undo");
            assert.equal(undo.disabled, true,
                "history still requires explicit exact-owner selection");
            const afterThirdUndo = await save();
            goUndo.dispatch("click");
            assert.equal(get("character").value, "á");
            assert.match(get("status").textContent,
                /Selected á for undo; draft unchanged/);
            assert.equal(await save(), afterThirdUndo,
                "navigating history cannot rewrite authored ink");
            assert.equal(goUndo.disabled, true);
            assert.equal(undo.disabled, false);
            undo.dispatch("click");
            assert.equal(goUndo.textContent, "Select á (NFD) to undo");
            goUndo.dispatch("click");
            assert.equal(get("character").value, nfd,
                "navigation retains combining acute instead of NFC");
            assert.equal(undo.disabled, false);
            assert.equal(await save(), encodeGlyphLibrary([first]));
            undo.dispatch("click");
            assert.equal(goUndo.disabled, true);
            assert.equal(goRedo.disabled, true,
                "same exact glyph is already selected for redo");
            assert.match(get("recovery-status").textContent,
                /0\/3 applied.*Redo: á \(NFD\)/);
            redo.dispatch("click");
            assert.equal(goRedo.textContent, "Select á to redo");
            assert.equal(goRedo.disabled, false);
            goRedo.dispatch("click");
            assert.equal(get("character").value, "á");
            assert.equal(await save(), encodeGlyphLibrary([first]));
            redo.dispatch("click");
            assert.equal(goRedo.textContent, "Select ∞ to redo");
            goRedo.dispatch("click");
            assert.equal(get("character").value, "∞");
            redo.dispatch("click");
            assert.equal(await save(), original);
            assert.equal(get("recovery-status").textContent.includes(
                "3/3 applied"), true);
            assert.equal(goRedo.disabled, true);
            assert.equal(goUndo.disabled, true);
            editor.reset();
            assert.equal(goUndo.disabled, true);
            assert.equal(goRedo.disabled, true);
            assert.equal(get("recovery-status").textContent,
                "No active glyph editing session.");
        } finally {
            ui.restore();
        }
    });

test("scoped history shortcuts preserve native input undo and exact glyphs",
    async () => {
        const ui = mountEditor();
        try {
            const { editor, get, downloads } = ui;
            const nfd = "a\u0301";
            const visit = (key) => {
                get("character").value = key;
                get("character").dispatch("input");
            };
            const keypress = (key, props = {}) => {
                let prevented = false;
                get("editor-region").dispatch("keydown", {
                    key, ctrlKey: true, metaKey: false, shiftKey: false,
                    repeat: false, isComposing: false, altKey: false,
                    defaultPrevented: false,
                    target: { closest: () => null },
                    preventDefault: () => { prevented = true; },
                    ...props,
                });
                return prevented;
            };
            const saved = async () => {
                get("save").dispatch("click");
                return await downloads.at(-1).text();
            };
            assert.equal(keypress("z"), false, "inactive editor");
            editor.start();
            visit(nfd);
            get("add").dispatch("click");
            const first = await saved();
            visit("á");
            get("add").dispatch("click");
            const second = await saved();
            assert.notEqual(second, first);
            // Browser input history always wins over app glyph history.
            assert.equal(keypress("z", {
                target: { closest: () => ({ tagName: "INPUT" }) },
            }), false);
            assert.equal(keypress("z", { altKey: true }), false);
            assert.equal(keypress("z", { repeat: true }), false);
            assert.equal(keypress("z", { isComposing: true }), false);
            assert.equal(keypress("z", { defaultPrevented: true }), false);
            assert.equal(keypress("z", { metaKey: true }), false,
                "two simultaneous platform modifiers are ignored");
            assert.equal(keypress("y", { shiftKey: true }), false);
            assert.equal(keypress("Z"), true, "uppercase keyboard key works");
            assert.equal(await saved(), first);
            assert.equal(keypress("z", { shiftKey: true }), true,
                "Ctrl+Shift+Z redoes the exact selected NFC key");
            assert.equal(await saved(), second);
            assert.equal(keypress("z", { metaKey: true, ctrlKey: false }),
                true, "Mac Command+Z uses the same recovery path");
            assert.equal(await saved(), first);
            assert.equal(keypress("y"), true, "Ctrl+Y redoes");
            assert.equal(await saved(), second);
            assert.equal(keypress("z"), true);
            visit(nfd);
            assert.equal(keypress("y"), false,
                "next redo belongs to NFC, not matching visible NFD");
            assert.equal(keypress("z"), true, "NFD has its own history step");
            assert.equal(get("preview").children.length, 0);
            assert.equal(keypress("z"), false, "no more undo history");
            assert.equal(keypress("z", { shiftKey: true }), true,
                "NFD restored by the same shortcut");
            assert.equal(await saved(), first);
            assert.equal(keypress("z", { metaKey: true, ctrlKey: false,
                shiftKey: true }), false,
                "Mac redo cannot skip the current NFC owner");
            get("recovery-redo-select").dispatch("click");
            assert.equal(get("character").value, "á");
            assert.equal(keypress("z", { metaKey: true, ctrlKey: false,
                shiftKey: true }), true);
            assert.equal(await saved(), second);
            editor.reset();
            assert.equal(keypress("z"), false);
            assert.equal(keypress("y"), false);
        } finally {
            ui.restore();
        }
    });

test("read-only authored slant guide predicts exact accepted pen coordinates",
    async () => {
        const ui = mountEditor();
        try {
            const { get, editor, downloads } = ui;
            const nfd = "a\u0301";
            const guide = get("slant-preview");
            const guideStatus = get("slant-preview-status");
            assert.equal(guide.children.length, 0);
            editor.start();
            assert.match(guideStatus.textContent, /Author this exact glyph/);
            get("character").value = nfd;
            get("character").dispatch("input");
            for (const [key, value] of Object.entries({
                x1: 450, y1: 1000, x2: 850, y2: 1600, width: 250,
            })) get(key).value = String(value);
            get("add").dispatch("click");
            const authored = [{
                grapheme: nfd, x1: 450, y1: 1000,
                x2: 850, y2: 1600, width: 250, color: "#243647",
            }];
            assert.equal(guide.children.length, 1);
            assert.match(guideStatus.textContent,
                /preview only, draft unchanged/);
            const coord = (key) => Number(guide.children[0]
                .attributes.get(key));
            const predicted = italicizeGlyphStrokes(
                authored, nfd, 12, 3550)[0];
            assert.equal(coord("x1"), predicted.x1);
            assert.equal(coord("x2"), predicted.x2);
            assert.equal(guide.children[0].attributes.get("stroke"),
                "#243647");
            assert.equal(downloads.length, 0,
                "guide must never download or mutate authored ink");
            get("authored-angle").value = "0";
            get("authored-angle").dispatch("input");
            assert.equal(coord("x1"), authored[0].x1);
            get("authored-baseline").value = "4501";
            get("authored-baseline").dispatch("input");
            assert.equal(guide.children.length, 0);
            assert.match(guideStatus.textContent, /italic baseline/);
            get("authored-baseline").value = "3550";
            get("authored-baseline").dispatch("input");
            assert.equal(guide.children.length, 1);
            get("authored-angle").value = "20";
            get("authored-angle").dispatch("input");
            assert.equal(guide.children.length, 1);
            get("authored-angle").value = "21";
            get("authored-angle").dispatch("input");
            assert.equal(guide.children.length, 0);
            assert.match(guideStatus.textContent, /Italic angle/);
            get("save").dispatch("click");
            assert.equal(await downloads.at(-1).text(),
                encodeGlyphLibrary(authored),
                "invalid preview cannot overwrite the existing draft");
            get("authored-angle").value = "12";
            get("authored-angle").dispatch("input");
            const before = [...guide.children[0].attributes];
            get("slant-authored").dispatch("click");
            assert.equal(get("preview").children.length, 1);
            assert.deepEqual([
                ...get("preview").children[0].attributes
            ].filter(([name]) => ["x1", "x2", "y1", "y2",
                "stroke", "stroke-width"].includes(name)),
            before.filter(([name]) => ["x1", "x2", "y1", "y2",
                "stroke", "stroke-width"].includes(name)));
            editor.reset();
            assert.equal(guide.children.length, 0);
            assert.match(guideStatus.textContent,
                /Requires a compatible backend session/);
        } finally {
            ui.restore();
        }
    });

test("slant preview refuses clipping and its display budget",
    async () => {
    const ui = mountEditor();
    try {
        const { get, editor, downloads } = ui;
        editor.start();
        get("character").value = "o";
        get("character").dispatch("input");
        const load = async (library) => {
            const bytes = encodeGlyphLibrary(library);
            get("load").files = [{
                size: bytes.length,
                arrayBuffer: async () =>
                    new TextEncoder().encode(bytes).buffer,
            }];
            await get("load").dispatch("change");
            return bytes;
        };
        const clipped = [{
            grapheme: "o", x1: 3100, y1: 300,
            x2: 3200, y2: 900, width: 250, color: "#243647",
        }];
        const original = await load(clipped);
        get("authored-angle").value = "20";
        get("authored-angle").dispatch("input");
        assert.equal(get("slant-preview").children.length, 0);
        assert.match(get("slant-preview-status").textContent,
            /Italic ink and pen clip/);
        assert.equal(get("slant-authored").disabled, false,
            "refused candidate still uses the action's bounds checks");
        get("slant-authored").dispatch("click");
        assert.match(get("status").textContent, /Italic ink and pen clip/);
        get("save").dispatch("click");
        assert.equal(await downloads.at(-1).text(), original);
        const full = Array.from({ length: 129 }, (_, index) => ({
            grapheme: "o", x1: 450 + index,
            y1: 1000, x2: 850 + index,
            y2: 1600, width: 250, color: "#243647",
        }));
        await load(full);
        assert.equal(get("slant-preview").children.length, 0);
        assert.match(get("slant-preview-status").textContent,
            /Preview limited to 128 authored segments/);
        get("authored-angle").value = "12";
        get("slant-authored").dispatch("click");
        get("save").dispatch("click");
        assert.equal(await downloads.at(-1).text(), encodeGlyphLibrary(
            italicizeGlyphStrokes(full, "o", 12, 3550)),
        "read-only preview limit does not lower native edit capacity");
    } finally {
        ui.restore();
    }
});

test("height scaling preserves exact Unicode rows and pen continuity", () => {
    const nfd = "a\u0301";
    const rows = [
        { grapheme: nfd, x1: 450, y1: 1000, x2: 850, y2: 1600,
            width: 250, color: "#243647" },
        { grapheme: "á", x1: 600, y1: 1200, x2: 900, y2: 1550,
            width: 250, color: "#123456" },
        { grapheme: nfd, x1: 850, y1: 1600, x2: 1300, y2: 2200,
            width: 250, color: "#243647" },
        { grapheme: "∞", x1: 500, y1: 900, x2: 700, y2: 1100,
            width: 250, color: "#112233" },
        { grapheme: nfd, x1: 1100, y1: 600, x2: 1700, y2: 1000,
            width: 250, color: "#334455" },
    ];
    const original = encodeGlyphLibrary(rows);
    const resized = scaleGlyphHeight(rows, nfd, 110, 2250);
    assert.deepEqual(resized, rows.map((line) => line.grapheme !== nfd
        ? line : { ...line,
            y1: Math.round(2250 + (line.y1 - 2250) * 1.1),
            y2: Math.round(2250 + (line.y2 - 2250) * 1.1),
        }));
    assert.equal(resized[1], rows[1]);
    assert.equal(resized[3], rows[3]);
    assert.deepEqual(candidatePenPaths(resized, nfd).map((path) =>
        [path.first, path.last]), [[1, 2], [3, 3]]);
    assert.equal(encodeGlyphLibrary(rows), original);
    assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(resized)), resized);
    assert.deepEqual(scaleGlyphHeight(rows, nfd, 100, 2250), rows);
    // Half-integer values round toward positive infinity in both engines.
    const half = [{ ...rows[0], y1: 2001, y2: 2003 }];
    assert.equal(scaleGlyphHeight(half, nfd, 50, 2000)[0].y1, 2001);
    for (const percent of [49, 151, 110.5, NaN, Infinity]) {
        assert.throws(() => scaleGlyphHeight(rows, nfd,
            percent, 2250), /Height percent/);
    }
    for (const baseline of [-1, 4501, 0.5, NaN]) {
        assert.throws(() => scaleGlyphHeight(rows, nfd,
            110, baseline), /glyph height baseline/);
    }
    assert.throws(() => scaleGlyphHeight(rows, "ab", 110,
        2250), /exact visible/);
    assert.throws(() => scaleGlyphHeight(rows, "ñ", 110,
        2250), /no authored ink/);
    const clipped = [{ ...rows[0], y1: 200, y2: 600 }];
    assert.throws(() => scaleGlyphHeight(clipped, nfd,
        150, 2250), /Resized ink and pen clip/);
    assert.equal(clipped[0].y1, 200);
    const badX = [{ ...rows[0], x1: 125, x2: 850, width: 251 }];
    assert.throws(() => scaleGlyphHeight(badX, nfd,
        100, 2250), /Resized ink and pen clip/);
    const negativeHalf = [{ ...rows[0], y1: 1999, y2: 2001 }];
    assert.equal(scaleGlyphHeight(negativeHalf, nfd, 50, 2000)[0].y1,
        2000, "negative half rounds toward positive infinity");
    const collapsed = [{ ...rows[0], x1: 450, x2: 450,
        y1: 2001, y2: 2002 }];
    assert.throws(() => scaleGlyphHeight(collapsed, nfd,
        50, 2000), /collapses a pen segment/);
    const full = Array.from({ length: 4096 }, (_, index) => ({
        ...rows[0], x1: 450 + index % 100,
        x2: 850 + index % 100,
    }));
    assert.equal(scaleGlyphHeight(full, nfd, 110, 2250).length, 4096);
});

test("height preview is exact and invalid edits keep the unsaved library",
    async () => {
    const ui = mountEditor();
    try {
        const { get, editor, downloads } = ui;
        const nfd = "a\u0301";
        const guide = get("height-preview");
        const status = get("height-preview-status");
        assert.equal(get("scale-height").disabled, true);
        editor.start();
        assert.equal(guide.children.length, 0);
        get("character").value = nfd;
        get("character").dispatch("input");
        for (const [key, value] of Object.entries({
            x1: 450, y1: 1000, x2: 850, y2: 1600, width: 250,
        })) get(key).value = String(value);
        get("add").dispatch("click");
        const authored = [{ grapheme: nfd,
            x1: 450, y1: 1000, x2: 850, y2: 1600,
            width: 250, color: "#243647" }];
        const proposed = scaleGlyphHeight(authored, nfd, 110, 2250)[0];
        assert.equal(get("scale-height").disabled, false);
        assert.equal(guide.children.length, 1);
        assert.equal(Number(guide.children[0].attributes.get("y1")),
            proposed.y1);
        assert.equal(Number(guide.children[0].attributes.get("y2")),
            proposed.y2);
        assert.match(status.textContent, /preview only, draft unchanged/);
        assert.equal(downloads.length, 0);
        get("height-percent").value = "151";
        get("height-percent").dispatch("input");
        assert.equal(guide.children.length, 0);
        assert.match(status.textContent, /Height percent/);
        get("scale-height").dispatch("click");
        assert.match(get("status").textContent, /Height percent/);
        get("height-percent").value = "110";
        get("height-percent").dispatch("input");
        get("height-baseline").value = "";
        get("height-baseline").dispatch("input");
        assert.equal(guide.children.length, 0);
        assert.match(status.textContent, /Choose a height/);
        get("height-baseline").value = "2250";
        get("height-baseline").dispatch("input");
        assert.equal(guide.children.length, 1);
        get("scale-height").dispatch("click");
        assert.match(get("status").textContent,
            /Resized all authored á height; unsaved/);
        get("save").dispatch("click");
        assert.equal(await downloads.at(-1).text(),
            encodeGlyphLibrary([proposed]));
        get("slant-authored").dispatch("click");
        assert.equal(get("slant-undo").disabled, false);
        get("scale-height").dispatch("click");
        assert.equal(get("slant-undo").disabled, false,
            "height edit offers a new byte-exact recovery step");
        get("slant-undo").dispatch("click");
        assert.match(get("status").textContent,
            /Undid last glyph height; unsaved/);
        assert.equal(get("slant-redo").disabled, false);
        get("slant-redo").dispatch("click");
        assert.match(get("status").textContent,
            /Redid last glyph height; unsaved/);
        get("character").value = "á";
        get("character").dispatch("input");
        assert.equal(get("scale-height").disabled, true);
        editor.reset();
        assert.equal(guide.children.length, 0);
        assert.match(status.textContent,
            /Requires a compatible backend session/);
        assert.equal(get("height-percent").disabled, true);
    } finally {
        ui.restore();
    }
});

test("whole-glyph width rescales exact Unicode and rejects clipped pen", () => {
    const nfd = "o\u0301";
    const source = [
        { grapheme: nfd, x1: 450, y1: 1000, x2: 850,
            y2: 1600, width: 250, color: "#243647" },
        { grapheme: "ó", x1: 300, y1: 1000, x2: 500,
            y2: 1500, width: 250, color: "#112233" },
        { grapheme: nfd, x1: 850, y1: 1600, x2: 1300,
            y2: 2200, width: 250, color: "#243647" },
    ];
    const before = encodeGlyphLibrary(source);
    const resized = scaleGlyphWidth(source, nfd, 110, 1750);
    assert.deepEqual(resized, source.map((line) => line.grapheme !== nfd
        ? line : { ...line,
            x1: Math.round(1750 + (line.x1 - 1750) * 1.1),
            x2: Math.round(1750 + (line.x2 - 1750) * 1.1),
        }));
    assert.equal(resized[1], source[1]);
    assert.deepEqual(candidatePenPaths(resized, nfd).map((path) =>
        [path.first, path.last]), [[1, 2]]);
    assert.equal(encodeGlyphLibrary(source), before);
    assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(resized)),
        resized);
    assert.deepEqual(scaleGlyphWidth(source, nfd, 100, 1750), source);
    const tie = [{ ...source[0], x1: 1749, x2: 1751 }];
    assert.equal(scaleGlyphWidth(tie, nfd, 50, 1750)[0].x1,
        1750, "negative half rounds upward");
    for (const percent of [49, 151, 110.5, NaN]) {
        assert.throws(() => scaleGlyphWidth(source, nfd,
            percent, 1750), /Width percent/);
    }
    for (const anchor of [-1, 3501, 1.5, NaN]) {
        assert.throws(() => scaleGlyphWidth(source, nfd,
            110, anchor), /glyph width anchor/);
    }
    assert.throws(() => scaleGlyphWidth(source, "ab", 110,
        1750), /exact visible/);
    assert.throws(() => scaleGlyphWidth(source, "á", 110,
        1750), /no authored ink/);
    const clipped = [{ ...source[0], x1: 200, x2: 600 }];
    assert.throws(() => scaleGlyphWidth(clipped, nfd,
        150, 1750), /Resized ink and pen clip/);
    const badY = [{ ...source[0], y1: 125, width: 251 }];
    assert.throws(() => scaleGlyphWidth(badY, nfd,
        100, 1750), /Resized ink and pen clip/);
    const collapsed = [{ ...source[0], y1: 2000, y2: 2000,
        x1: 1749, x2: 1750 }];
    assert.throws(() => scaleGlyphWidth(collapsed, nfd,
        50, 1750), /collapses a pen segment/);
    assert.equal(encodeGlyphLibrary(source), before);
    const full = Array.from({ length: 4096 }, (_, i) => ({
        ...source[0], x1: 450 + i % 100, x2: 850 + i % 100,
    }));
    assert.equal(scaleGlyphWidth(full, nfd, 110, 1750).length, 4096);
});

test("width guide is read-only and accepted TSV is explicit", async () => {
    const ui = mountEditor();
    try {
        const { get, editor, downloads } = ui;
        const nfd = "o\u0301";
        editor.start();
        get("character").value = nfd;
        get("character").dispatch("input");
        assert.equal(get("scale-width").disabled, true);
        for (const [key, value] of Object.entries({
            x1: 450, y1: 1000, x2: 850, y2: 1600, width: 250,
        })) get(key).value = String(value);
        get("add").dispatch("click");
        const source = [{ grapheme: nfd,
            x1: 450, y1: 1000, x2: 850, y2: 1600,
            width: 250, color: "#243647" }];
        const proposed = scaleGlyphWidth(source, nfd, 110, 1750)[0];
        const guide = get("width-preview");
        assert.equal(get("scale-width").disabled, false);
        assert.equal(guide.children.length, 1);
        assert.equal(Number(guide.children[0].attributes.get("x1")),
            proposed.x1);
        assert.equal(downloads.length, 0);
        get("width-percent").value = "151";
        get("width-percent").dispatch("input");
        assert.equal(guide.children.length, 0);
        get("scale-width").dispatch("click");
        assert.match(get("status").textContent, /Width percent/);
        get("width-percent").value = "110";
        get("width-percent").dispatch("input");
        assert.equal(guide.children.length, 1);
        get("scale-width").dispatch("click");
        assert.match(get("status").textContent,
            /Resized all authored ó width; unsaved/);
        get("save").dispatch("click");
        assert.equal(await downloads.at(-1).text(),
            encodeGlyphLibrary([proposed]));
        editor.reset();
        assert.equal(guide.children.length, 0);
        assert.equal(get("scale-width").disabled, true);
    } finally {
        ui.restore();
    }
});

test("ink thickness changes only the exact glyph pen", () => {
    const nfd = "o\u0301";
    const source = [
        { grapheme: nfd, x1: 500, y1: 800, x2: 850,
            y2: 1200, width: 251, color: "#243647" },
        { grapheme: "ó", x1: 300, y1: 1000, x2: 500,
            y2: 1500, width: 250, color: "#112233" },
        { grapheme: nfd, x1: 850, y1: 1200, x2: 1100,
            y2: 1700, width: 251, color: "#243647" },
    ];
    const before = encodeGlyphLibrary(source);
    const thicker = scaleGlyphInk(source, nfd, 110);
    assert.deepEqual(thicker, source.map((line) => line.grapheme === nfd
        ? { ...line, width: 276 } : line));
    assert.equal(thicker[1], source[1]);
    assert.deepEqual(candidatePenPaths(thicker, nfd).map((path) =>
        [path.first, path.last]), [[1, 2]]);
    assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(thicker)),
        thicker);
    assert.deepEqual(scaleGlyphInk(source, nfd, 100), source);
    assert.equal(scaleGlyphInk([source[0]], nfd, 50)[0].width, 126,
        "half-up ink thickness rounding matches native Rust");
    for (const percent of [49, 151, 110.5, NaN]) {
        assert.throws(() => scaleGlyphInk(source, nfd, percent),
            /Ink percent/);
    }
    assert.throws(() => scaleGlyphInk(source, "ab", 110),
        /exact visible/);
    assert.throws(() => scaleGlyphInk(source, "ñ", 110),
        /no authored ink/);
    assert.throws(() => scaleGlyphInk([
        { ...source[0], width: 80 },
    ], nfd, 50), /Scaled pen width/);
    assert.throws(() => scaleGlyphInk([
        { ...source[0], width: 1500 },
    ], nfd, 150), /Scaled pen width/);
    assert.throws(() => scaleGlyphInk([
        { ...source[0], x1: 125 },
    ], nfd, 110), /Scaled ink and pen clip/);
    assert.throws(() => scaleGlyphInk([
        { ...source[0], y2: 4400 },
    ], nfd, 110), /Scaled ink and pen clip/);
    assert.equal(encodeGlyphLibrary(source), before,
        "every refusal preserves the original bytes");
    const full = Array.from({ length: 4096 }, (_, i) => ({
        ...source[0], x1: 500 + i % 100, x2: 850 + i % 100,
    }));
    assert.equal(scaleGlyphInk(full, nfd, 110).length, 4096);
});

test("ink preview stays read-only until explicit safe draft edit", async () => {
    const ui = mountEditor();
    try {
        const { get, editor, downloads } = ui;
        const nfd = "o\u0301";
        editor.start();
        get("character").value = nfd;
        get("character").dispatch("input");
        assert.equal(get("scale-ink").disabled, true);
        for (const [key, value] of Object.entries({
            x1: 500, y1: 800, x2: 850, y2: 1200, width: 251,
        })) get(key).value = String(value);
        get("add").dispatch("click");
        const original = [{ grapheme: nfd, x1: 500, y1: 800,
            x2: 850, y2: 1200, width: 251, color: "#243647" }];
        assert.equal(get("scale-ink").disabled, false);
        assert.equal(get("ink-preview").children.length, 1);
        assert.equal(Number(get("ink-preview").children[0]
            .attributes.get("stroke-width")), 276);
        assert.equal(downloads.length, 0);
        get("ink-percent").value = "151";
        get("ink-percent").dispatch("input");
        assert.equal(get("ink-preview").children.length, 0);
        get("scale-ink").dispatch("click");
        assert.match(get("status").textContent, /Ink percent/);
        get("save").dispatch("click");
        assert.equal(await downloads.at(-1).text(),
            encodeGlyphLibrary(original));
        get("ink-percent").value = "110";
        get("ink-percent").dispatch("input");
        get("scale-ink").dispatch("click");
        assert.match(get("status").textContent,
            /Resized all authored ó pen thickness; unsaved/);
        get("save").dispatch("click");
        assert.equal(await downloads.at(-1).text(),
            encodeGlyphLibrary(scaleGlyphInk(original, nfd, 110)));
        editor.reset();
        assert.equal(get("ink-preview").children.length, 0);
        assert.equal(get("scale-ink").disabled, true);
    } finally {
        ui.restore();
    }
});


test("exact one-step transform undo restores slant height width and ink",
    async () => {
        const ui = mountEditor();
        try {
            const { editor, get, downloads } = ui;
            const nfd = "o\u0301";
            const undo = get("slant-undo");
            const redo = get("slant-redo");
            const add = (grapheme, coordinates, width) => {
                get("character").value = grapheme;
                get("character").dispatch("input");
                for (const [key, value] of Object.entries({
                    x1: coordinates[0], y1: coordinates[1],
                    x2: coordinates[2], y2: coordinates[3], width,
                })) get(key).value = String(value);
                get("add").dispatch("click");
            };
            const recoverBytes = async (button, expected, label) => {
                button.dispatch("click");
                get("save").dispatch("click");
                assert.equal(await downloads.at(-1).text(),
                    encodeGlyphLibrary(expected), label);
            };
            editor.start();
            assert.equal(undo.textContent, "Undo last glyph transform");
            add(nfd, [500, 800, 850, 1200], 251);
            add("ó", [300, 1000, 500, 1500], 250);
            add(nfd, [850, 1200, 1100, 1700], 251);
            add("∞", [1500, 2000, 1600, 2200], 251);
            get("character").value = nfd;
            get("character").dispatch("input");
            const source = [
                { grapheme: nfd, x1: 500, y1: 800, x2: 850,
                    y2: 1200, width: 251, color: "#243647" },
                { grapheme: "ó", x1: 300, y1: 1000, x2: 500,
                    y2: 1500, width: 250, color: "#243647" },
                { grapheme: nfd, x1: 850, y1: 1200, x2: 1100,
                    y2: 1700, width: 251, color: "#243647" },
                { grapheme: "∞", x1: 1500, y1: 2000, x2: 1600,
                    y2: 2200, width: 251, color: "#243647" },
            ];
            const slanted = italicizeGlyphStrokes(source, nfd, 12, 3550);
            get("slant-authored").dispatch("click");
            assert.equal(undo.disabled, false);
            assert.equal(undo.textContent, "Undo last glyph slant");
            const tall = scaleGlyphHeight(slanted, nfd, 110, 2250);
            get("scale-height").dispatch("click");
            assert.equal(undo.textContent, "Undo last glyph height");
            assert.equal(undo.disabled, false,
                "a new transform replaces rather than clears recovery");
            await recoverBytes(undo, slanted,
                "undo height restores slanted original exact bytes");
            assert.match(get("status").textContent,
                /Glyph TSV downloaded/);
            assert.equal(redo.disabled, false);
            get("height-percent").value = "100";
            get("scale-height").dispatch("click");
            assert.equal(redo.disabled, false,
                "identity height must preserve redo of accepted height");
            get("height-percent").value = "110";
            await recoverBytes(redo, tall,
                "redo height restores revised bytes");
            const wider = scaleGlyphWidth(tall, nfd, 110, 1750);
            get("scale-width").dispatch("click");
            assert.equal(undo.textContent, "Undo last glyph width");
            await recoverBytes(undo, tall, "undo width restores exact height");
            await recoverBytes(redo, wider, "redo width restores exact bytes");
            const thicker = scaleGlyphInk(wider, nfd, 110);
            get("scale-ink").dispatch("click");
            assert.equal(undo.textContent, "Undo last glyph pen thickness");
            assert.equal(undo.disabled, false);
            assert.equal(redo.disabled, true);
            get("ink-percent").value = "100";
            get("scale-ink").dispatch("click");
            assert.equal(undo.disabled, false,
                "identity ink must retain previous accepted recovery");
            get("ink-percent").value = "110";
            assert.equal(downloads.length, 4,
                "transforms never download files automatically");
            get("ink-percent").value = "151";
            get("scale-ink").dispatch("click");
            assert.match(get("status").textContent, /Ink percent/);
            assert.equal(undo.disabled, false,
                "invalid transform must preserve valid undo history");
            await recoverBytes(undo, wider,
                "undo thickness restores pre-rounded pen widths");
            assert.equal(redo.disabled, false);
            assert.equal(redo.textContent, "Redo last glyph pen thickness");
            get("character").value = "ó";
            get("character").dispatch("input");
            assert.equal(redo.disabled, true,
                "other exact glyph cannot invoke an earlier recovery");
            assert.equal(redo.textContent, "Redo last glyph transform");
            get("character").value = nfd;
            get("character").dispatch("input");
            await recoverBytes(redo, thicker, "redo thickness exact bytes");
            // A manual edit creates another history step across graphemes.
            add("∞", [1650, 2300, 1750, 2400], 251);
            assert.equal(undo.disabled, false);
            assert.equal(undo.textContent, "Undo last stroke addition");
            assert.equal(redo.disabled, true);
            get("character").value = nfd;
            get("character").dispatch("input");
            undo.dispatch("click");
            get("save").dispatch("click");
            assert.equal(await downloads.at(-1).text(), encodeGlyphLibrary([
                ...thicker,
                { grapheme: "∞", x1: 1650, y1: 2300, x2: 1750,
                    y2: 2400, width: 251, color: "#243647" },
            ]));
            editor.reset();
            assert.equal(undo.disabled, true);
            assert.equal(redo.disabled, true);
        } finally {
            ui.restore();
        }
    });

test("whole-glyph translation preserves exact ink paths and refuses clips",
    () => {
    const nfd = "o\u0301";
    const source = [
        { ...stroke(nfd), x1: 450, y1: 1000, x2: 850, y2: 1600,
            width: 251 },
        { ...stroke("ó"), x1: 300, y1: 1000, x2: 500, y2: 1500 },
        { ...stroke(nfd), x1: 850, y1: 1600, x2: 1300, y2: 2200,
            width: 251 },
    ];
    const before = structuredClone(source);
    const moved = translateGlyphStrokes(source, nfd, -200, 175);
    assert.deepEqual(moved.map((row) => row.grapheme),
        [nfd, "ó", nfd]);
    assert.equal(moved[0].x1, 250);
    assert.equal(moved[0].y1, 1175);
    assert.equal(moved[2].x1, moved[0].x2);
    assert.equal(moved[2].y1, moved[0].y2);
    assert.deepEqual(moved[1], source[1]);
    assert.deepEqual(source, before);
    assert.deepEqual(translateGlyphStrokes(source, nfd, 0, 0), before);
    for (const [glyph, dx, dy] of [
        [nfd, -326, 175], [nfd, 0, -1000], [nfd, 3501, 0],
        [nfd, 0, 4501], [nfd, 1.5, 0], ["ó", -300, 0],
        ["ñ", 0, 1], ["oo", 0, 1],
    ]) {
        assert.throws(() => translateGlyphStrokes(source, glyph, dx, dy));
    }
    assert.equal(translateGlyphStrokes(Array.from({ length: 4096 },
        (_, i) => ({ ...stroke(nfd), x1: 500, y1: 500,
            x2: 600 + i % 100, y2: 700 })), nfd, 100, 100).length,
        4096);
});

test("whole-glyph position preview, apply and exact one-step recovery",
    async () => {
    const ui = mountEditor();
    try {
        const { get, editor, downloads } = ui;
        const nfd = "o\u0301";
        const preview = get("move-preview");
        const status = get("move-preview-status");
        assert.equal(get("move-all").disabled, true);
        editor.start();
        get("character").value = nfd;
        get("character").dispatch("input");
        for (const [key, value] of Object.entries({
            x1: 450, y1: 1000, x2: 850, y2: 1600, width: 251,
        })) get(key).value = String(value);
        get("add").dispatch("click");
        const original = [{ grapheme: nfd, x1: 450, y1: 1000,
            x2: 850, y2: 1600, width: 251, color: "#243647" }];
        assert.equal(get("move-all").disabled, false);
        assert.equal(preview.children.length, 1);
        assert.equal(downloads.length, 0);
        get("move-x").value = "-200";
        get("move-x").dispatch("input");
        get("move-y").value = "175";
        get("move-y").dispatch("input");
        assert.equal(Number(preview.children[0].attributes.get("x1")), 250);
        assert.equal(Number(preview.children[0].attributes.get("y1")), 1175);
        assert.match(status.textContent, /preview only, draft unchanged/);
        get("move-x").value = "-326";
        get("move-x").dispatch("input");
        assert.equal(preview.children.length, 0);
        assert.match(status.textContent, /clips glyph cell/);
        get("move-all").dispatch("click");
        assert.match(get("status").textContent, /clips glyph cell/);
        get("move-x").value = "-200";
        get("move-x").dispatch("input");
        get("move-all").dispatch("click");
        const moved = translateGlyphStrokes(original, nfd, -200, 175);
        assert.match(get("status").textContent, /Moved all authored/);
        assert.equal(downloads.length, 0);
        get("save").dispatch("click");
        assert.equal(await downloads.at(-1).text(), encodeGlyphLibrary(moved));
        assert.equal(get("slant-undo").disabled, false);
        assert.equal(get("slant-undo").textContent, "Undo last glyph move");
        get("slant-undo").dispatch("click");
        get("save").dispatch("click");
        assert.equal(await downloads.at(-1).text(),
            encodeGlyphLibrary(original));
        get("slant-redo").dispatch("click");
        get("save").dispatch("click");
        assert.equal(await downloads.at(-1).text(), encodeGlyphLibrary(moved));
        get("move-x").value = "0";
        get("move-y").value = "0";
        get("move-all").dispatch("click");
        assert.equal(get("slant-undo").disabled, false,
            "zero movement must preserve valid history");
        get("character").value = "ó";
        get("character").dispatch("input");
        assert.equal(get("move-all").disabled, true);
        editor.reset();
        assert.equal(preview.children.length, 0);
        assert.match(status.textContent, /Requires a compatible backend/);
    } finally {
        ui.restore();
    }
});


test("whole-glyph rotation preserves directed exact Unicode ink", () => {
    const nfd = "o\u0301";
    const source = [
        { ...stroke(nfd), x1: 450, y1: 1000, x2: 850, y2: 1600,
            width: 251 },
        { ...stroke("ó"), x1: 600, y1: 1200, x2: 900, y2: 1550 },
        { ...stroke(nfd), x1: 850, y1: 1600, x2: 1300, y2: 2200,
            width: 251 },
        { ...stroke("∞"), x1: 500, y1: 900, x2: 700, y2: 1100 },
    ];
    const original = structuredClone(source);
    for (const angle of [-10, 0, 10, 20]) {
        const rotated = rotateGlyphStrokes(source, nfd,
            angle, 1750, 2250);
        assert.equal(rotated.length, 4);
        assert.equal(rotated[2].x1, rotated[0].x2);
        assert.equal(rotated[2].y1, rotated[0].y2);
        assert.deepEqual(rotated[1], source[1]);
        assert.deepEqual(rotated[3], source[3]);
    }
    assert.deepEqual(rotateGlyphStrokes(source, nfd, 0, 0, 0), source);
    for (const [glyph, angle, x, y] of [
        [nfd, 21, 1750, 2250], [nfd, -21, 1750, 2250],
        [nfd, 1.5, 1750, 2250], [nfd, 10, 3501, 2250],
        [nfd, 10, 1750, 4501], [nfd, 20, 0, 0],
        ["ñ", 10, 1750, 2250], ["ab", 10, 1750, 2250],
    ]) {
        assert.throws(() => rotateGlyphStrokes(source,
            glyph, angle, x, y));
    }
    assert.deepEqual(source, original);
    assert.throws(() => rotateGlyphStrokes([{
        grapheme: nfd, x1: 1300, y1: 1626,
        x2: 1301, y2: 1626, width: 250, color: "#243647",
    }], nfd, 20, 1750, 2250), /collapses a pen segment/);
    const full = Array.from({ length: 4096 }, (_, i) => ({
        ...stroke(nfd), x1: 1200, y1: 1600,
        x2: 1300 + i % 100, y2: 1700,
    }));
    assert.equal(rotateGlyphStrokes(full, nfd,
        10, 1750, 2250).length, 4096);
});

test("whole-glyph rotation proposal and one-step undo are session-safe",
    async () => {
    const ui = mountEditor();
    try {
        const { get, editor, downloads } = ui;
        const nfd = "o\u0301";
        const preview = get("rotate-preview");
        assert.equal(get("rotate-all").disabled, true);
        editor.start();
        get("character").value = nfd;
        get("character").dispatch("input");
        for (const [key, value] of Object.entries({
            x1: 450, y1: 1000, x2: 850, y2: 1600, width: 251,
        })) get(key).value = String(value);
        get("add").dispatch("click");
        const original = [{ grapheme: nfd, x1: 450, y1: 1000,
            x2: 850, y2: 1600, width: 251, color: "#243647" }];
        assert.equal(get("rotate-all").disabled, false);
        get("rotate-angle").value = "10";
        get("rotate-angle").dispatch("input");
        const proposed = rotateGlyphStrokes(original, nfd,
            10, 1750, 2250);
        assert.equal(Number(preview.children[0].attributes.get("x1")),
            proposed[0].x1);
        assert.match(get("rotate-preview-status").textContent,
            /preview only, draft unchanged/);
        assert.equal(downloads.length, 0);
        get("rotate-angle").value = "20";
        get("rotate-angle").dispatch("input");
        get("rotate-pivot-x").value = "0";
        get("rotate-pivot-x").dispatch("input");
        get("rotate-pivot-y").value = "0";
        get("rotate-pivot-y").dispatch("input");
        assert.equal(preview.children.length, 0);
        get("rotate-all").dispatch("click");
        assert.match(get("status").textContent, /clips glyph cell/);
        get("rotate-angle").value = "10";
        get("rotate-pivot-x").value = "1750";
        get("rotate-pivot-y").value = "2250";
        get("rotate-all").dispatch("click");
        assert.match(get("status").textContent, /Rotated all authored/);
        get("save").dispatch("click");
        assert.equal(await downloads.at(-1).text(),
            encodeGlyphLibrary(proposed));
        assert.equal(get("slant-undo").textContent,
            "Undo last glyph rotation");
        get("slant-undo").dispatch("click");
        get("save").dispatch("click");
        assert.equal(await downloads.at(-1).text(),
            encodeGlyphLibrary(original));
        get("slant-redo").dispatch("click");
        get("save").dispatch("click");
        assert.equal(await downloads.at(-1).text(),
            encodeGlyphLibrary(proposed));
        get("rotate-angle").value = "0";
        get("rotate-all").dispatch("click");
        assert.equal(get("slant-undo").disabled, false);
        get("character").value = "ñ";
        get("character").dispatch("input");
        assert.equal(get("rotate-all").disabled, true);
        editor.reset();
        assert.equal(preview.children.length, 0);
        assert.match(get("rotate-preview-status").textContent,
            /Requires a compatible backend/);
    } finally {
        ui.restore();
    }
});

test("opt-in cursive e and acute é keep exact directed pen paths", () => {
    const geometry = [1550, 2450, 700, 250, "#123456"];
    const printed = geometricStarter("e", ...geometry);
    const body = geometricStarter("e", ...geometry, undefined, "cursive");
    assert.equal(printed.length, 37);
    assert.equal(body.length, 39);
    assert.notDeepEqual(body, printed,
        "manual connected-pen e is not silently substituted for printed e");
    assert.deepEqual(body.slice(2, 38), printed.slice(0, 36)
        .toReversed().map((line) => ({ ...line, x1: line.x2,
            y1: line.y2, x2: line.x1, y2: line.y1 })),
    "the authored circular C contour is traversed in reverse pen order");
    for (const glyph of ["e", "é", "e\u0301"]) {
        const lines = geometricStarter(glyph, ...geometry,
            undefined, "cursive");
        assert.equal(lines.length, glyph === "e" ? 39 : 40);
        const paths = candidatePenPaths(lines, glyph);
        assert.deepEqual(paths.map(({ first, last }) => [first, last]),
            glyph === "e" ? [[1, 39]] : [[1, 39], [40, 40]],
            "base has one continuously directed path; acute lifts once");
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(lines)),
            lines, "exact Unicode identity survives native UTF-8 TSV");
    }
    assert.deepEqual(geometricStarter("e", ...geometry), printed,
        "building the cursive e never mutates the default printed form");
    assert.deepEqual(geometricStarter("é", ...geometry),
        geometricStarter("e\u0301", ...geometry).map((line) => ({
            ...line, grapheme: "é",
        })), "NFC/NFD printed shape remains separate and unchanged");
    assert.throws(() => geometricStarter("e", 1550, 2450,
        1400, 250, "#123456", undefined, "cursive"),
    /Cursive e and pen clip/);
    assert.throws(() => geometricStarter("K", ...geometry,
        undefined, "cursive"), /Cursive variant/);
});

test("browser cursive e/é guides and manual TSV stay session-safe",
    async () => {
        const ui = mountEditor();
        try {
            const { editor, get, downloads } = ui;
            editor.start();
            get("form").value = "e-cursive";
            get("form").dispatch("change");
            for (const id of ["ideal-entry", "ideal-approach",
                "ideal-exit", "ideal-c-loop"]) {
                assert.equal(get(id).attributes.get("visibility"),
                    "visible", `connected e shows ${id}`);
            }
            assert.equal(get("ideal-e-bar").attributes.get("visibility"),
                "hidden", "connected crossing replaces printed e bar");
            assert.match(get("ideal-c-loop").attributes.get("d"),
                / A 1100 1100 0 1 0 /,
                "SVG guide reverses the C arc direction");
            get("character").value = "e";
            get("character").dispatch("input");
            get("construct").dispatch("click");
            get("save").dispatch("click");
            assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
                geometricStarter("e", 1550, 2450, 1100, 250,
                    "#243647", undefined, "cursive"));
            get("circle-radius").value = "700";
            get("circle-radius").dispatch("input");
            get("form").value = "e\u0301-cursive";
            get("form").dispatch("change");
            get("character").value = "e\u0301";
            get("character").dispatch("input");
            assert.equal(get("ideal-accent").attributes.get("visibility"),
                "visible", "NFD acute remains separately lifted ink");
            get("construct").dispatch("click");
            get("save").dispatch("click");
            const lines = parseGlyphLibrary(await downloads.at(-1).text());
            assert.deepEqual(lines.filter((line) =>
                line.grapheme === "e\u0301"), geometricStarter("e\u0301",
                1550, 2450, 700, 250, "#243647", undefined, "cursive"));
            get("circle-radius").value = "1400";
            get("circle-radius").dispatch("input");
            assert.equal(get("ideal-c-loop").attributes.get("visibility"),
                "hidden", "clipped draft cannot show stale guide");
            get("construct").dispatch("click");
            get("save").dispatch("click");
            assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
                lines, "invalid geometry preserves existing unsaved ink");
            editor.reset();
            assert.equal(get("ideal-entry").attributes.get("visibility"),
                "hidden", "session end clears the connected-pen guide");
        } finally {
            ui.restore();
        }
    });

test("opt-in continuous-pen c keeps a single reversible open arc", () => {
    const geometry = [1550, 2450, 700, 250, "#243647"];
    const printed = geometricStarter("c", ...geometry);
    const cursive = geometricStarter("c", ...geometry,
        undefined, "cursive");
    assert.equal(printed.length, 36);
    assert.equal(cursive.length, 38);
    assert.deepEqual(candidatePenPaths(cursive, "c").map(
        ({ first, last }) => [first, last]), [[1, 38]],
    "entry, reversed circle contour and exit form one pen-down gesture");
    assert.deepEqual(cursive.slice(1, 37), printed.toReversed().map(
        (line) => ({ ...line, x1: line.x2, y1: line.y2,
            x2: line.x1, y2: line.y1 })),
    "manual c preserves exact sampled circular C contour in reverse order");
    assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(cursive)), cursive);
    assert.deepEqual(geometricStarter("c", ...geometry), printed,
        "opt-in form cannot replace default printed c");
    assert.throws(() => geometricStarter("C", ...geometry,
        undefined, "cursive"), /Cursive variant/);
    assert.throws(() => geometricStarter("c", 1550, 2450,
        1600, 250, "#243647", undefined, "cursive"),
    /Shape and pen thickness must fit/);
});

test("browser manual cursive c SVG and download are exact", async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        editor.start();
        get("form").value = "c-cursive";
        get("form").dispatch("change");
        assert.equal(get("ideal-c-loop").attributes.get("visibility"),
            "visible");
        assert.match(get("ideal-c-loop").attributes.get("d"),
            / A 1100 1100 0 1 0 /);
        assert.equal(get("ideal-entry").attributes.get("visibility"),
            "visible");
        assert.equal(get("ideal-exit").attributes.get("visibility"),
            "visible");
        assert.equal(get("ideal-approach").attributes.get("visibility"),
            "hidden", "only e needs an inside crossing");
        get("character").value = "c";
        get("character").dispatch("input");
        get("construct").dispatch("click");
        get("save").dispatch("click");
        const expected = geometricStarter("c", 1550, 2450, 1100, 250,
            "#243647", undefined, "cursive");
        assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
            expected);
        get("circle-radius").value = "1600";
        get("circle-radius").dispatch("input");
        assert.equal(get("ideal-c-loop").attributes.get("visibility"),
            "hidden", "invalid radius hides stale C contour");
        get("construct").dispatch("click");
        get("save").dispatch("click");
        assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
            expected, "invalid starter cannot alter the unsaved draft");
        editor.reset();
        assert.equal(get("ideal-entry").attributes.get("visibility"),
            "hidden");
    } finally {
        ui.restore();
    }
});

test("manual continuous-pen i/í retain directed body and lifted marks", () => {
    const args = [1550, 2450, 1100, 250, "#243647"];
    for (const glyph of ["i", "í", "i\u0301"]) {
        const printed = geometricStarter(glyph, ...args);
        const cursive = geometricStarter(glyph, ...args,
            undefined, "cursive");
        assert.equal(printed.length, 2);
        assert.equal(cursive.length, 4);
        assert.deepEqual(candidatePenPaths(cursive, glyph).map(
            ({ first, last }) => [first, last]), [[1, 3], [4, 4]],
        "the dot or acute requires exactly one deliberate pen lift");
        assert.deepEqual(cursive[1], printed[0],
            "opt-in entry/exit preserve the exact printed downstroke");
        assert.deepEqual(cursive[3], printed[1],
            "the existing dot or acute is not replaced or joined");
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(cursive)),
            cursive, "native TSV keeps exact Unicode identity and pen order");
        assert.deepEqual(geometricStarter(glyph, ...args), printed,
            "choosing cursive must never mutate the printed baseline");
    }
    assert.deepEqual(geometricStarter("i\u0301", ...args,
        undefined, "cursive").map((s) => ({ ...s, grapheme: "í" })),
    geometricStarter("í", ...args, undefined, "cursive"),
    "NFC and NFD are distinct keys for identical physical geometry");
    assert.throws(() => geometricStarter("i", 1550, 2450, 700,
        250, "#243647", undefined, "cursive"),
    /Dotted i requires visible pen-clearance/);
    assert.throws(() => geometricStarter("I", ...args,
        undefined, "cursive"), /Cursive variant/);
});

test("browser manual cursive i/í guides, exact TSV and refusal", async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        editor.start();
        get("form").value = "i-cursive";
        get("form").dispatch("change");
        for (const part of ["ideal-entry", "ideal-exit",
            "ideal-i-stem", "ideal-i-dot"]) {
            assert.equal(get(part).attributes.get("visibility"),
                "visible", `manual i exposes ${part}`);
        }
        assert.equal(get("ideal-approach").attributes.get("visibility"),
            "hidden", "i has no e-style interior crossing");
        assert.equal(get("ideal-c-loop").attributes.get("visibility"),
            "hidden", "i has no circular arc");
        assert.equal(get("ideal-entry").attributes.get("x2"), "1550");
        assert.equal(get("ideal-entry").attributes.get("y2"), "2010");
        get("character").value = "i";
        get("character").dispatch("input");
        get("construct").dispatch("click");
        get("form").value = "i\u0301-cursive";
        get("form").dispatch("change");
        get("character").value = "i\u0301";
        get("character").dispatch("input");
        assert.equal(get("ideal-i-dot").attributes.get("visibility"),
            "hidden", "the acute replaces the disconnected dot");
        assert.equal(get("ideal-accent").attributes.get("visibility"),
            "visible");
        get("construct").dispatch("click");
        get("save").dispatch("click");
        const ink = ["i", "i\u0301"].flatMap((g) => geometricStarter(
            g, 1550, 2450, 1100, 250, "#243647",
            undefined, "cursive"));
        assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
            ink);
        get("circle-radius").value = "700";
        get("circle-radius").dispatch("input");
        assert.equal(get("ideal-entry").attributes.get("visibility"),
            "hidden", "clipped dot gap cannot leave a stale guide");
        get("construct").dispatch("click");
        get("save").dispatch("click");
        assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
            ink, "invalid cursive marks leave the authored draft intact");
        editor.reset();
        assert.equal(get("ideal-exit").attributes.get("visibility"),
            "hidden", "reset clears manual continuous-pen guide");
    } finally {
        ui.restore();
    }
});

test("manual cursive l joins a tall stem without inventing a pen lift", () => {
    const args = [1550, 2450, 1100, 250, "#243647"];
    const printed = geometricStarter("l", ...args);
    const cursive = geometricStarter("l", ...args, undefined, "cursive");
    assert.equal(printed.length, 1);
    assert.equal(cursive.length, 3);
    assert.deepEqual(cursive[1], printed[0],
        "the original full-height l downstroke remains unchanged");
    assert.deepEqual(candidatePenPaths(cursive, "l").map(
        ({ first, last }) => [first, last]), [[1, 3]],
    "entry, tall downstroke and exit form one uninterrupted path");
    assert.equal(cursive[0].y2, 800, "entry meets tall stem top");
    assert.equal(cursive[2].y1, 3550, "exit starts at its baseline");
    assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(cursive)),
        cursive, "no invisible stroke or path marker is synthesized");
    assert.deepEqual(geometricStarter("l", ...args), printed);
    assert.throws(() => geometricStarter("l", 1550, 2100,
        1400, 250, "#243647", undefined, "cursive"),
    /Tall stem and pen must fit/);
    assert.throws(() => geometricStarter("L", ...args,
        undefined, "cursive"), /Cursive variant/);
});

test("browser continuous-pen l guide, download and refusal", async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        editor.start();
        get("form").value = "l-cursive";
        get("form").dispatch("change");
        for (const part of ["ideal-entry", "ideal-exit",
            "ideal-i-stem"]) {
            assert.equal(get(part).attributes.get("visibility"),
                "visible", `cursive l shows ${part}`);
        }
        for (const part of ["ideal-approach", "ideal-c-loop",
            "ideal-i-dot", "ideal-accent"]) {
            assert.equal(get(part).attributes.get("visibility"),
                "hidden", `cursive l never invents ${part}`);
        }
        assert.equal(get("ideal-entry").attributes.get("y2"), "800");
        get("character").value = "l";
        get("character").dispatch("input");
        get("construct").dispatch("click");
        get("save").dispatch("click");
        const expected = geometricStarter("l", 1550, 2450, 1100,
            250, "#243647", undefined, "cursive");
        assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
            expected);
        get("circle-radius").value = "1600";
        get("circle-radius").dispatch("input");
        assert.equal(get("ideal-i-stem").attributes.get("visibility"),
            "hidden", "clipped tall stem must not leave stale SVG");
        get("construct").dispatch("click");
        get("save").dispatch("click");
        assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
            expected, "bad placement cannot change authored ink");
        editor.reset();
        assert.equal(get("ideal-entry").attributes.get("visibility"),
            "hidden");
    } finally {
        ui.restore();
    }
});


test("manual cursive t preserves lifted crossbar and exact stem", () => {
    const args = [1550, 2450, 1100, 250, "#243647"];
    const printed = geometricStarter("t", ...args);
    const joined = geometricStarter("t", ...args, undefined, "cursive");
    assert.equal(printed.length, 2);
    assert.equal(joined.length, 4);
    assert.deepEqual(joined[1], printed[0],
        "joined body retains the original t stem geometry");
    assert.deepEqual(joined[3], printed[1],
        "the crossbar remains its own lifted original pen stroke");
    assert.deepEqual(candidatePenPaths(joined, "t").map(
        ({ first, last }) => [first, last]), [[1, 3], [4, 4]]);
    assert.equal(joined[0].y2, 1350);
    assert.equal(joined[2].y1, 3550);
    assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(joined)), joined);
    assert.deepEqual(geometricStarter("t", ...args), printed,
        "opt-in form leaves the printed t unchanged");
    assert.throws(() => geometricStarter("t", 1550, 1250,
        1300, 250, "#243647", undefined, "cursive"),
    /Shape and pen thickness must fit/);
    assert.throws(() => geometricStarter("T", ...args,
        undefined, "cursive"), /Cursive variant/);
});

test("browser manual cursive t guides and saves a lifted bar", async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        editor.start();
        get("form").value = "t-cursive";
        get("form").dispatch("change");
        for (const part of ["ideal-entry", "ideal-exit",
            "ideal-i-stem", "ideal-e-bar"]) {
            assert.equal(get(part).attributes.get("visibility"),
                "visible", `manual t must show ${part}`);
        }
        assert.equal(get("ideal-entry").attributes.get("y2"), "1350");
        assert.equal(get("ideal-approach").attributes.get("visibility"),
            "hidden");
        assert.equal(get("ideal-i-dot").attributes.get("visibility"),
            "hidden", "the t crossbar is not a dotted i mark");
        get("character").value = "t";
        get("character").dispatch("input");
        get("construct").dispatch("click");
        get("save").dispatch("click");
        const expected = geometricStarter("t", 1550, 2450, 1100,
            250, "#243647", undefined, "cursive");
        assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
            expected);
        get("circle-radius").value = "1550";
        get("circle-radius").dispatch("input");
        assert.equal(get("ideal-entry").attributes.get("visibility"),
            "hidden", "clipped t cannot leave a stale connector guide");
        get("construct").dispatch("click");
        get("save").dispatch("click");
        assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
            expected, "invalid selection preserves saved TSV");
        editor.reset();
        assert.equal(get("ideal-entry").attributes.get("visibility"),
            "hidden", "session reset removes the manual t guide");
    } finally {
        ui.restore();
    }
});

test("manual cursive n and ñ join the hump but lift for tilde", () => {
    const args = [1550, 2450, 1100, 250, "#243647"];
    const printed = geometricStarter("n", ...args);
    const body = geometricStarter("n", ...args, undefined, "cursive");
    assert.equal(printed.length, 34);
    assert.equal(body.length, 37);
    assert.deepEqual(body[1], printed[0], "left stem remains exact");
    assert.deepEqual(body.slice(3, 36), printed.slice(1),
        "hump and descending right stem remain exact");
    assert.deepEqual(candidatePenPaths(body, "n").map(
        ({ first, last }) => [first, last]), [[1, 37]]);
    assert.deepEqual({ x: body[0].x2, y: body[0].y2 },
        { x: body[1].x1, y: body[1].y1 });
    assert.deepEqual({ x: body[2].x2, y: body[2].y2 },
        { x: printed[1].x1, y: printed[1].y1 });
    for (const glyph of ["ñ", "n\u0303"]) {
        const lines = geometricStarter(glyph, ...args,
            undefined, "cursive");
        assert.equal(lines.length, 40);
        assert.deepEqual(candidatePenPaths(lines, glyph).map(
            ({ first, last }) => [first, last]),
            [[1, 37], [38, 40]],
            "a continuous body and a detached three-stroke tilde");
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(lines)),
            lines, "native TSV retains exact Unicode glyph identity");
    }
    assert.deepEqual(geometricStarter("n", ...args), printed);
    assert.throws(() => geometricStarter("n", 300, 2450,
        1100, 250, "#243647", undefined, "cursive"),
    /Shape and pen|Cursive hump/);
});


test("browser manual n/ñ SVG, exact TSV and clipping refusal", async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        editor.start();
        for (const [form, glyph] of [["n-cursive", "n"],
            ["ñ-cursive", "ñ"], ["n\u0303-cursive", "n\u0303"]]) {
            get("form").value = form;
            get("form").dispatch("change");
            get("character").value = glyph;
            get("character").dispatch("input");
            for (const id of ["ideal-entry", "ideal-approach",
                "ideal-exit", "ideal-rounded", "ideal-rounded-left",
                "ideal-rounded-right"]) {
                assert.equal(get(id).attributes.get("visibility"),
                    "visible", `manual ${glyph} shows ${id}`);
            }
            assert.equal(get("ideal-approach").attributes.get("x1"),
                "450");
            assert.equal(get("ideal-approach").attributes.get("y1"),
                "1350");
            assert.equal(get("ideal-approach").attributes.get("y2"),
                "2450");
            assert.equal(get("ideal-c-loop").attributes.get("visibility"),
                "hidden", "hump never substitutes a circular loop");
            assert.equal(get("ideal-diacritic").attributes.get("visibility"),
                glyph === "n" ? "hidden" : "visible");
            get("construct").dispatch("click");
        }
        get("save").dispatch("click");
        const glyphs = ["n", "ñ", "n\u0303"];
        const expected = glyphs.flatMap((glyph) => geometricStarter(
            glyph, 1550, 2450, 1100, 250, "#243647",
            undefined, "cursive"));
        assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
            expected);
        get("circle-radius").value = "1600";
        get("circle-radius").dispatch("input");
        assert.equal(get("ideal-rounded").attributes.get("visibility"),
            "hidden", "clipped starter cannot leave a misleading guide");
        get("construct").dispatch("click");
        get("save").dispatch("click");
        assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
            expected, "refused geometry leaves the draft and download exact");
        editor.reset();
        assert.equal(get("ideal-entry").attributes.get("visibility"),
            "hidden");
    } finally {
        ui.restore();
    }
});

test("manual cursive u and decorated ú/ü retain exact pen lifts", () => {
    const args = [1550, 2450, 1100, 250, "#243647"];
    const print = geometricStarter("u", ...args);
    assert.equal(print.length, 34);
    const body = geometricStarter("u", ...args, undefined, "cursive");
    assert.equal(body.length, 36);
    assert.deepEqual(body.slice(1, 35), print);
    assert.deepEqual(candidatePenPaths(body, "u").map(
        ({ first, last }) => [first, last]), [[1, 36]]);
    for (const [glyph, paths] of [
        ["ú", [[1, 36], [37, 37]]],
        ["u\u0301", [[1, 36], [37, 37]]],
        ["ü", [[1, 36], [37, 37], [38, 38]]],
        ["u\u0308", [[1, 36], [37, 37], [38, 38]]],
    ]) {
        const strokes = geometricStarter(glyph, ...args,
            undefined, "cursive");
        assert.deepEqual(candidatePenPaths(strokes, glyph).map(
            ({ first, last }) => [first, last]), paths);
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(strokes)),
            strokes, "exact composed/decomposed Unicode survives native TSV");
    }
    assert.deepEqual(geometricStarter("u", ...args), print);
    assert.throws(() => geometricStarter("u", 1550, 2450,
        1450, 250, "#243647", undefined, "cursive"),
    /clip glyph cell|Shape and pen/);
});

test("browser manual u/ú/ü picks visible curves and exact TSV", async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        editor.start();
        const glyphs = ["u", "ú", "ü", "u\u0301", "u\u0308"];
        for (const glyph of glyphs) {
            get("form").value = `${glyph}-cursive`;
            get("form").dispatch("change");
            get("character").value = glyph;
            get("character").dispatch("input");
            for (const id of ["ideal-entry", "ideal-exit",
                "ideal-rounded", "ideal-rounded-left",
                "ideal-rounded-right"]) {
                assert.equal(get(id).attributes.get("visibility"),
                    "visible", `manual ${glyph} shows ${id}`);
            }
            for (const id of ["ideal-approach", "ideal-c-loop"]) {
                assert.equal(get(id).attributes.get("visibility"),
                    "hidden", `${glyph} has no crossing or stray loop`);
            }
            assert.equal(get("ideal-entry").attributes.get("y2"),
                "1350", "entry starts at printed u's first stem");
            assert.equal(get("ideal-exit").attributes.get("y1"),
                "1350", "exit starts where printed u ends");
            assert.equal(get("ideal-accent").attributes.get("visibility"),
                glyph.includes("ú") || glyph.includes("\u0301")
                    ? "visible" : "hidden");
            assert.equal(get("ideal-diacritic").attributes.get("visibility"),
                glyph.includes("ü") || glyph.includes("\u0308")
                    ? "visible" : "hidden");
            get("construct").dispatch("click");
        }
        get("save").dispatch("click");
        const expected = glyphs.flatMap((glyph) => geometricStarter(
            glyph, 1550, 2450, 1100, 250, "#243647",
            undefined, "cursive"));
        assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
            expected);
        get("circle-radius").value = "1600";
        get("circle-radius").dispatch("input");
        assert.equal(get("ideal-rounded").attributes.get("visibility"),
            "hidden", "clipping hides the SVG guide");
        get("construct").dispatch("click");
        get("save").dispatch("click");
        assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
            expected, "invalid geometry leaves the authored ink unchanged");
        editor.reset();
        assert.equal(get("ideal-entry").attributes.get("visibility"),
            "hidden");
    } finally {
        ui.restore();
    }
});

test("manual cursive m retains two analytic humps in one pen path", () => {
    const args = [1550, 2450, 1100, 250, "#243647"];
    const printed = geometricStarter("m", ...args);
    const joined = geometricStarter("m", ...args, undefined, "cursive");
    assert.equal(printed.length, 67);
    assert.equal(joined.length, 71, "entry, two returns and exit");
    assert.deepEqual(joined.slice(3, 35), printed.slice(1, 33),
        "the first two sampled arches remain unchanged");
    assert.deepEqual(joined.slice(35, 36), printed.slice(33, 34),
        "middle downstroke remains editable");
    assert.deepEqual(joined.slice(37, 70), printed.slice(34),
        "second pair of arches and right downstroke remain unchanged");
    assert.deepEqual(joined.slice(0, 3).map((line) => [
        line.x1, line.y1, line.x2, line.y2,
    ]), [[250, 3750, 450, 3550], [450, 3550, 450, 1350],
        [450, 1350, 450, 2450]]);
    assert.deepEqual(joined[36], {
        grapheme: "m", x1: 1550, y1: 3550, x2: 1550, y2: 2450,
        width: 250, color: "#243647",
    });
    assert.deepEqual(candidatePenPaths(joined, "m").map(
        ({ first, last }) => [first, last]), [[1, 71]],
    "the first stem and both humps remain one explicit pen gesture");
    assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(joined)), joined);
    assert.deepEqual(geometricStarter("m", ...args), printed,
        "printed m is never silently replaced by cursive ink");
    assert.throws(() => geometricStarter("m", 1550, 2450,
        1450, 250, "#243647", undefined, "cursive"),
    /Shape and pen|clip glyph cell/);
});

test("browser manual cursive m previews both retraces and saves exact ink",
    async () => {
        const ui = mountEditor();
        try {
            const { editor, get, downloads } = ui;
            editor.start();
            get("form").value = "m-cursive";
            get("form").dispatch("change");
            get("character").value = "m";
            get("character").dispatch("input");
            for (const id of ["ideal-entry", "ideal-approach",
                "ideal-exit", "ideal-rounded", "ideal-rounded-left",
                "ideal-rounded-right"]) {
                assert.equal(get(id).attributes.get("visibility"),
                    "visible", `${id} belongs to the connected m guide`);
            }
            assert.equal(get("ideal-approach").attributes.get("y1"),
                "1350", "first raised stem returns to the first hump");
            assert.equal(get("ideal-approach").attributes.get("y2"),
                "2450");
            assert.match(get("ideal-rounded").attributes.get("d"),
                /M 1550 2450 L 1550 3550 L 1550 2450$/,
                "second downstroke returns to the next hump");
            assert.equal(get("ideal-c-loop").attributes.get("visibility"),
                "hidden");
            get("construct").dispatch("click");
            get("save").dispatch("click");
            const accepted = parseGlyphLibrary(await downloads.at(-1).text());
            assert.deepEqual(accepted, geometricStarter("m", 1550, 2450,
                1100, 250, "#243647", undefined, "cursive"));
            get("circle-radius").value = "1600";
            get("circle-radius").dispatch("input");
            assert.equal(get("ideal-rounded").attributes.get("visibility"),
                "hidden", "bad geometry must hide the guide");
            get("construct").dispatch("click");
            get("save").dispatch("click");
            assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
                accepted, "clipped geometry must not change authored ink");
            editor.reset();
            assert.equal(get("ideal-entry").attributes.get("visibility"),
                "hidden");
        } finally {
            ui.restore();
        }
    });

test("manual cursive h retraces its ascender into one rounded path", () => {
    const args = [1550, 2450, 1100, 250, "#243647"];
    const printed = geometricStarter("h", ...args);
    const cursive = geometricStarter("h", ...args, undefined, "cursive");
    assert.equal(printed.length, 34);
    assert.equal(cursive.length, 37);
    assert.deepEqual(cursive[1], printed[0],
        "the tall ascender does not lose its printed geometry");
    assert.deepEqual(cursive.slice(3, 36), printed.slice(1),
        "both smooth shoulder arcs and the descending stem are unchanged");
    assert.deepEqual(cursive[2], {
        grapheme: "h", x1: 450, y1: 800, x2: 450, y2: 2450,
        width: 250, color: "#243647",
    }, "the pen descends to the exact arc origin before continuing");
    assert.deepEqual(candidatePenPaths(cursive, "h").map(
        ({ first, last }) => [first, last]), [[1, 37]]);
    assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(cursive)), cursive);
    assert.deepEqual(geometricStarter("h", ...args), printed);
    assert.throws(() => geometricStarter("h", 1550, 2450,
        1450, 250, "#243647", undefined, "cursive"),
    /Shape and pen|Ascender stem|clip glyph cell/);
});

test("browser manual cursive h shows the tall retrace, saves exact TSV",
    async () => {
        const ui = mountEditor();
        try {
            const { editor, get, downloads } = ui;
            editor.start();
            get("form").value = "h-cursive";
            get("form").dispatch("change");
            get("character").value = "h";
            get("character").dispatch("input");
            for (const id of ["ideal-entry", "ideal-approach",
                "ideal-exit", "ideal-rounded", "ideal-rounded-left",
                "ideal-rounded-right"]) {
                assert.equal(get(id).attributes.get("visibility"),
                    "visible", `${id} is shown on the cursive h guide`);
            }
            assert.equal(get("ideal-rounded-left").attributes.get("y2"),
                "800", "the ascender reaches above the n x-height");
            assert.equal(get("ideal-approach").attributes.get("y1"), "800");
            assert.equal(get("ideal-approach").attributes.get("y2"), "2450");
            assert.equal(get("ideal-c-loop").attributes.get("visibility"),
                "hidden");
            get("construct").dispatch("click");
            get("save").dispatch("click");
            const accepted = parseGlyphLibrary(await downloads.at(-1).text());
            assert.deepEqual(accepted, geometricStarter("h", 1550, 2450,
                1100, 250, "#243647", undefined, "cursive"));
            get("circle-radius").value = "1600";
            get("circle-radius").dispatch("input");
            assert.equal(get("ideal-rounded").attributes.get("visibility"),
                "hidden", "clipped ascender must hide all of the guide");
            get("construct").dispatch("click");
            get("save").dispatch("click");
            assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
                accepted, "refusal leaves accepted strokes unchanged");
            editor.reset();
            assert.equal(get("ideal-entry").attributes.get("visibility"),
                "hidden");
        } finally {
            ui.restore();
        }
    });

test("manual cursive r retraces its stem but does not invent a right stem",
    () => {
        const args = [1550, 2450, 1100, 250, "#243647"];
        const printed = geometricStarter("r", ...args);
        const joined = geometricStarter("r", ...args, undefined, "cursive");
        assert.equal(printed.length, 33);
        assert.equal(joined.length, 36);
        assert.deepEqual(joined[1], printed[0],
            "the source left stem keeps its exact direction and length");
        assert.deepEqual(joined[2], {
            grapheme: "r", x1: 450, y1: 1350, x2: 450, y2: 2450,
            width: 250, color: "#243647",
        }, "a pen-down retrace joins the shoulder after the tall stem");
        assert.deepEqual(joined.slice(3, 35), printed.slice(1),
            "the 32 original quadratic shoulder chords are unchanged");
        assert.deepEqual(joined[35], {
            grapheme: "r", x1: 2650, y1: 2450, x2: 2850, y2: 2250,
            width: 250, color: "#243647",
        }, "r exits at shoulder height, not n's descending baseline");
        assert.deepEqual(candidatePenPaths(joined, "r").map(
            ({ first, last }) => [first, last]), [[1, 36]]);
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(joined)), joined);
        assert.deepEqual(geometricStarter("r", ...args), printed,
            "printed r remains an independent selection");
        assert.throws(() => geometricStarter("r", 1550, 2450,
            1400, 250, "#243647", undefined, "cursive"),
        /Cursive hump and pen clip glyph cell/);
        assert.throws(() => geometricStarter("R", ...args,
            undefined, "cursive"), /Cursive variant/);
    });

test("browser cursive r guide, exact download, and clipped refusal",
    async () => {
        const ui = mountEditor();
        try {
            const { editor, get, downloads } = ui;
            editor.start();
            get("form").value = "r-cursive";
            get("form").dispatch("change");
            get("character").value = "r";
            get("character").dispatch("input");
            for (const id of ["ideal-entry", "ideal-approach",
                "ideal-exit", "ideal-rounded", "ideal-rounded-left"]) {
                assert.equal(get(id).attributes.get("visibility"),
                    "visible", `manual r shows ${id}`);
            }
            assert.equal(get("ideal-rounded-right").attributes
                .get("visibility"), "hidden",
            "the r shoulder never acquires a printed n right stem");
            assert.equal(get("ideal-approach").attributes.get("y1"), "1350");
            assert.equal(get("ideal-approach").attributes.get("y2"), "2450");
            assert.equal(get("ideal-exit").attributes.get("y1"), "2450");
            assert.equal(get("ideal-exit").attributes.get("y2"), "2250");
            get("construct").dispatch("click");
            get("save").dispatch("click");
            const accepted = parseGlyphLibrary(await downloads.at(-1).text());
            assert.deepEqual(accepted, geometricStarter("r", 1550, 2450,
                1100, 250, "#243647", undefined, "cursive"));
            get("circle-radius").value = "1400";
            get("circle-radius").dispatch("input");
            assert.equal(get("ideal-rounded").attributes.get("visibility"),
                "hidden", "a clipped entry hides the entire shape guide");
            get("construct").dispatch("click");
            get("save").dispatch("click");
            assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
                accepted, "invalid edit preserves accepted TSV bytes");
            editor.reset();
            assert.equal(get("ideal-entry").attributes.get("visibility"),
                "hidden", "session end removes the r join guide");
        } finally {
            ui.restore();
        }
    });

test("manual cursive f keeps a tall stem and separately lifted crossbar",
    () => {
        const args = [1550, 2450, 1100, 250, "#243647"];
        const printed = geometricStarter("f", ...args);
        const cursive = geometricStarter("f", ...args, undefined, "cursive");
        assert.equal(printed.length, 2);
        assert.equal(cursive.length, 4);
        assert.deepEqual(cursive[1], printed[0],
            "the tall printed f ascender remains exact");
        assert.deepEqual(cursive[3], printed[1],
            "the horizontal f bar is still a separate pen-down stroke");
        assert.deepEqual(candidatePenPaths(cursive, "f").map(
            ({ first, last }) => [first, last]), [[1, 3], [4, 4]]);
        assert.deepEqual(cursive[0], {
            grapheme: "f", x1: 1350, y1: 1000, x2: 1550, y2: 800,
            width: 250, color: "#243647",
        });
        assert.equal(cursive[2].y1, 3550,
            "the exit starts at the same lower baseline as printed f");
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(cursive)),
            cursive);
        assert.deepEqual(geometricStarter("f", ...args), printed);
        assert.throws(() => geometricStarter("f", 1550, 1900,
            1200, 250, "#243647", undefined, "cursive"),
        /Tall stem and pen must fit/);
        assert.throws(() => geometricStarter("F", ...args,
            undefined, "cursive"), /Cursive variant/);
    });

test("browser cursive f draws its crossbar and saves exact lifted ink",
    async () => {
        const ui = mountEditor();
        try {
            const { editor, get, downloads } = ui;
            editor.start();
            get("form").value = "f-cursive";
            get("form").dispatch("change");
            get("character").value = "f";
            get("character").dispatch("input");
            for (const id of ["ideal-entry", "ideal-exit",
                "ideal-i-stem", "ideal-e-bar"]) {
                assert.equal(get(id).attributes.get("visibility"),
                    "visible", `manual f shows ${id}`);
            }
            assert.equal(get("ideal-entry").attributes.get("y2"), "800");
            assert.equal(get("ideal-approach").attributes.get("visibility"),
                "hidden", "f does not synthesize a crossbar-to-stem join");
            assert.equal(get("ideal-i-dot").attributes.get("visibility"),
                "hidden", "f never borrows the i dot");
            get("construct").dispatch("click");
            get("save").dispatch("click");
            const accepted = parseGlyphLibrary(await downloads.at(-1).text());
            assert.deepEqual(accepted, geometricStarter("f", 1550, 2450,
                1100, 250, "#243647", undefined, "cursive"));
            get("circle-radius").value = "1600";
            get("circle-radius").dispatch("input");
            assert.equal(get("ideal-entry").attributes.get("visibility"),
                "hidden", "clipped ascender hides both connector guides");
            get("construct").dispatch("click");
            get("save").dispatch("click");
            assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
                accepted, "invalid f replacement keeps exact accepted TSV");
            editor.reset();
            assert.equal(get("ideal-exit").attributes.get("visibility"),
                "hidden");
        } finally {
            ui.restore();
        }
    });


test("manual cursive j keeps one hooked descender and lifted dot", () => {
    const args = [1550, 2450, 1100, 250, "#243647"];
    const printed = geometricStarter("j", ...args);
    const joined = geometricStarter("j", ...args, undefined, "cursive");
    assert.equal(printed.length, 18);
    assert.equal(joined.length, 20);
    assert.deepEqual(joined.slice(1, 18), printed.slice(0, 17),
        "the exact printed stem and 32 directed quadratic chords survive");
    assert.deepEqual(joined[19], printed[17],
        "the dot stays a separate lifted stroke");
    assert.deepEqual(candidatePenPaths(joined, "j").map(
        ({ first, last }) => [first, last]), [[1, 19], [20, 20]]);
    assert.deepEqual(joined[0], {
        grapheme: "j", x1: 1350, y1: 2210, x2: 1550, y2: 2010,
        width: 250, color: "#243647",
    });
    assert.deepEqual(joined[18], {
        grapheme: "j", x1: 1000, y1: 3990, x2: 1200, y2: 3790,
        width: 250, color: "#243647",
    });
    assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(joined)), joined);
    assert.deepEqual(geometricStarter("j", ...args), printed);
    assert.throws(() => geometricStarter("j", 1550, 2450,
        1400, 250, "#243647", undefined, "cursive"),
    /Hooked descender and pen must fit/);
    assert.throws(() => geometricStarter("J", ...args,
        undefined, "cursive"), /Cursive variant/);
});

test("browser cursive j guide and TSV keep dot lifted after clipping refusal",
    async () => {
        const ui = mountEditor();
        try {
            const { editor, get, downloads } = ui;
            editor.start();
            get("form").value = "j-cursive";
            get("form").dispatch("change");
            get("character").value = "j";
            get("character").dispatch("input");
            for (const id of ["ideal-entry", "ideal-exit",
                "ideal-i-stem", "ideal-i-dot", "ideal-rounded"]) {
                assert.equal(get(id).attributes.get("visibility"),
                    "visible", `manual j shows ${id}`);
            }
            assert.equal(get("ideal-entry").attributes.get("y2"),
                "2010", "entry joins the actual printed j stem");
            assert.equal(get("ideal-exit").attributes.get("x1"),
                "1000", "exit begins at the sampled hook end");
            assert.equal(get("ideal-approach").attributes.get("visibility"),
                "hidden", "j does not invent a stem-to-dot join");
            get("construct").dispatch("click");
            get("save").dispatch("click");
            const accepted = parseGlyphLibrary(await downloads.at(-1).text());
            assert.deepEqual(accepted, geometricStarter("j", 1550, 2450,
                1100, 250, "#243647", undefined, "cursive"));
            get("circle-radius").value = "1400";
            get("circle-radius").dispatch("input");
            assert.equal(get("ideal-entry").attributes.get("visibility"),
                "hidden", "clipped descender hides all connected guides");
            get("construct").dispatch("click");
            get("save").dispatch("click");
            assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
                accepted, "rejected j never replaces saved exact Unicode ink");
            editor.reset();
            assert.equal(get("ideal-exit").attributes.get("visibility"),
                "hidden");
        } finally {
            ui.restore();
        }
    });


test("manual connected v/w keep both printed zigzags exact", () => {
    const args = [1550, 2450, 1100, 250, "#243647"];
    for (const [glyph, count] of [["v", 2], ["w", 4]]) {
        const printed = geometricStarter(glyph, ...args);
        const joined = geometricStarter(glyph, ...args,
            undefined, "cursive");
        assert.equal(printed.length, count);
        assert.equal(joined.length, count + 2);
        assert.deepEqual(joined.slice(1, -1), printed,
            `exact printed ${glyph} zigzag is not modified`);
        assert.deepEqual(candidatePenPaths(joined, glyph).map(
            ({ first, last }) => [first, last]), [[1, count + 2]]);
        assert.deepEqual(joined[0], {
            grapheme: glyph, x1: 250, y1: 1550,
            x2: 450, y2: 1350, width: 250, color: "#243647",
        });
        assert.deepEqual(joined.at(-1), {
            grapheme: glyph, x1: 2650, y1: 1350,
            x2: 2850, y2: 1150, width: 250, color: "#243647",
        });
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(joined)),
            joined);
        assert.deepEqual(geometricStarter(glyph, ...args), printed);
        assert.equal(geometricStarter(glyph, 1300, 2450,
            1100, 250, "#243647").length, count);
        assert.throws(() => geometricStarter(glyph, 1300, 2450,
            1100, 250, "#243647", undefined, "cursive"),
        /Cursive zigzag and pen clip/);
        assert.throws(() => geometricStarter(glyph.toUpperCase(), ...args,
            undefined, "cursive"), /Cursive variant/);
    }
});

test("browser manual v/w previews exact straight ink and refuses clipped entry",
    async () => {
        const ui = mountEditor();
        try {
            const { editor, get, downloads } = ui;
            editor.start();
            for (const glyph of ["v", "w"]) {
                get("form").value = `${glyph}-cursive`;
                get("form").dispatch("change");
                get("character").value = glyph;
                get("character").dispatch("input");
                for (const id of ["ideal-entry", "ideal-exit",
                    "ideal-rounded"]) {
                    assert.equal(get(id).attributes.get("visibility"),
                        "visible", `manual ${glyph} shows ${id}`);
                }
                for (const id of ["ideal-approach", "ideal-i-dot",
                    "ideal-rounded-left", "ideal-rounded-right"]) {
                    assert.equal(get(id).attributes.get("visibility"),
                        "hidden", `manual ${glyph} does not invent ${id}`);
                }
                assert.equal(get("ideal-entry").attributes.get("x2"),
                    "450");
                assert.equal(get("ideal-exit").attributes.get("y1"),
                    "1350");
                get("construct").dispatch("click");
            }
            get("save").dispatch("click");
            const accepted = parseGlyphLibrary(await downloads.at(-1).text());
            assert.deepEqual(accepted, ["v", "w"].flatMap((glyph) =>
                geometricStarter(glyph, 1550, 2450, 1100, 250,
                    "#243647", undefined, "cursive")));
            get("circle-x").value = "1300";
            get("circle-x").dispatch("input");
            assert.equal(get("ideal-entry").attributes.get("visibility"),
                "hidden", "clipped v/w entry hides preview pen guides");
            get("construct").dispatch("click");
            get("save").dispatch("click");
            assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
                accepted, "a rejected w cannot replace an admitted draft");
            editor.reset();
            assert.equal(get("ideal-exit").attributes.get("visibility"),
                "hidden");
        } finally {
            ui.restore();
        }
    });


test("manual connected z keeps printed angular chain exact", () => {
    const args = [1550, 2450, 1100, 250, "#243647"];
    const printed = geometricStarter("z", ...args);
    const joined = geometricStarter("z", ...args, undefined, "cursive");
    assert.equal(printed.length, 3);
    assert.equal(joined.length, 5);
    assert.deepEqual(joined.slice(1, -1), printed);
    assert.deepEqual(candidatePenPaths(joined, "z").map(
        ({ first, last }) => [first, last]), [[1, 5]]);
    assert.deepEqual(joined[0], {
        grapheme: "z", x1: 250, y1: 1550, x2: 450, y2: 1350,
        width: 250, color: "#243647",
    });
    assert.deepEqual(joined.at(-1), {
        grapheme: "z", x1: 2650, y1: 3550, x2: 2850, y2: 3350,
        width: 250, color: "#243647",
    });
    assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(joined)), joined);
    assert.deepEqual(geometricStarter("z", ...args), printed);
    assert.throws(() => geometricStarter("z", 1300, 2450,
        1100, 250, "#243647", undefined, "cursive"),
    /Cursive zigzag and pen clip/);
    assert.throws(() => geometricStarter("Z", ...args,
        undefined, "cursive"), /Cursive variant/);
});

test("browser manual z previews exact pen path and refuses clipped entry",
    async () => {
        const ui = mountEditor();
        try {
            const { editor, get, downloads } = ui;
            editor.start();
            get("form").value = "z-cursive";
            get("form").dispatch("change");
            get("character").value = "z";
            get("character").dispatch("input");
            for (const id of ["ideal-entry", "ideal-exit",
                "ideal-rounded"]) {
                assert.equal(get(id).attributes.get("visibility"),
                    "visible", `manual z shows ${id}`);
            }
            for (const id of ["ideal-approach", "ideal-i-dot",
                "ideal-rounded-left", "ideal-rounded-right"]) {
                assert.equal(get(id).attributes.get("visibility"),
                    "hidden", `manual z does not invent ${id}`);
            }
            assert.equal(get("ideal-entry").attributes.get("x2"), "450");
            assert.equal(get("ideal-exit").attributes.get("y1"), "3550");
            get("construct").dispatch("click");
            get("save").dispatch("click");
            const accepted = parseGlyphLibrary(await downloads.at(-1).text());
            assert.deepEqual(accepted, geometricStarter("z", 1550, 2450,
                1100, 250, "#243647", undefined, "cursive"));
            get("circle-x").value = "1300";
            get("circle-x").dispatch("input");
            assert.equal(get("ideal-entry").attributes.get("visibility"),
                "hidden", "a clipped entry hides the guides");
            get("construct").dispatch("click");
            get("save").dispatch("click");
            assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
                accepted, "failed z cannot replace the valid saved draft");
            editor.reset();
            assert.equal(get("ideal-exit").attributes.get("visibility"),
                "hidden");
        } finally {
            ui.restore();
        }
    });

// The four smooth quadratic arcs already form one ordered ink path.
test("manual connected s preserves every sampled curve chord", () => {
    const args = [1550, 2450, 1100, 250, "#243647"];
    const printed = geometricStarter("s", ...args);
    const joined = geometricStarter("s", ...args, undefined, "cursive");
    assert.equal(printed.length, 64);
    assert.equal(joined.length, 66);
    assert.deepEqual(joined.slice(1, -1), printed,
        "the exact four tangent-continuous printed arcs remain untouched");
    assert.deepEqual(candidatePenPaths(joined, "s").map(
        ({ first, last }) => [first, last]), [[1, 66]]);
    assert.deepEqual(joined[0], {
        grapheme: "s", x1: 2120, y1: 1770,
        x2: 2320, y2: 1570, width: 250, color: "#243647",
    });
    assert.deepEqual(joined.at(-1), {
        grapheme: "s", x1: 780, y1: 3330,
        x2: 980, y2: 3130, width: 250, color: "#243647",
    });
    assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(joined)), joined);
    assert.deepEqual(geometricStarter("s", ...args), printed);
    assert.throws(() => geometricStarter("s", 1200, 2450,
        1100, 250, "#243647", undefined, "cursive"), /Shape and pen/);
    assert.throws(() => geometricStarter("S", ...args,
        undefined, "cursive"), /Cursive variant/);
});

test("browser manual s previews one smooth contour and preserves refused TSV",
    async () => {
        const ui = mountEditor();
        try {
            const { editor, get, downloads } = ui;
            editor.start();
            get("form").value = "s-cursive";
            get("form").dispatch("change");
            get("character").value = "s";
            get("character").dispatch("input");
            for (const id of ["ideal-entry", "ideal-exit",
                "ideal-rounded"]) {
                assert.equal(get(id).attributes.get("visibility"),
                    "visible", `manual s shows ${id}`);
            }
            for (const id of ["ideal-approach", "ideal-i-dot",
                "ideal-rounded-left", "ideal-rounded-right"]) {
                assert.equal(get(id).attributes.get("visibility"),
                    "hidden", `manual s does not invent ${id}`);
            }
            assert.match(get("ideal-rounded").attributes.get("d"),
                /^M 2320 1570 Q 1550 1350 780 1900/);
            assert.equal(get("ideal-entry").attributes.get("x2"), "2320");
            assert.equal(get("ideal-exit").attributes.get("y1"), "3330");
            get("construct").dispatch("click");
            assert.equal(get("preview").children.length, 66);
            get("save").dispatch("click");
            const accepted = parseGlyphLibrary(await downloads.at(-1).text());
            assert.deepEqual(accepted, geometricStarter("s", 1550, 2450,
                1100, 250, "#243647", undefined, "cursive"));
            get("circle-x").value = "1200";
            get("circle-x").dispatch("input");
            assert.equal(get("ideal-entry").attributes.get("visibility"),
                "hidden", "clipped s hides both connector guides");
            get("construct").dispatch("click");
            get("save").dispatch("click");
            assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
                accepted, "invalid s cannot replace exact saved ink");
            editor.reset();
            assert.equal(get("ideal-exit").attributes.get("visibility"),
                "hidden");
        } finally {
            ui.restore();
        }
    });

// The y right fork is deliberately doubled to keep one pen down.
test("manual connected y retains all three directed printed strokes", () => {
    const args = [1550, 2450, 1100, 250, "#243647"];
    const printed = geometricStarter("y", ...args);
    const joined = geometricStarter("y", ...args, undefined, "cursive");
    assert.equal(printed.length, 3);
    assert.equal(joined.length, 6);
    assert.deepEqual([joined[1], joined[3], joined[4]], printed,
        "original two arms and descender retain exact pen direction");
    assert.deepEqual(joined[2], {
        ...printed[1], x1: printed[1].x2, y1: printed[1].y2,
        x2: printed[1].x1, y2: printed[1].y1,
    }, "the extra stroke retraces the right arm deliberately");
    assert.deepEqual(candidatePenPaths(joined, "y").map(
        ({ first, last }) => [first, last]), [[1, 6]]);
    assert.deepEqual(joined[0], {
        grapheme: "y", x1: 250, y1: 1550,
        x2: 450, y2: 1350, width: 250, color: "#243647",
    });
    assert.deepEqual(joined.at(-1), {
        grapheme: "y", x1: 1000, y1: 4100,
        x2: 1200, y2: 3900, width: 250, color: "#243647",
    });
    assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(joined)), joined);
    assert.deepEqual(geometricStarter("y", ...args), printed);
    assert.equal(geometricStarter("y", 1250, 2450,
        1100, 250, "#243647").length, 3,
        "the printed fork remains valid near the left cell margin");
    assert.throws(() => geometricStarter("y", 1250, 2450,
        1100, 250, "#243647", undefined, "cursive"),
        /Cursive y connector and pen clip/);
    assert.throws(() => geometricStarter("y", 1550, 2450,
        1300, 250, "#243647", undefined, "cursive"), /Straight strokes/);
    assert.throws(() => geometricStarter("Y", ...args,
        undefined, "cursive"), /Cursive variant/);
});

test("browser manual y shows fork retrace and refuses clipped replacement",
    async () => {
        const ui = mountEditor();
        try {
            const { editor, get, downloads } = ui;
            editor.start();
            get("form").value = "y-cursive";
            get("form").dispatch("change");
            get("character").value = "y";
            get("character").dispatch("input");
            for (const id of ["ideal-entry", "ideal-approach",
                "ideal-exit", "ideal-rounded"]) {
                assert.equal(get(id).attributes.get("visibility"),
                    "visible", `manual y shows ${id}`);
            }
            assert.equal(get("ideal-approach").attributes.get("x1"),
                "1550", "pen retrace begins at the shared fork");
            assert.equal(get("ideal-approach").attributes.get("x2"),
                "2650", "pen retrace reaches right arm's upper end");
            assert.equal(get("ideal-exit").attributes.get("y1"), "4100");
            for (const id of ["ideal-i-dot", "ideal-rounded-left",
                "ideal-rounded-right"]) {
                assert.equal(get(id).attributes.get("visibility"),
                    "hidden", `manual y does not invent ${id}`);
            }
            get("construct").dispatch("click");
            assert.equal(get("preview").children.length, 6);
            get("save").dispatch("click");
            const accepted = parseGlyphLibrary(await downloads.at(-1).text());
            assert.deepEqual(accepted, geometricStarter("y", 1550, 2450,
                1100, 250, "#243647", undefined, "cursive"));
            get("circle-x").value = "1250";
            get("circle-x").dispatch("input");
            assert.equal(get("ideal-approach").attributes.get("visibility"),
                "hidden", "clipped connector hides the fork retrace guide");
            get("construct").dispatch("click");
            get("save").dispatch("click");
            assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
                accepted, "clipped y leaves the accepted TSV unchanged");
            editor.reset();
            assert.equal(get("ideal-exit").attributes.get("visibility"),
                "hidden");
        } finally {
            ui.restore();
        }
    });

// A visible half-stem retrace and upper arm reversal make a one-pen k.
test("manual connected k preserves printed stem and diagonal order", () => {
    const args = [1550, 2450, 1100, 250, "#243647"];
    const printed = geometricStarter("k", ...args);
    const joined = geometricStarter("k", ...args, undefined, "cursive");
    assert.equal(printed.length, 3);
    assert.equal(joined.length, 7);
    assert.deepEqual([joined[1], joined[4], joined[5]], printed,
        "all printed k strokes retain their original directed geometry");
    assert.deepEqual(joined[2], {
        ...printed[0], x1: printed[0].x2, y1: printed[0].y2,
        x2: printed[1].x2, y2: printed[1].y2,
    }, "the downstroke retraces from the bottom to its fork");
    assert.deepEqual(joined[3], {
        ...printed[1], x1: printed[1].x2, y1: printed[1].y2,
        x2: printed[1].x1, y2: printed[1].y1,
    }, "the upper arm reverses once before its authored downward line");
    assert.deepEqual(candidatePenPaths(joined, "k").map(
        ({ first, last }) => [first, last]), [[1, 7]]);
    assert.deepEqual(joined[0], {
        grapheme: "k", x1: 250, y1: 1000,
        x2: 450, y2: 800, width: 250, color: "#243647",
    });
    assert.deepEqual(joined.at(-1), {
        grapheme: "k", x1: 2650, y1: 3550,
        x2: 2850, y2: 3350, width: 250, color: "#243647",
    });
    assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(joined)), joined);
    assert.deepEqual(geometricStarter("k", ...args), printed);
    assert.equal(geometricStarter("k", 1250, 2450,
        1100, 250, "#243647").length, 3,
        "printed k fits even when the optional entry cannot");
    assert.throws(() => geometricStarter("k", 1250, 2450,
        1100, 250, "#243647", undefined, "cursive"),
        /Cursive k connector and pen clip/);
    assert.throws(() => geometricStarter("K", ...args,
        undefined, "cursive"), /Cursive variant/);
});

test("browser manual k previews both retraces and refuses clipped ink",
    async () => {
        const ui = mountEditor();
        try {
            const { editor, get, downloads } = ui;
            editor.start();
            get("form").value = "k-cursive";
            get("form").dispatch("change");
            get("character").value = "k";
            get("character").dispatch("input");
            for (const id of ["ideal-entry", "ideal-approach",
                "ideal-exit", "ideal-rounded", "ideal-rounded-left"]) {
                assert.equal(get(id).attributes.get("visibility"),
                    "visible", `manual k shows ${id}`);
            }
            assert.equal(get("ideal-rounded-left").attributes.get("y1"),
                "3550", "stem retrace starts at its lowest point");
            assert.equal(get("ideal-rounded-left").attributes.get("y2"),
                "2450", "stem retrace ends at the diagonal fork");
            assert.equal(get("ideal-approach").attributes.get("x1"),
                "450", "the upper-diagonal reversal begins at the fork");
            assert.equal(get("ideal-approach").attributes.get("x2"),
                "2650", "the upper-diagonal reversal reaches the arm top");
            assert.equal(get("ideal-entry").attributes.get("y2"), "800");
            assert.equal(get("ideal-exit").attributes.get("y1"), "3550");
            for (const id of ["ideal-rounded-right", "ideal-i-dot"]) {
                assert.equal(get(id).attributes.get("visibility"),
                    "hidden", `manual k does not invent ${id}`);
            }
            get("construct").dispatch("click");
            assert.equal(get("preview").children.length, 7);
            get("save").dispatch("click");
            const accepted = parseGlyphLibrary(await downloads.at(-1).text());
            assert.deepEqual(accepted, geometricStarter("k", 1550, 2450,
                1100, 250, "#243647", undefined, "cursive"));
            get("circle-x").value = "1250";
            get("circle-x").dispatch("input");
            assert.equal(get("ideal-rounded-left").attributes
                .get("visibility"), "hidden",
            "invalid connector cannot display a stale retrace guide");
            get("construct").dispatch("click");
            get("save").dispatch("click");
            assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
                accepted, "clipped k cannot overwrite accepted TSV");
            editor.reset();
            assert.equal(get("ideal-exit").attributes.get("visibility"),
                "hidden");
        } finally {
            ui.restore();
        }
    });

// An opt-in b joins its printed ascender to the original full circular bowl.
test("manual continuous b retains exact circle and connected pen order", () => {
    const args = [1550, 2450, 1100, 250, "#243647"];
    const printed = geometricStarter("b", ...args);
    const joined = geometricStarter("b", ...args, undefined, "cursive");
    assert.equal(printed.length, 49);
    assert.equal(joined.length, 76);
    assert.deepEqual(joined[1], printed[48],
        "the printed ascender retains its original direction and length");
    assert.deepEqual(joined.slice(3, 27), printed.slice(24, 48),
        "the upper semicircle carries the pen to the loop start");
    assert.deepEqual(joined.slice(27, 75), printed.slice(0, 48),
        "every original printed circle chord appears in order");
    assert.deepEqual(joined[2], {
        grapheme: "b", x1: 450, y1: 3550,
        x2: 450, y2: 2450, width: 250, color: "#243647",
    }, "the ascender retraces precisely to the left circle tangent");
    assert.deepEqual(joined[0], {
        grapheme: "b", x1: 250, y1: 1000,
        x2: 450, y2: 800, width: 250, color: "#243647",
    });
    assert.deepEqual(joined.at(-1), {
        grapheme: "b", x1: 2650, y1: 2450,
        x2: 2850, y2: 2250, width: 250, color: "#243647",
    });
    assert.deepEqual(candidatePenPaths(joined, "b").map(
        ({ first, last }) => [first, last]), [[1, 76]]);
    assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(joined)), joined);
    assert.deepEqual(geometricStarter("b", ...args), printed);
    assert.equal(geometricStarter("b", 1400, 2450,
        1100, 250, "#243647").length, 49,
    "valid printed b must not be blocked by an optional entry");
    assert.throws(() => geometricStarter("b", 1400, 2450,
        1100, 250, "#243647", undefined, "cursive"),
    /Cursive b connector and pen clip/);
    assert.throws(() => geometricStarter("B", ...args,
        undefined, "cursive"), /Cursive variant/);
});

test("browser manual b previews exact ascender retrace and saves TSV",
    async () => {
        const ui = mountEditor();
        try {
            const { editor, get, downloads } = ui;
            editor.start();
            get("form").value = "b-cursive";
            get("form").dispatch("change");
            get("character").value = "b";
            get("character").dispatch("input");
            for (const id of ["ideal-circle", "ideal-stem", "ideal-entry",
                "ideal-approach", "ideal-exit"]) {
                assert.equal(get(id).attributes.get("visibility"),
                    "visible", `manual b previews ${id}`);
            }
            assert.equal(get("ideal-entry").attributes.get("y2"), "800");
            assert.equal(get("ideal-approach").attributes.get("y1"), "3550");
            assert.equal(get("ideal-approach").attributes.get("y2"), "2450");
            assert.equal(get("ideal-approach").attributes.get("x2"), "450");
            assert.equal(get("ideal-exit").attributes.get("x1"), "2650");
            assert.equal(get("ideal-rounded").attributes.get("visibility"),
                "hidden", "b must not expose an unrelated arch");
            get("construct").dispatch("click");
            assert.equal(get("preview").children.length, 76);
            get("save").dispatch("click");
            const accepted = parseGlyphLibrary(await downloads.at(-1).text());
            assert.deepEqual(accepted, geometricStarter("b", 1550, 2450,
                1100, 250, "#243647", undefined, "cursive"));
            get("circle-x").value = "1400";
            get("circle-x").dispatch("input");
            assert.equal(get("ideal-approach").attributes
                .get("visibility"), "hidden",
            "clipped b must hide its stale retrace guide");
            get("construct").dispatch("click");
            get("save").dispatch("click");
            assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
                accepted, "invalid b does not overwrite accepted ink");
            editor.reset();
            assert.equal(get("ideal-entry").attributes.get("visibility"),
                "hidden");
        } finally {
            ui.restore();
        }
    });

// The p descender joins its complete circular bowl without lifting the pen.
test("manual continuous p preserves its printed descender and circle", () => {
    const args = [1550, 2450, 1100, 250, "#243647"];
    const printed = geometricStarter("p", ...args);
    const joined = geometricStarter("p", ...args, undefined, "cursive");
    assert.equal(printed.length, 49);
    assert.equal(joined.length, 76);
    assert.deepEqual(joined[1], printed[48]);
    assert.deepEqual(joined.slice(3, 27), printed.slice(24, 48));
    assert.deepEqual(joined.slice(27, 75), printed.slice(0, 48));
    assert.deepEqual(joined[0], {
        grapheme: "p", x1: 250, y1: 1550,
        x2: 450, y2: 1350, width: 250, color: "#243647",
    });
    assert.deepEqual(joined[2], {
        grapheme: "p", x1: 450, y1: 4100,
        x2: 450, y2: 2450, width: 250, color: "#243647",
    });
    assert.deepEqual(candidatePenPaths(joined, "p").map(
        ({ first, last }) => [first, last]), [[1, 76]]);
    assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(joined)), joined);
    assert.deepEqual(geometricStarter("p", ...args), printed);
    assert.equal(geometricStarter("p", 1400, 2450, 1100,
        250, "#243647").length, 49);
    assert.throws(() => geometricStarter("p", 1400, 2450,
        1100, 250, "#243647", undefined, "cursive"),
    /Cursive p connector and pen clip/);
    assert.throws(() => geometricStarter("P", ...args,
        undefined, "cursive"), /Cursive variant/);
});

test("browser manual p previews descending retrace and refuses clip",
    async () => {
        const ui = mountEditor();
        try {
            const { editor, get, downloads } = ui;
            editor.start();
            get("form").value = "p-cursive";
            get("form").dispatch("change");
            get("character").value = "p";
            get("character").dispatch("input");
            for (const id of ["ideal-circle", "ideal-stem", "ideal-entry",
                "ideal-approach", "ideal-exit"]) {
                assert.equal(get(id).attributes.get("visibility"),
                    "visible", `cursive p shows ${id}`);
            }
            assert.equal(get("ideal-entry").attributes.get("y2"), "1350");
            assert.equal(get("ideal-stem").attributes.get("y2"), "4100");
            assert.equal(get("ideal-approach").attributes.get("y1"), "4100");
            assert.equal(get("ideal-approach").attributes.get("y2"), "2450");
            get("construct").dispatch("click");
            assert.equal(get("preview").children.length, 76);
            get("save").dispatch("click");
            const accepted = parseGlyphLibrary(await downloads.at(-1).text());
            assert.deepEqual(accepted, geometricStarter("p", 1550, 2450,
                1100, 250, "#243647", undefined, "cursive"));
            get("circle-x").value = "1400";
            get("circle-x").dispatch("input");
            assert.equal(get("ideal-approach").attributes
                .get("visibility"), "hidden");
            get("construct").dispatch("click");
            get("save").dispatch("click");
            assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
                accepted, "clipped p does not replace downloaded ink");
            editor.reset();
            assert.equal(get("ideal-entry").attributes.get("visibility"),
                "hidden");
        } finally {
            ui.restore();
        }
    });

// Right-stem d/q retain the original circle and exact ascender/descender.
test("manual cursive d/q carry the full bowl into their right stems", () => {
    const args = [1550, 2450, 1100, 250, "#243647"];
    for (const [glyph, top, bottom] of [
        ["d", 800, 3550], ["q", 1350, 4100],
    ]) {
        const printed = geometricStarter(glyph, ...args);
        const joined = geometricStarter(glyph, ...args, undefined,
            "cursive");
        assert.equal(printed.length, 49);
        assert.equal(joined.length, 76);
        assert.deepEqual(joined.slice(1, 25), printed.slice(24, 48));
        assert.deepEqual(joined.slice(25, 49), printed.slice(0, 24));
        assert.deepEqual(joined.slice(49, 73), printed.slice(24, 48));
        assert.deepEqual(joined[73], {
            grapheme: glyph, x1: 2650, y1: 2450, x2: 2650, y2: top,
            width: 250, color: "#243647",
        });
        assert.deepEqual(joined[74], printed[48]);
        assert.deepEqual(joined[75], {
            grapheme: glyph, x1: 2650, y1: bottom,
            x2: 2850, y2: bottom - 200,
            width: 250, color: "#243647",
        });
        assert.deepEqual(candidatePenPaths(joined, glyph).map(
            ({ first, last }) => [first, last]), [[1, 76]]);
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(joined)),
            joined);
        assert.deepEqual(geometricStarter(glyph, ...args), printed);
        assert.equal(geometricStarter(glyph, 1400, 2450, 1100,
            250, "#243647").length, 49);
        assert.throws(() => geometricStarter(glyph, 1400, 2450,
            1100, 250, "#243647", undefined, "cursive"),
        new RegExp(`Cursive ${glyph} connector and pen clip`));
        assert.throws(() => geometricStarter(glyph.toUpperCase(), ...args,
            undefined, "cursive"), /Cursive variant/);
    }
});

test("browser manual d/q SVG, TSV and clipped-entry refusal", async () => {
    const ui = mountEditor();
    try {
        const { editor, get, downloads } = ui;
        editor.start();
        for (const [glyph, top, bottom] of [
            ["d", "800", "3550"], ["q", "1350", "4100"],
        ]) {
            get("form").value = `${glyph}-cursive`;
            get("form").dispatch("change");
            get("character").value = glyph;
            get("character").dispatch("input");
            for (const id of ["ideal-circle", "ideal-stem", "ideal-entry",
                "ideal-approach", "ideal-exit"]) {
                assert.equal(get(id).attributes.get("visibility"),
                    "visible", `manual ${glyph} previews ${id}`);
            }
            assert.equal(get("ideal-approach").attributes.get("x1"),
                "2650");
            assert.equal(get("ideal-approach").attributes.get("y1"),
                "2450");
            assert.equal(get("ideal-approach").attributes.get("y2"), top);
            assert.equal(get("ideal-exit").attributes.get("y1"), bottom);
            get("construct").dispatch("click");
            assert.equal(get("preview").children.length, 76);
        }
        get("save").dispatch("click");
        const accepted = parseGlyphLibrary(await downloads.at(-1).text());
        assert.deepEqual(accepted, ["d", "q"].flatMap((glyph) =>
            geometricStarter(glyph, 1550, 2450, 1100, 250,
                "#243647", undefined, "cursive")));
        get("circle-x").value = "1400";
        get("circle-x").dispatch("input");
        assert.equal(get("ideal-approach").attributes
            .get("visibility"), "hidden");
        get("construct").dispatch("click");
        get("save").dispatch("click");
        assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
            accepted, "invalid q cannot overwrite accepted d/q strokes");
        editor.reset();
        assert.equal(get("ideal-entry").attributes.get("visibility"),
            "hidden");
    } finally {
        ui.restore();
    }
});

// The two printed X diagonals stay intact; deliberate half-diagonal
// retraces connect them into one optional pen gesture at their crossing.
test("manual cursive x retraces its crossing, keeps printed diagonals", () => {
    const args = [1550, 2450, 1100, 250, "#243647"];
    const printed = geometricStarter("x", ...args);
    const joined = geometricStarter("x", ...args, undefined, "cursive");
    assert.equal(printed.length, 2);
    assert.equal(joined.length, 6);
    assert.deepEqual(joined[1], printed[0]);
    assert.deepEqual(joined[4], printed[1]);
    assert.deepEqual(joined.slice(2, 4).map(({ x1, y1, x2, y2 }) =>
        [x1, y1, x2, y2]), [
        [2650, 3550, 1550, 2450],
        [1550, 2450, 2650, 1350],
    ]);
    assert.deepEqual(candidatePenPaths(joined, "x").map(
        ({ first, last }) => [first, last]), [[1, 6]]);
    assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(joined)), joined);
    assert.deepEqual(geometricStarter("x", ...args), printed);
    assert.equal(geometricStarter("x", 1400, 2450, 1100,
        250, "#243647").length, 2);
    assert.throws(() => geometricStarter("x", 1400, 2450, 1100,
        250, "#243647", undefined, "cursive"),
    /Cursive x connector and pen clip/);
    assert.throws(() => geometricStarter("X", ...args, undefined,
        "cursive"), /Cursive variant/);
});

test("browser manual x shows both retraces and refuses clipped entry",
    async () => {
        const ui = mountEditor();
        try {
            const { editor, get, downloads } = ui;
            editor.start();
            get("form").value = "x-cursive";
            get("form").dispatch("change");
            get("character").value = "x";
            get("character").dispatch("input");
            for (const id of ["ideal-rounded", "ideal-entry",
                "ideal-approach", "ideal-rounded-left", "ideal-exit"]) {
                assert.equal(get(id).attributes.get("visibility"),
                    "visible", `manual x previews ${id}`);
            }
            assert.equal(get("ideal-approach").attributes.get("x1"),
                "2650");
            assert.equal(get("ideal-approach").attributes.get("y2"),
                "2450");
            assert.equal(get("ideal-rounded-left").attributes.get("x1"),
                "1550");
            assert.equal(get("ideal-rounded-left").attributes.get("y2"),
                "1350");
            get("construct").dispatch("click");
            assert.equal(get("preview").children.length, 6);
            get("save").dispatch("click");
            const accepted = parseGlyphLibrary(await downloads.at(-1).text());
            assert.deepEqual(accepted, geometricStarter("x", 1550,
                2450, 1100, 250, "#243647", undefined, "cursive"));
            get("circle-x").value = "1400";
            get("circle-x").dispatch("input");
            assert.equal(get("ideal-entry").attributes.get("visibility"),
                "hidden");
            get("construct").dispatch("click");
            get("save").dispatch("click");
            assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
                accepted, "invalid x leaves accepted glyph ink unchanged");
            editor.reset();
            assert.equal(get("ideal-rounded-left").attributes
                .get("visibility"), "hidden");
        } finally {
            ui.restore();
        }
    });

// The manual g keeps the exact circle and its independently sampled hook.
// A second lower semicircle carries the pen back to the right stem.
test("manual cursive g connects a complete bowl to its original hook", () => {
    const args = [1550, 2450, 1100, 250, "#243647"];
    const printed = geometricStarter("g", ...args);
    const joined = geometricStarter("g", ...args, undefined, "cursive");
    assert.equal(printed.length, 65);
    assert.equal(joined.length, 91);
    assert.deepEqual(joined.slice(1, 25), printed.slice(24, 48));
    assert.deepEqual(joined.slice(25, 49), printed.slice(0, 24));
    assert.deepEqual(joined.slice(49, 73), printed.slice(24, 48));
    assert.deepEqual(joined.slice(73, 90), printed.slice(48));
    assert.deepEqual(joined[90], {
        grapheme: "g", x1: 2100, y1: 4045,
        x2: 2300, y2: 3845, width: 250, color: "#243647",
    });
    assert.deepEqual(candidatePenPaths(joined, "g").map(
        ({ first, last }) => [first, last]), [[1, 91]]);
    assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(joined)), joined);
    assert.deepEqual(geometricStarter("g", ...args), printed);
    assert.equal(geometricStarter("g", 1400, 2450,
        1100, 250, "#243647").length, 65);
    assert.throws(() => geometricStarter("g", 1400, 2450,
        1100, 250, "#243647", undefined, "cursive"),
    /Cursive g connector and pen clip/);
    assert.throws(() => geometricStarter("G", ...args, undefined,
        "cursive"), /Cursive variant/);
});

test("browser manual g shows hook exit and retains clipped-out TSV",
    async () => {
        const ui = mountEditor();
        try {
            const { editor, get, downloads } = ui;
            editor.start();
            get("form").value = "g-cursive";
            get("form").dispatch("change");
            get("character").value = "g";
            get("character").dispatch("input");
            for (const id of ["ideal-circle", "ideal-stem",
                "ideal-rounded", "ideal-entry", "ideal-exit"]) {
                assert.equal(get(id).attributes.get("visibility"),
                    "visible", `manual g previews ${id}`);
            }
            assert.equal(get("ideal-approach").attributes.get("visibility"),
                "hidden");
            assert.equal(get("ideal-exit").attributes.get("x1"), "2100");
            assert.equal(get("ideal-exit").attributes.get("y1"), "4045");
            get("construct").dispatch("click");
            assert.equal(get("preview").children.length, 91);
            get("save").dispatch("click");
            const accepted = parseGlyphLibrary(await downloads.at(-1).text());
            assert.deepEqual(accepted, geometricStarter("g", 1550,
                2450, 1100, 250, "#243647", undefined, "cursive"));
            get("circle-x").value = "1400";
            get("circle-x").dispatch("input");
            assert.equal(get("ideal-entry").attributes.get("visibility"),
                "hidden");
            get("construct").dispatch("click");
            get("save").dispatch("click");
            assert.deepEqual(parseGlyphLibrary(await downloads.at(-1).text()),
                accepted, "clipped g cannot overwrite accepted glyph ink");
            editor.reset();
            assert.equal(get("ideal-exit").attributes.get("visibility"),
                "hidden");
        } finally {
            ui.restore();
        }
    });

// Destructive glyph-wide edits must enter the same eight-step history as a
// pen trace; one exact Unicode key must never remove another key's ink.
test("clear exact glyph is one reversible isolated edit, including empty draft",
    async () => {
        const ui = mountEditor();
        try {
            const { editor, get, downloads } = ui;
            const nfd = "n\u0303";
            const clear = get("clear");
            const undo = get("slant-undo");
            const redo = get("slant-redo");
            const visit = (glyph) => {
                get("character").value = glyph;
                get("character").dispatch("input");
            };
            const saved = async () => {
                get("save").dispatch("click");
                return await downloads.at(-1).text();
            };
            assert.equal(clear.disabled, true, "inactive action is disabled");
            clear.dispatch("click");
            editor.start();
            assert.equal(clear.disabled, true, "unwritten glyph cannot clear");
            for (const [glyph, x] of [[nfd, 100], ["ñ", 200],
                [nfd, 300], ["∞", 400]]) {
                visit(glyph);
                get("x1").value = String(x);
                get("add").dispatch("click");
            }
            const original = await saved();
            const expected = parseGlyphLibrary(original).filter(
                ({ grapheme }) => grapheme !== nfd);
            visit("m");
            clear.dispatch("click");
            visit(nfd);
            assert.match(get("recovery-status").textContent,
                /Session edits: 4\/4 applied/,
                "clearing missing ink cannot add a history step");
            // An unfinished pointer trace must not silently delete authored
            // ink or interrupt the current drawing gesture.
            const pointer = (clientX, clientY) => ({
                clientX, clientY, pointerId: 13, isPrimary: true, button: 0,
            });
            get("preview").dispatch("pointerdown", pointer(10, 20));
            assert.equal(clear.disabled, true);
            clear.dispatch("click");
            get("preview").dispatch("pointercancel", { pointerId: 13 });
            assert.equal(clear.disabled, false);
            assert.equal(await saved(), original);
            assert.equal(clear.disabled, false);
            const downloadedBefore = downloads.length;
            clear.dispatch("click");
            assert.equal(downloads.length, downloadedBefore,
                "removing ink cannot write a TSV without a download gesture");
            assert.equal(get("preview").children.length, 0);
            assert.equal(get("selected").disabled, true);
            assert.equal(clear.disabled, true, "nothing remains for exact key");
            assert.match(get("status").textContent, /Cleared 2 strokes/);
            assert.equal(undo.textContent, "Undo last glyph clear");
            assert.deepEqual(parseGlyphLibrary(await saved()), expected);

            visit("ñ");
            assert.equal(undo.disabled, true,
                "another normalised-visible grapheme cannot undo NFD");
            clear.dispatch("click"); // This button is valid for ñ now.
            assert.equal(undo.textContent, "Undo last glyph clear");
            assert.deepEqual(parseGlyphLibrary(await saved()),
                expected.filter(({ grapheme }) => grapheme !== "ñ"));
            undo.dispatch("click");
            assert.deepEqual(parseGlyphLibrary(await saved()), expected);
            get("recovery-undo-select").dispatch("click");
            assert.equal(get("character").value, nfd,
                "navigation preserves exact Unicode identity");
            undo.dispatch("click");
            assert.equal(await saved(), original,
                "undo reconstructs interleaved rows and original TSV bytes");
            visit("m");
            clear.dispatch("click");
            visit(nfd);
            assert.equal(redo.disabled, false,
                "missing-glyph removal must not truncate the redo branch");
            redo.dispatch("click");
            assert.deepEqual(parseGlyphLibrary(await saved()), expected);

            visit("ñ");
            clear.dispatch("click");
            visit("∞");
            clear.dispatch("click");
            assert.equal(get("preview").children.length, 0);
            const beforeRefusal = downloads.length;
            get("save").dispatch("click");
            assert.equal(downloads.length, beforeRefusal,
                "an empty in-memory library must not download invalid TSV");
            assert.match(get("status").textContent,
                /Save requires 1\.\.4096 authored glyph strokes/);
            assert.equal(undo.textContent, "Undo last glyph clear");
            undo.dispatch("click");
            assert.deepEqual(parseGlyphLibrary(await saved()),
                expected.filter(({ grapheme }) => grapheme === "∞"));
            redo.dispatch("click");
            assert.equal(clear.disabled, true);
            editor.reset();
            assert.equal(undo.disabled, true);
            assert.equal(redo.disabled, true);
            editor.start();
            assert.equal(clear.disabled, true);
        } finally {
            ui.restore();
        }
    });


test("whole-glyph reflection preserves directed exact-Unicode pen paths",
    () => {
    const nfd = "o\u0301";
    const source = [
        { ...stroke(nfd), x1: 450, y1: 1000, x2: 850, y2: 1600,
            width: 251, color: "#243647" },
        { ...stroke("ó"), x1: 200, y1: 1000, x2: 550, y2: 1500 },
        { ...stroke(nfd), x1: 850, y1: 1600, x2: 1300, y2: 2200,
            width: 251, color: "#243647" },
        { ...stroke("∞"), x1: 300, y1: 1700, x2: 500, y2: 1900 },
    ];
    const original = structuredClone(source);
    const leftRight = reflectGlyphStrokes(source, nfd, "x", 1750);
    assert.deepEqual(leftRight.map((line) => line.grapheme),
        [nfd, "ó", nfd, "∞"]);
    assert.equal(leftRight[0].x1, 3050);
    assert.equal(leftRight[0].x2, 2650);
    assert.equal(leftRight[2].x1, 2650);
    assert.equal(leftRight[2].y1, leftRight[0].y2);
    assert.equal(leftRight[2].x1, leftRight[0].x2);
    assert.equal(leftRight[0].width, 251);
    assert.deepEqual(leftRight[1], source[1]);
    assert.deepEqual(leftRight[3], source[3]);
    assert.deepEqual(reflectGlyphStrokes(leftRight, nfd, "x", 1750),
        original, "twice across the same axis returns every byte");
    const upDown = reflectGlyphStrokes(source, nfd, "y", 2250);
    assert.equal(upDown[0].y1, 3500);
    assert.equal(upDown[0].y2, 2900);
    assert.equal(upDown[2].y1, upDown[0].y2);
    assert.equal(upDown[0].x1, source[0].x1);
    assert.deepEqual(reflectGlyphStrokes(upDown, nfd, "y", 2250),
        original);
    assert.equal(encodeGlyphLibrary(leftRight),
        encodeGlyphLibrary(parseGlyphLibrary(encodeGlyphLibrary(leftRight))));
    assert.deepEqual(source, original);
    for (const [glyph, axis, pivot] of [
        [nfd, "x", 400], [nfd, "y", 400],
        [nfd, "x", 3501], [nfd, "y", 4501],
        [nfd, "x", 1.5], [nfd, "x", -1],
        [nfd, "horizontal", 1750],
        ["ñ", "x", 1750], ["ab", "x", 1750],
    ]) {
        assert.throws(() => reflectGlyphStrokes(source,
            glyph, axis, pivot),
        "invalid axis, selected Unicode or pivot must refuse");
    }
    const full = Array.from({ length: 4096 }, (_, i) => ({
        ...stroke(nfd), x1: 1400, y1: 1600,
        x2: 1600 + i % 100, y2: 1700,
    }));
    assert.equal(reflectGlyphStrokes(full, nfd, "x", 1750).length,
        4096, "bounded full-size libraries remain editable");
});

test("whole-glyph reflection preview is opt-in with bounded undo and TSV",
    async () => {
    const ui = mountEditor();
    try {
        const { get, editor, downloads } = ui;
        const nfd = "o\u0301";
        const preview = get("reflect-preview");
        const status = get("reflect-preview-status");
        assert.equal(get("reflect-all").disabled, true);
        editor.start();
        get("character").value = nfd;
        get("character").dispatch("input");
        for (const [key, value] of Object.entries({
            x1: 450, y1: 1000, x2: 850, y2: 1600, width: 251,
        })) get(key).value = String(value);
        get("add").dispatch("click");
        const original = [{ grapheme: nfd, x1: 450, y1: 1000,
            x2: 850, y2: 1600, width: 251, color: "#243647" }];
        assert.equal(get("reflect-all").disabled, false);
        assert.equal(preview.children.length, 1);
        assert.equal(Number(preview.children[0].attributes.get("x1")),
            3050);
        assert.match(status.textContent, /preview only, draft unchanged/);
        assert.equal(downloads.length, 0);
        get("reflect-pivot").value = "400";
        get("reflect-pivot").dispatch("input");
        assert.equal(preview.children.length, 0);
        assert.match(status.textContent, /clips glyph cell/);
        get("reflect-all").dispatch("click");
        assert.match(get("status").textContent, /clips glyph cell/);
        get("reflect-pivot").value = "1.75e3";
        get("reflect-pivot").dispatch("input");
        assert.equal(preview.children.length, 0);
        get("reflect-all").dispatch("click");
        assert.match(get("status").textContent, /whole decimal/);
        get("reflect-pivot").value = "1750";
        get("reflect-pivot").dispatch("input");
        get("reflect-all").dispatch("click");
        const horizontally = reflectGlyphStrokes(original, nfd,
            "x", 1750);
        assert.match(get("status").textContent, /Reflected all authored/);
        assert.equal(downloads.length, 0);
        get("save").dispatch("click");
        assert.equal(await downloads.at(-1).text(),
            encodeGlyphLibrary(horizontally));
        assert.equal(get("slant-undo").textContent,
            "Undo last glyph reflection");
        get("slant-undo").dispatch("click");
        get("save").dispatch("click");
        assert.equal(await downloads.at(-1).text(),
            encodeGlyphLibrary(original));
        get("slant-redo").dispatch("click");
        get("save").dispatch("click");
        assert.equal(await downloads.at(-1).text(),
            encodeGlyphLibrary(horizontally));
        get("reflect-axis").value = "y";
        get("reflect-axis").dispatch("change");
        get("reflect-pivot").value = "2250";
        get("reflect-pivot").dispatch("input");
        const both = reflectGlyphStrokes(horizontally, nfd, "y", 2250);
        assert.equal(Number(preview.children[0].attributes.get("y1")),
            both[0].y1);
        get("reflect-all").dispatch("click");
        get("save").dispatch("click");
        assert.equal(await downloads.at(-1).text(), encodeGlyphLibrary(both));
        get("slant-undo").dispatch("click");
        get("save").dispatch("click");
        assert.equal(await downloads.at(-1).text(),
            encodeGlyphLibrary(horizontally));
        editor.reset();
        assert.equal(get("reflect-all").disabled, true);
        assert.equal(preview.children.length, 0);
        assert.equal(get("slant-redo").disabled, true);
    } finally {
        ui.restore();
    }
});


test("subdivision keeps ordered directed ink and exact Unicode boundaries",
    () => {
    const nfd = "n\u0303";
    const source = [
        { ...stroke(nfd), x1: 1000, y1: 900, x2: 1601, y2: 1702,
            width: 251, color: "#112233" },
        { ...stroke("ñ"), x1: 400, y1: 800, x2: 500, y2: 1200 },
        { ...stroke(nfd), x1: 1601, y1: 1702, x2: 2000, y2: 2200,
            width: 251, color: "#112233" },
        { ...stroke("∞"), x1: 600, y1: 600, x2: 900, y2: 900 },
    ];
    const before = structuredClone(source);
    const result = subdivideGlyphStroke(source, nfd, 0);
    assert.deepEqual(result.map((item) => item.grapheme),
        [nfd, nfd, "ñ", nfd, "∞"]);
    assert.deepEqual([result[0].x1, result[0].y1,
        result[0].x2, result[0].y2], [1000, 900, 1301, 1301]);
    assert.deepEqual([result[1].x1, result[1].y1,
        result[1].x2, result[1].y2], [1301, 1301, 1601, 1702]);
    assert.equal(result[0].width, 251);
    assert.equal(result[1].width, 251);
    assert.equal(result[0].color, "#112233");
    assert.equal(result[1].color, "#112233");
    assert.deepEqual(result.slice(2), source.slice(1));
    assert.deepEqual(candidatePenPaths(result, nfd).map(({ first, last }) =>
        [first, last]), [[1, 3]],
    "the inserted seam must not create an inferred pen lift");
    const twice = subdivideGlyphStroke(result, nfd, 1);
    assert.equal(twice[1].x2, twice[2].x1);
    assert.equal(twice[1].y2, twice[2].y1);
    assert.equal(encodeGlyphLibrary(result),
        encodeGlyphLibrary(parseGlyphLibrary(encodeGlyphLibrary(result))));
    assert.deepEqual(source, before, "pure edit cannot mutate source");
    for (const [key, index] of [
        ["ñ", 0], ["n", 0], [nfd, 1], [nfd, -1],
        [nfd, 0.5], [nfd, 4], ["ab", 0],
    ]) {
        assert.throws(() => subdivideGlyphStroke(source, key, index));
    }
    for (const tiny of [
        { x1: 1000, y1: 1200, x2: 1001, y2: 1200 },
        { x1: 1000, y1: 1200, x2: 1000, y2: 1201 },
    ]) {
        assert.throws(() => subdivideGlyphStroke([
            { ...source[0], ...tiny },
        ], nfd, 0), /too short/);
    }
    const atLimit = Array.from({ length: 4096 }, () => source[0]);
    assert.throws(() => subdivideGlyphStroke(atLimit, nfd, 0),
        /4096/, "native stroke count cannot overflow");
    const nearLimit = atLimit.slice(1);
    assert.equal(subdivideGlyphStroke(nearLimit, nfd, 0).length, 4096);
});

test("selected stroke subdivision is one recoverable, browser-only edit",
    async () => {
    const ui = mountEditor();
    try {
        const { get, editor, downloads } = ui;
        const nfd = "n\u0303";
        const split = get("subdivide");
        const choose = (glyph, index) => {
            get("character").value = glyph;
            get("character").dispatch("input");
            get("selected").value = String(index);
            get("selected").dispatch("change");
        };
        const add = (glyph, x1, y1, x2, y2) => {
            get("character").value = glyph;
            get("character").dispatch("input");
            for (const [key, value] of Object.entries({
                x1, y1, x2, y2, width: 251,
            })) get(key).value = String(value);
            get("add").dispatch("click");
        };
        assert.equal(split.disabled, true);
        editor.start();
        add(nfd, 1000, 900, 1601, 1702);
        add("ñ", 400, 800, 500, 1200);
        add(nfd, 1601, 1702, 2000, 2200);
        choose(nfd, 0);
        assert.equal(split.disabled, false);
        get("save").dispatch("click");
        const original = await downloads.at(-1).text();
        const count = downloads.length;
        split.dispatch("click");
        assert.equal(downloads.length, count,
            "split must not implicitly write a TSV");
        const splitLines = parseGlyphLibrary(original);
        const expected = subdivideGlyphStroke(splitLines, nfd, 0);
        assert.equal(get("selected").value, "1",
            "new trailing half is editable after insertion");
        assert.equal(get("x1").value, "1301");
        assert.equal(get("y1").value, "1301");
        assert.match(get("paths").textContent, /strokes 1–3/);
        assert.equal(get("slant-undo").textContent,
            "Undo last stroke subdivision");
        choose("ñ", 2);
        assert.equal(get("slant-undo").disabled, true,
            "NFC selection cannot recover the NFD edit");
        choose(nfd, 1);
        get("save").dispatch("click");
        assert.equal(await downloads.at(-1).text(),
            encodeGlyphLibrary(expected));
        get("slant-undo").dispatch("click");
        assert.equal(get("selected").value, "0",
            "undo restores the original selected stroke");
        get("save").dispatch("click");
        assert.equal(await downloads.at(-1).text(), original);
        get("slant-redo").dispatch("click");
        assert.equal(get("selected").value, "1");
        get("save").dispatch("click");
        assert.equal(await downloads.at(-1).text(),
            encodeGlyphLibrary(expected));
        add(nfd, 1100, 3000, 1101, 3000);
        choose(nfd, 4);
        assert.equal(split.disabled, true,
            "one-unit segment has no strict integer midpoint");
        const refused = downloads.length;
        split.dispatch("click");
        assert.equal(downloads.length, refused);
        assert.equal(get("slant-undo").textContent,
            "Undo last stroke addition");
        editor.reset();
        assert.equal(split.disabled, true);
        assert.equal(get("slant-undo").disabled, true);
    } finally {
        ui.restore();
    }
});


test("lossless stroke merge preserves forward ink and exact Unicode TSV slots",
    () => {
    const nfd = "n\u0303";
    const first = { ...stroke(nfd), x1: 400, y1: 600,
        x2: 800, y2: 1000, width: 251, color: "#112233" };
    const second = { ...first, x1: 800, y1: 1000,
        x2: 1600, y2: 1800, color: "#112233" };
    const other = stroke("ñ");
    const more = { ...stroke(nfd), x1: 2200, y1: 2300,
        x2: 2800, y2: 2900 };
    const source = [first, other, second, stroke("∞"), more];
    const before = structuredClone(source);
    const merged = mergeGlyphStrokes(source, nfd, 0);
    assert.deepEqual(merged, [
        { ...first, x2: 1600, y2: 1800 }, other, stroke("∞"), more,
    ], "next glyph-order segment can be interleaved with other glyphs");
    assert.deepEqual(source, before, "source remains unchanged");
    assert.equal(encodeGlyphLibrary(merged),
        encodeGlyphLibrary(parseGlyphLibrary(encodeGlyphLibrary(merged))));
    assert.deepEqual(candidatePenPaths(merged, nfd), [
        { first: 1, last: 1, start: { x: 400, y: 600 },
            end: { x: 1600, y: 1800 }, width: 251,
            color: "#112233" },
        { first: 2, last: 2, start: { x: 2200, y: 2300 },
            end: { x: 2800, y: 2900 }, width: 250,
            color: "#A1B2C3" },
    ]);
    assert.deepEqual(mergeGlyphStrokes([
        first, { ...second, color: "#112233" },
    ], nfd, 0), [{ ...first, x2: 1600, y2: 1800 }]);
    const even = { ...first, x1: 1000, y1: 1000,
        x2: 2000, y2: 3000 };
    const halves = subdivideGlyphStroke([even], nfd, 0);
    assert.deepEqual(mergeGlyphStrokes(halves, nfd, 0), [even],
        "exact integer midpoint subdivision admits a lossless inverse");
    assert.deepEqual(mergeGlyphStrokes([
        { ...first, color: "#a1b2c3" },
        { ...second, color: "#A1B2C3" },
    ], nfd, 0), [{ ...first, color: "#A1B2C3",
        x2: 1600, y2: 1800 }], "RGB comparison is canonical");
});

test("stroke merge refuses non-lossless seams and unadmitted sources", () => {
    const nfd = "n\u0303";
    const first = { ...stroke(nfd), x1: 500, y1: 500,
        x2: 1000, y2: 1500 };
    const second = { ...first, x1: 1000, y1: 1500,
        x2: 1500, y2: 2500 };
    const good = [first, stroke("ñ"), second];
    for (const [glyph, index] of [["ñ", 0], ["n", 0],
        [nfd, 1], [nfd, 2], [nfd, -1], [nfd, 0.5], ["ab", 0]]) {
        assert.throws(() => mergeGlyphStrokes(good, glyph, index));
    }
    const refused = [
        { ...second, x1: 1001 }, // one micrometre gap
        { ...second, x2: 1501 }, // turn, not a straight path
        { ...second, x2: 500, y2: 500 }, // backtrack
        { ...second, width: 260 }, // different round pen
        { ...second, color: "#FFFFFF" }, // different pigment
    ];
    for (const next of refused) {
        const candidate = [first, stroke("ñ"), next];
        const before = encodeGlyphLibrary(candidate);
        assert.throws(() => mergeGlyphStrokes(candidate, nfd, 0),
            /touching, straight, co-directed/);
        assert.equal(encodeGlyphLibrary(candidate), before,
            "rejected merge cannot mutate caller ink");
    }
    const roundedOffLine = subdivideGlyphStroke([
        { ...first, x1: 1000, y1: 900, x2: 1601, y2: 1702 },
    ], nfd, 0);
    assert.throws(() => mergeGlyphStrokes(roundedOffLine, nfd, 0),
        /straight/, "rounded off-line split cannot be called lossless");
    assert.throws(() => mergeGlyphStrokes([
        first, { ...second, width: 79 },
    ], nfd, 0), /bounds/, "unadmitted unrelated pen data refuses");
    const atLimit = Array.from({ length: 4096 }, (_, i) => ({
        ...first, x1: 500 + (i % 3), x2: 1000 + (i % 3),
    }));
    atLimit[0] = first;
    atLimit[1] = second;
    assert.equal(mergeGlyphStrokes(atLimit, nfd, 0).length, 4095,
        "merging can reduce an exact full-cap library");
});

test("merge control offers atomic exact-Unicode recovery and no implicit save",
    async () => {
    const ui = mountEditor();
    try {
        const { get, editor, downloads } = ui;
        const nfd = "n\u0303";
        const merge = get("merge");
        const choose = (glyph, index) => {
            get("character").value = glyph;
            get("character").dispatch("input");
            get("selected").value = String(index);
            get("selected").dispatch("change");
        };
        const add = (glyph, x1, y1, x2, y2) => {
            get("character").value = glyph;
            get("character").dispatch("input");
            for (const [key, value] of Object.entries({
                x1, y1, x2, y2, width: 251,
            })) get(key).value = String(value);
            get("add").dispatch("click");
        };
        assert.equal(merge.disabled, true);
        editor.start();
        add(nfd, 1000, 1000, 1600, 1600);
        add("ñ", 300, 500, 700, 900);
        add(nfd, 1600, 1600, 2200, 2200);
        choose(nfd, 0);
        assert.equal(merge.disabled, false);
        get("save").dispatch("click");
        const original = await downloads.at(-1).text();
        const previousDownloads = downloads.length;
        merge.dispatch("click");
        assert.equal(downloads.length, previousDownloads,
            "authoring must never silently commit a file");
        const expected = mergeGlyphStrokes(parseGlyphLibrary(original),
            nfd, 0);
        assert.equal(get("selected").value, "0");
        assert.equal(get("x2").value, "2200");
        assert.match(get("paths").textContent, /1 directed strokes/);
        assert.equal(get("slant-undo").textContent,
            "Undo last stroke merge");
        choose("ñ", 1);
        assert.equal(get("slant-undo").disabled, true,
            "NFC selection cannot undo NFD editing");
        choose(nfd, 0);
        get("save").dispatch("click");
        assert.equal(await downloads.at(-1).text(),
            encodeGlyphLibrary(expected));
        assert.equal(merge.disabled, true,
            "no following NFD segment remains");
        get("slant-undo").dispatch("click");
        assert.equal(get("selected").value, "0");
        get("save").dispatch("click");
        assert.equal(await downloads.at(-1).text(), original);
        get("slant-redo").dispatch("click");
        get("save").dispatch("click");
        assert.equal(await downloads.at(-1).text(),
            encodeGlyphLibrary(expected));
        get("subdivide").dispatch("click");
        choose(nfd, 0);
        assert.equal(merge.disabled, false,
            "exact midpoint split can be restored by merge");
        merge.dispatch("click");
        get("save").dispatch("click");
        assert.equal(await downloads.at(-1).text(),
            encodeGlyphLibrary(expected));
        editor.reset();
        assert.equal(merge.disabled, true);
        assert.equal(get("slant-undo").disabled, true);
    } finally {
        ui.restore();
    }
});

test("exact lattice subdivision and merge are byte-identical on angled ink",
    () => {
    const nfd = "n\u0303";
    const angled = { ...stroke(nfd), x1: 1000, y1: 900,
        x2: 1603, y2: 1704, width: 251, color: "#123456" };
    const reversed = { ...angled, x1: 3200, y1: 3500,
        x2: 2597, y2: 2696 };
    const original = [angled, stroke("ñ"), reversed, stroke("∞")];
    const bytes = encodeGlyphLibrary(original);
    const rounded = subdivideGlyphStroke([angled], nfd, 0);
    assert.throws(() => mergeGlyphStrokes(rounded, nfd, 0),
        /straight/, "rounded sample changes the original contour");
    const split = splitGlyphStrokeExactly(original, nfd, 0);
    assert.deepEqual(split.slice(0, 2), [
        { ...angled, x2: 1303, y2: 1304 },
        { ...angled, x1: 1303, y1: 1304 },
    ], "odd lattice step chooses an exact upper-middle point");
    assert.deepEqual(split[2], original[1], "interleaved NFC stays ordered");
    assert.equal(encodeGlyphLibrary(mergeGlyphStrokes(split, nfd, 0)),
        bytes, "all authored TSV bytes return after an exact split/merge");
    const negative = splitGlyphStrokeExactly(original, nfd, 2);
    assert.deepEqual(negative.slice(2, 4), [
        { ...reversed, x2: 2897, y2: 3096 },
        { ...reversed, x1: 2897, y1: 3096 },
    ], "reversed diagonal retains direction and exact collinearity");
    assert.equal(encodeGlyphLibrary(mergeGlyphStrokes(negative, nfd, 2)),
        bytes, "negative coordinate travel must be reversible as well");
    assert.deepEqual(original, [angled, stroke("ñ"), reversed, stroke("∞")],
        "pure editing cannot mutate authored caller data");
    assert.equal(candidatePenPaths(split, nfd)[0].last, 2);
    for (const line of [
        { ...angled, x2: 1601, y2: 1702 },
        { ...angled, x2: 1001, y2: 900 },
        { ...angled, x2: 1000, y2: 901 },
    ]) {
        const source = [line, stroke("ñ")];
        const before = encodeGlyphLibrary(source);
        assert.throws(() => splitGlyphStrokeExactly(source, nfd, 0),
            /strict integer point/);
        assert.equal(encodeGlyphLibrary(source), before);
    }
    assert.throws(() => splitGlyphStrokeExactly(original, "ñ", 0),
        /selected exact glyph/);
    assert.throws(() => splitGlyphStrokeExactly(original, nfd, 0.5));
    assert.throws(() => splitGlyphStrokeExactly(original, "ab", 0));
    const full = Array.from({ length: 4096 }, () => angled);
    assert.throws(() => splitGlyphStrokeExactly(full, nfd, 0), /4096/);
});

test("lossless split uses one session-only undo and never auto-downloads",
    async () => {
    const ui = mountEditor();
    try {
        const { get, editor, downloads } = ui;
        const nfd = "n\u0303";
        const select = (glyph, index) => {
            get("character").value = glyph;
            get("character").dispatch("input");
            get("selected").value = String(index);
            get("selected").dispatch("change");
        };
        const add = (glyph, x1, y1, x2, y2) => {
            get("character").value = glyph;
            get("character").dispatch("input");
            for (const [axis, value] of Object.entries({
                x1, y1, x2, y2, width: 251,
            })) get(axis).value = String(value);
            get("add").dispatch("click");
        };
        const exact = get("split-exact");
        assert.equal(exact.disabled, true);
        editor.start();
        add(nfd, 1000, 900, 1603, 1704);
        add("ñ", 500, 700, 800, 900);
        select(nfd, 0);
        assert.equal(exact.disabled, false);
        get("save").dispatch("click");
        const original = await downloads.at(-1).text();
        const count = downloads.length;
        exact.dispatch("click");
        assert.equal(downloads.length, count,
            "accepted authoring cannot implicitly export a file");
        assert.equal(get("selected").value, "1",
            "new second segment is selected for further retuning");
        assert.equal(get("x1").value, "1303");
        assert.equal(get("y1").value, "1304");
        assert.equal(get("slant-undo").textContent,
            "Undo last lossless stroke split");
        select(nfd, 1);
        get("save").dispatch("click");
        const split = await downloads.at(-1).text();
        assert.equal(split, encodeGlyphLibrary(splitGlyphStrokeExactly(
            parseGlyphLibrary(original), nfd, 0)));
        select("ñ", 2);
        assert.equal(get("slant-undo").disabled, true,
            "other Unicode shape cannot recover this exact edit");
        select(nfd, 1);
        get("slant-undo").dispatch("click");
        get("save").dispatch("click");
        assert.equal(await downloads.at(-1).text(), original);
        get("slant-redo").dispatch("click");
        select(nfd, 0);
        assert.equal(get("merge").disabled, false);
        get("merge").dispatch("click");
        get("save").dispatch("click");
        assert.equal(await downloads.at(-1).text(), original,
            "native TSV source bytes survive an exact split/merge");
        add(nfd, 1000, 900, 1601, 1702);
        select(nfd, 2);
        assert.equal(exact.disabled, true,
            "primitive line has no interior lattice point");
        editor.reset();
        assert.equal(exact.disabled, true);
        assert.equal(get("slant-undo").disabled, true);
    } finally {
        ui.restore();
    }
});

test("selected path scale preserves Unicode slots, joints and negative ties",
    () => {
        const nfd = "n\u0303";
        const a = { ...stroke(nfd), x1: 500, y1: 700,
            x2: 900, y2: 1200, width: 251, color: "#112233" };
        const b = { ...a, x1: 900, y1: 1200,
            x2: 1400, y2: 1900 };
        const other = stroke("ñ");
        const accent = { ...a, x1: 2200, y1: 2500,
            x2: 2700, y2: 3000, width: 200, color: "#445566" };
        const original = [a, other, b, stroke("∞"), accent];
        const originalBytes = encodeGlyphLibrary(original);
        const changed = scaleCandidatePath(original, nfd, 2, 125, 75);
        const expected = [{ ...a, x2: 1000, y2: 1075 },
            other, { ...b, x1: 1000, y1: 1075,
                x2: 1625, y2: 1600 }, original[3], accent];
        assert.deepEqual(changed, { strokes: expected, selectedIndex: 2 });
        assert.deepEqual(candidatePenPaths(changed.strokes, nfd).map((path) =>
            [path.first, path.last]), [[1, 2], [3, 3]]);
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(changed.strokes)),
            expected);
        assert.equal(encodeGlyphLibrary(original), originalBytes);
        const negative = { ...stroke("á"), x1: 1000, y1: 1500,
            x2: 999, y2: 1499 };
        assert.deepEqual(scaleCandidatePath([negative], "á", 0,
            50, 125).strokes, [
            { ...negative, x2: 1000, y2: 1499 },
        ]);
        const shortNegative = { ...negative, y2: 1500 };
        assert.throws(() => scaleCandidatePath([shortNegative],
            "á", 0, 50, 100), /endpoints must differ/i);
        const shortPositive = { ...negative, x2: 1001, y2: 1500 };
        assert.throws(() => scaleCandidatePath([shortPositive],
            "á", 0, 101, 100), /does not change/);
        const nearEdge = { ...a, x2: 3000 };
        for (const [input, selected, x, y] of [
            [original, 2, 100, 100], [original, 2, 49, 100],
            [original, 2, 100, 151], [original, 2, 125.5, 100],
            [original, -1, 125, 100], [original, 1, 125, 100],
            [original, 2, Infinity, 100],
            [[nearEdge], 0, 125, 100],
        ]) {
            assert.throws(() => scaleCandidatePath(input,
                nfd, selected, x, y), /path|selected|clip|bounds/i);
        }
        assert.equal(encodeGlyphLibrary(original), originalBytes);
    });

test("selected path scale UI is undoable and exports only accepted ink",
    async () => {
        const ui = mountEditor();
        try {
            const { editor, get, downloads } = ui;
            const nfd = "n\u0303";
            const a = { ...stroke(nfd), x1: 500, y1: 700,
                x2: 900, y2: 1200, width: 251, color: "#112233" };
            const b = { ...a, x1: 900, y1: 1200,
                x2: 1400, y2: 1900 };
            const original = [a, stroke("ñ"), b, stroke("∞")];
            const bytes = new TextEncoder().encode(
                encodeGlyphLibrary(original));
            assert.equal(get("path-scale").disabled, true);
            editor.start();
            get("load").files = [{ size: bytes.length,
                arrayBuffer: async () => bytes.buffer }];
            await get("load").dispatch("change");
            get("character").value = nfd;
            get("character").dispatch("input");
            get("selected").value = "2";
            get("selected").dispatch("change");
            assert.equal(get("path-scale").disabled, false);
            assert.equal(get("path-scale-x").disabled, false);
            assert.equal(get("path-scale-y").disabled, false);
            get("path-scale-x").value = "125";
            get("path-scale-y").value = "75";
            get("path-scale").dispatch("click");
            assert.match(get("status").textContent,
                /Scaled selected continuous pen path; unsaved/);
            get("save").dispatch("click");
            const changed = scaleCandidatePath(original, nfd, 2, 125, 75);
            assert.equal(await downloads[0].text(),
                encodeGlyphLibrary(changed.strokes));
            get("slant-undo").dispatch("click");
            assert.match(get("status").textContent,
                /Undid last pen path scale/);
            get("save").dispatch("click");
            assert.equal(await downloads[1].text(),
                encodeGlyphLibrary(original));
            get("slant-redo").dispatch("click");
            get("save").dispatch("click");
            assert.equal(await downloads[2].text(),
                encodeGlyphLibrary(changed.strokes));
            get("path-scale-x").value = "200";
            get("path-scale-y").value = "150";
            get("path-scale").dispatch("click");
            assert.match(get("status").textContent, /50–150/);
            get("save").dispatch("click");
            assert.equal(await downloads[3].text(),
                encodeGlyphLibrary(changed.strokes));
            editor.reset();
            assert.equal(get("path-scale").disabled, true);
            assert.equal(get("path-scale-x").disabled, true);
            assert.equal(get("path-scale-y").disabled, true);
        } finally {
            ui.restore();
        }
    });

test("path scaling preserves closed contour joints and full 4096-stroke budget",
    () => {
        const glyph = "n\u0303";
        const base = { ...stroke(glyph), x1: 500, y1: 700,
            x2: 900, y2: 700, width: 251, color: "#112233" };
        const top = { ...base, x1: 900, y1: 700, x2: 900, y2: 1000 };
        const bottom = { ...base, x1: 900, y1: 1000,
            x2: 500, y2: 1000 };
        const left = { ...base, x1: 500, y1: 1000,
            x2: 500, y2: 700 };
        const other = stroke("ñ");
        const original = [base, other, top, bottom, left];
        const result = scaleCandidatePath(original, glyph, 3, 125, 75);
        assert.deepEqual(result.strokes, [
            { ...base, x2: 1000 }, other,
            { ...top, x1: 1000, x2: 1000, y2: 925 },
            { ...bottom, x1: 1000, y1: 925, y2: 925 },
            { ...left, y1: 925 },
        ]);
        assert.deepEqual(candidatePenPaths(result.strokes, glyph).map(
            ({ first, last, start, end }) => [first, last, start, end]),
        [[1, 4, { x: 500, y: 700 }, { x: 500, y: 700 }]]);
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(result.strokes)),
            result.strokes);
        assert.deepEqual(original, [base, other, top, bottom, left]);
        const large = Array.from({ length: 4096 }, (_, index) => ({
            ...base, x1: 500 + (index % 2) * 2,
            x2: 500 + ((index + 1) % 2) * 2,
        }));
        const before = encodeGlyphLibrary(large);
        const scaled = scaleCandidatePath(large, glyph, 4095, 150, 100);
        assert.equal(scaled.selectedIndex, 4095);
        assert.equal(scaled.strokes.length, 4096);
        assert.deepEqual(candidatePenPaths(scaled.strokes, glyph).map(
            ({ first, last }) => [first, last]), [[1, 4096]]);
        assert.equal(scaled.strokes[0].x2, 503);
        assert.equal(scaled.strokes[4095].x2, 500);
        assert.equal(encodeGlyphLibrary(large), before);
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(scaled.strokes)),
            scaled.strokes);
    });


test("selected path reflection preserves Unicode and reverses without loss",
    () => {
        const nfd = "n\u0303";
        const a = { ...stroke(nfd), x1: 500, y1: 700,
            x2: 900, y2: 1200, width: 251, color: "#112233" };
        const b = { ...a, x1: 900, y1: 1200, x2: 1400, y2: 1900 };
        const accent = { ...a, x1: 2200, y1: 2500,
            x2: 2700, y2: 3000, color: "#445566" };
        const original = [a, stroke("ñ"), b, stroke("∞"), accent];
        const source = encodeGlyphLibrary(original);
        const mirrored = reflectCandidatePath(original, nfd, 2, "x", 1750);
        assert.deepEqual(mirrored, { strokes: [
            { ...a, x1: 3000, x2: 2600 }, original[1],
            { ...b, x1: 2600, x2: 2100 }, original[3], accent,
        ], selectedIndex: 2 });
        assert.deepEqual(candidatePenPaths(mirrored.strokes, nfd).map((p) =>
            [p.first, p.last]), [[1, 2], [3, 3]]);
        assert.equal(encodeGlyphLibrary(reflectCandidatePath(
            mirrored.strokes, nfd, 2, "x", 1750).strokes), source,
        "a second mirror restores all original authored bytes");
        const vertical = reflectCandidatePath(original, nfd, 0, "y", 2250);
        assert.equal(vertical.strokes[0].y1, 3800);
        assert.equal(vertical.strokes[2].y2, 2600);
        assert.equal(vertical.strokes[1], original[1]);
        assert.equal(encodeGlyphLibrary(original), source);
        for (const [rows, key, index, axis, pivot] of [
            [original, nfd, -1, "x", 1750],
            [original, "ñ", 0, "x", 1750],
            [original, nfd, 0, "z", 1750],
            [original, nfd, 0, "x", 3501],
            [original, nfd, 0, "y", -1],
            [original, nfd, 0, "x", 0],
            [original, nfd, 0, "y", 0],
            [original, nfd, 0, "x", 1.5],
        ]) {
            assert.throws(() => reflectCandidatePath(rows, key, index,
                axis, pivot), /path|axis|pivot|clip|selected/i);
        }
        const centered = { ...a, x1: 1750, x2: 1750 };
        assert.throws(() => reflectCandidatePath([centered], nfd, 0,
            "x", 1750), /does not change/);
        const full = Array.from({ length: 4096 }, (_, index) => ({
            ...a, x1: 500 + index % 100, x2: 900,
        }));
        assert.equal(reflectCandidatePath(full, nfd, 4095,
            "x", 1750).strokes.length, 4096);
        assert.equal(encodeGlyphLibrary(original), source);
    });

test("selected path reflection UI keeps deliberate TSV and bounded undo",
    async () => {
        const ui = mountEditor();
        try {
            const { editor, get, downloads } = ui;
            const nfd = "n\u0303";
            const a = { ...stroke(nfd), x1: 500, y1: 700,
                x2: 900, y2: 1200, width: 251, color: "#112233" };
            const b = { ...a, x1: 900, y1: 1200,
                x2: 1400, y2: 1900 };
            const original = [a, stroke("ñ"), b, stroke("∞")];
            const bytes = new TextEncoder().encode(
                encodeGlyphLibrary(original));
            assert.equal(get("path-reflect").disabled, true);
            editor.start();
            get("load").files = [{ size: bytes.length,
                arrayBuffer: async () => bytes.buffer }];
            await get("load").dispatch("change");
            get("character").value = nfd;
            get("character").dispatch("input");
            get("selected").value = "2";
            get("selected").dispatch("change");
            assert.equal(get("path-reflect").disabled, false);
            assert.equal(get("path-reflect-axis").disabled, false);
            get("path-reflect-axis").value = "x";
            get("path-reflect-pivot").value = "1750";
            get("path-reflect").dispatch("click");
            assert.match(get("status").textContent,
                /Reflected selected continuous pen path; unsaved/);
            const result = reflectCandidatePath(original, nfd, 2,
                "x", 1750).strokes;
            assert.equal(downloads.length, 0,
                "editing never downloads without an explicit save");
            get("save").dispatch("click");
            assert.equal(await downloads[0].text(),
                encodeGlyphLibrary(result));
            get("slant-undo").dispatch("click");
            assert.match(get("status").textContent,
                /Undid last pen path reflection/);
            get("save").dispatch("click");
            assert.equal(await downloads[1].text(),
                encodeGlyphLibrary(original));
            get("slant-redo").dispatch("click");
            get("save").dispatch("click");
            assert.equal(await downloads[2].text(),
                encodeGlyphLibrary(result));
            get("path-reflect-pivot").value = "0";
            get("path-reflect").dispatch("click");
            assert.match(get("status").textContent, /clips/);
            get("save").dispatch("click");
            assert.equal(await downloads[3].text(),
                encodeGlyphLibrary(result));
            editor.reset();
            assert.equal(get("path-reflect").disabled, true);
            assert.equal(get("slant-undo").disabled, true);
        } finally {
            ui.restore();
        }
    });

test("selected path mirror preserves closed loops and 4096 connected ink",
    () => {
        const nfd = "n\u0303";
        const first = { ...stroke(nfd), x1: 500, y1: 700,
            x2: 900, y2: 700, width: 251, color: "#112233" };
        const top = { ...first, x1: 900, y1: 700, x2: 900, y2: 1000 };
        const bottom = { ...first, x1: 900, y1: 1000,
            x2: 500, y2: 1000 };
        const left = { ...first, x1: 500, y1: 1000,
            x2: 500, y2: 700 };
        const other = stroke("ñ");
        const original = [first, other, top, bottom, left];
        const mirrored = reflectCandidatePath(original, nfd, 3,
            "x", 1750).strokes;
        assert.deepEqual(candidatePenPaths(mirrored, nfd).map((path) =>
            [path.first, path.last, path.start, path.end]), [[
            1, 4, { x: 3000, y: 700 }, { x: 3000, y: 700 },
        ]]);
        assert.equal(mirrored[1], other,
            "unselected NFC stays at its exact interleaved TSV position");
        assert.equal(encodeGlyphLibrary(reflectCandidatePath(
            mirrored, nfd, 3, "x", 1750).strokes),
        encodeGlyphLibrary(original), "closed shape returns without loss");
        const full = Array.from({ length: 4096 }, (_, index) => ({
            ...first, x1: 500 + 2 * (index % 2),
            x2: 500 + 2 * ((index + 1) % 2),
        }));
        assert.equal(candidatePenPaths(full, nfd).length, 1);
        const result = reflectCandidatePath(full, nfd, 2048,
            "x", 1750).strokes;
        assert.equal(result.length, 4096);
        assert.equal(candidatePenPaths(result, nfd).length, 1);
        assert.equal(result[0].x1, 3000);
        assert.equal(result[0].x2, 2998);
        assert.equal(encodeGlyphLibrary(reflectCandidatePath(
            result, nfd, 2048, "x", 1750).strokes),
        encodeGlyphLibrary(full));
    });

test("selected path rotation keeps exact Unicode slots and directed joints",
    () => {
        const nfd = "n\u0303";
        const a = { ...stroke(nfd), x1: 500, y1: 700,
            x2: 900, y2: 1200, width: 251, color: "#112233" };
        const b = { ...a, x1: 900, y1: 1200,
            x2: 1400, y2: 1900 };
        const accent = { ...a, x1: 2200, y1: 2500,
            x2: 2700, y2: 3000, color: "#445566" };
        const original = [a, stroke("ñ"), b, stroke("∞"), accent];
        const before = encodeGlyphLibrary(original);
        const result = rotateCandidatePath(original, nfd, 2, 20, 1750, 2250);
        assert.deepEqual(result, { strokes: [
            { ...a, x1: 1106, y1: 366, x2: 1310, y2: 973 },
            original[1],
            { ...b, x1: 1310, y1: 973, x2: 1541, y2: 1801 },
            original[3], accent,
        ], selectedIndex: 2 });
        assert.deepEqual(candidatePenPaths(result.strokes, nfd).map((p) =>
            [p.first, p.last]), [[1, 2], [3, 3]]);
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(result.strokes)),
            result.strokes);
        assert.equal(encodeGlyphLibrary(original), before,
            "proposal must never change source TSV");
        assert.deepEqual(rotateCandidatePath(original, nfd, 0,
            -10, 1750, 2250).strokes[0], {
            ...a, x1: 250, y1: 941, x2: 731, y2: 1364,
        });
    });

test("selected path rotation refuses invalid or unchanged physical ink",
    () => {
        const key = "n\u0303";
        const base = { ...stroke(key), x1: 500, y1: 700,
            x2: 900, y2: 1200, width: 251 };
        const rows = [base, stroke("ñ")];
        const before = encodeGlyphLibrary(rows);
        for (const [index, angle, pivotX, pivotY] of [
            [-1, 10, 1750, 2250], [1, 10, 1750, 2250],
            [0, 0, 1750, 2250], [0, 21, 1750, 2250],
            [0, -21, 1750, 2250], [0, 0.5, 1750, 2250],
            [0, Infinity, 1750, 2250], [0, 10, -1, 2250],
            [0, 10, 3501, 2250], [0, 10, 1750, 4501],
            [0, 10, 1750, 1.5], [0, 20, 3500, 4500],
        ]) {
            assert.throws(() => rotateCandidatePath(rows, key,
                index, angle, pivotX, pivotY), /path|rotation|clip|selected/i);
        }
        assert.throws(() => rotateCandidatePath(rows, "ñ", 0,
            10, 1750, 2250), /selected/);
        assert.throws(() => rotateCandidatePath([
            { ...base, x1: 1750, y1: 2250,
                x2: 1751, y2: 2250 },
        ], key, 0, 1, 1750, 2250), /does not change/);
        assert.equal(encodeGlyphLibrary(rows), before);
    });

test("selected path rotation preserves closed loops and full stroke budget",
    () => {
        const key = "n\u0303";
        const first = { ...stroke(key), x1: 1000, y1: 1000,
            x2: 1400, y2: 1000, width: 200 };
        const loop = [first, stroke("ñ"),
            { ...first, x1: 1400, y1: 1000, x2: 1400, y2: 1400 },
            { ...first, x1: 1400, y1: 1400, x2: 1000, y2: 1400 },
            { ...first, x1: 1000, y1: 1400, x2: 1000, y2: 1000 }];
        const changed = rotateCandidatePath(loop, key, 3,
            15, 1750, 2250);
        const paths = candidatePenPaths(changed.strokes, key);
        assert.deepEqual(paths.map((p) => [p.first, p.last]), [[1, 4]]);
        assert.deepEqual(paths[0].start, paths[0].end);
        assert.equal(changed.strokes[1], loop[1]);
        const full = Array.from({ length: 4096 }, (_, index) => ({
            ...first, x1: 1000 + 2 * (index % 2),
            x2: 1000 + 2 * ((index + 1) % 2),
        }));
        const output = rotateCandidatePath(full, key, 2048,
            20, 1750, 2250);
        assert.equal(output.strokes.length, 4096);
        assert.equal(output.selectedIndex, 2048);
        assert.equal(candidatePenPaths(output.strokes, key).length, 1);
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(output.strokes)),
            output.strokes);
    });

test("selected path rotation UI saves only accepted and undoable TSV",
    async () => {
        const ui = mountEditor();
        try {
            const { editor, get, downloads } = ui;
            const key = "n\u0303";
            const a = { ...stroke(key), x1: 500, y1: 700,
                x2: 900, y2: 1200, width: 251 };
            const b = { ...a, x1: 900, y1: 1200,
                x2: 1400, y2: 1900 };
            const original = [a, stroke("ñ"), b];
            const bytes = new TextEncoder().encode(
                encodeGlyphLibrary(original));
            assert.equal(get("path-rotate").disabled, true);
            editor.start();
            get("load").files = [{ size: bytes.length,
                arrayBuffer: async () => bytes.buffer }];
            await get("load").dispatch("change");
            get("character").value = key;
            get("character").dispatch("input");
            get("selected").value = "2";
            get("selected").dispatch("change");
            for (const field of ["rotate", "rotate-degrees",
                "rotate-pivot-x", "rotate-pivot-y"]) {
                assert.equal(get(`path-${field}`).disabled, false);
            }
            get("path-rotate-degrees").value = "20";
            get("path-rotate").dispatch("click");
            assert.match(get("status").textContent,
                /Rotated selected continuous pen path; unsaved/);
            assert.equal(downloads.length, 0);
            const result = rotateCandidatePath(original, key, 2,
                20, 1750, 2250).strokes;
            get("save").dispatch("click");
            assert.equal(await downloads[0].text(), encodeGlyphLibrary(result));
            get("slant-undo").dispatch("click");
            assert.match(get("status").textContent,
                /Undid last pen path rotation/);
            get("save").dispatch("click");
            assert.equal(await downloads[1].text(),
                encodeGlyphLibrary(original));
            get("slant-redo").dispatch("click");
            get("save").dispatch("click");
            assert.equal(await downloads[2].text(), encodeGlyphLibrary(result));
            get("path-rotate-degrees").value = "0";
            get("path-rotate").dispatch("click");
            assert.match(get("status").textContent, /nonzero/);
            get("save").dispatch("click");
            assert.equal(await downloads[3].text(), encodeGlyphLibrary(result));
            editor.reset();
            assert.equal(get("path-rotate").disabled, true);
            assert.equal(get("slant-undo").disabled, true);
        } finally {
            ui.restore();
        }
    });

test("selected path slant changes only exact Unicode pen-path joints", () => {
    const nfd = "n\u0303";
    const a = { ...stroke(nfd), x1: 800, y1: 800,
        x2: 1100, y2: 1400, width: 251, color: "#112233" };
    const b = { ...a, x1: 1100, y1: 1400,
        x2: 1400, y2: 2000 };
    const accent = { ...a, x1: 2200, y1: 2500,
        x2: 2700, y2: 3000, color: "#445566" };
    const other = { ...a, grapheme: "ñ", x1: 2200, y1: 600,
        x2: 2200, y2: 3300, width: 300 };
    const infinity = { ...a, grapheme: "∞", x1: 800, y1: 800,
        x2: 1700, y2: 1500, width: 200 };
    const original = [a, other, b, infinity, accent];
    const before = encodeGlyphLibrary(original);
    const changed = shearCandidatePath(original, nfd, 2, 20, 2600);
    assert.deepEqual(changed, { strokes: [
        { ...a, x1: 1455, x2: 1537 }, other,
        { ...b, x1: 1537, x2: 1618 }, infinity, accent,
    ], selectedIndex: 2 });
    assert.deepEqual(candidatePenPaths(changed.strokes, nfd).map((path) =>
        [path.first, path.last]), [[1, 2], [3, 3]]);
    assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(changed.strokes)),
        changed.strokes);
    const left = shearCandidatePath(original, nfd, 0, -20, 2600);
    assert.equal(left.strokes[0].x1, 145);
    assert.equal(left.strokes[0].x2, left.strokes[2].x1);
    assert.equal(encodeGlyphLibrary(original), before,
        "browser slant is a proposal, not source mutation");
});

test("selected path slant validates bounds, no-ops and full contours", () => {
    const nfd = "n\u0303";
    const a = { ...stroke(nfd), x1: 800, y1: 800,
        x2: 1100, y2: 1400, width: 251 };
    const base = [a, { ...a, grapheme: "ñ" }];
    const original = encodeGlyphLibrary(base);
    for (const [index, angle, baseline] of [
        [-1, 10, 2000], [1, 10, 2000],
        [0, 0, 2000], [0, 21, 2000], [0, -21, 2000],
        [0, 1.5, 2000], [0, NaN, 2000],
        [0, 10, -1], [0, 10, 4501], [0, 10, 2000.5],
        [0, -20, 4500],
    ]) {
        assert.throws(() => shearCandidatePath(base, nfd,
            index, angle, baseline), /path|slant|clip|selected/i);
    }
    assert.throws(() => shearCandidatePath(base,
        "nn", 0, 10, 2000), /selected/i);
    assert.throws(() => shearCandidatePath([{
        ...a, x1: 1000, x2: 1500, y1: 2000, y2: 2000,
    }], nfd, 0, 10, 2000), /does not change/);
    assert.equal(encodeGlyphLibrary(base), original);

    const loop = [{ ...a, x1: 1000, y1: 1000, x2: 1400, y2: 1000,
        width: 200 },
    { ...a, x1: 1400, y1: 1000, x2: 1400, y2: 1400, width: 200 },
    { ...a, x1: 1400, y1: 1400, x2: 1000, y2: 1400, width: 200 },
    { ...a, x1: 1000, y1: 1400, x2: 1000, y2: 1000, width: 200 }];
    const closed = shearCandidatePath(loop, nfd, 2, -15, 2000);
    assert.deepEqual(candidatePenPaths(closed.strokes, nfd)[0].start,
        candidatePenPaths(closed.strokes, nfd)[0].end);
    const full = Array.from({ length: 4096 }, (_, index) => ({
        ...a, x1: 1000 + 2 * (index % 2),
        x2: 1000 + 2 * ((index + 1) % 2), y1: 1000, y2: 1000,
        width: 200,
    }));
    const changed = shearCandidatePath(full, nfd, 2048, 20, 2000);
    assert.equal(changed.strokes.length, 4096);
    assert.equal(candidatePenPaths(changed.strokes, nfd).length, 1);
    assert.equal(changed.strokes[0].x1, 1364);
    assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(changed.strokes)),
        changed.strokes);
});

test("selected path slant UI is undoable and only deliberately exported",
    async () => {
        const ui = mountEditor();
        try {
            const { editor, get, downloads } = ui;
            const key = "n\u0303";
            const a = { ...stroke(key), x1: 800, y1: 800,
                x2: 1100, y2: 1400, width: 251 };
            const b = { ...a, x1: 1100, y1: 1400,
                x2: 1400, y2: 2000 };
            const original = [a, { ...a, grapheme: "ñ" }, b];
            const bytes = new TextEncoder().encode(
                encodeGlyphLibrary(original));
            assert.equal(get("path-shear").disabled, true);
            editor.start();
            get("load").files = [{ size: bytes.length,
                arrayBuffer: async () => bytes.buffer }];
            await get("load").dispatch("change");
            get("character").value = key;
            get("character").dispatch("input");
            get("selected").value = "2";
            get("selected").dispatch("change");
            for (const field of ["shear", "shear-degrees",
                "shear-baseline"]) {
                assert.equal(get(`path-${field}`).disabled, false);
            }
            get("path-shear-degrees").value = "20";
            get("path-shear-baseline").value = "2600";
            get("path-shear").dispatch("click");
            assert.match(get("status").textContent,
                /Slanted selected continuous pen path; unsaved/);
            assert.equal(downloads.length, 0);
            const result = shearCandidatePath(original, key, 2,
                20, 2600).strokes;
            get("save").dispatch("click");
            assert.equal(await downloads[0].text(), encodeGlyphLibrary(result));
            get("slant-undo").dispatch("click");
            assert.match(get("status").textContent,
                /Undid last pen path slant/);
            get("save").dispatch("click");
            assert.equal(await downloads[1].text(),
                encodeGlyphLibrary(original));
            get("slant-redo").dispatch("click");
            get("save").dispatch("click");
            assert.equal(await downloads[2].text(), encodeGlyphLibrary(result));
            for (const invalid of ["0", "21", "-21", "1e1", "+10", "10.5"]) {
                get("path-shear-degrees").value = invalid;
                get("path-shear").dispatch("click");
                get("save").dispatch("click");
                assert.equal(await downloads.at(-1).text(),
                    encodeGlyphLibrary(result));
            }
            editor.reset();
            assert.equal(get("path-shear").disabled, true);
        } finally {
            ui.restore();
        }
    });
