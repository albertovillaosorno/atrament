// Copyright © 2026 Alberto Villa Osorno. SPDX-License-Identifier: LicenseRef-Proprietary
// Browser authored Unicode stroke authoring and session-safe TSV contracts.
import assert from "node:assert/strict";
import test from "node:test";
const { parseGlyphLibrary, encodeGlyphLibrary, createGlyphStrokeEditor,
    geometricStarter, italicizeGlyphStrokes, scaleGlyphHeight, scaleGlyphWidth,
    scaleGlyphInk, translateGlyphStrokes, rotateGlyphStrokes,
    composeAccentedGlyph, varyGlyphStrokes,
    candidatePenPaths, reverseCandidatePath, moveCandidatePath,
    rotateClosedCandidatePath, closeCandidatePath,
    translateCandidatePath } =
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
    const ids = ["glyph-character", "glyph-color", "glyph-load",
        "glyph-add", "glyph-copy-from", "glyph-copy", "glyph-compose",
        "glyph-selected",
        "glyph-form", "glyph-construct", "glyph-seed-lowercase",
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
        "glyph-move-x", "glyph-move-y", "glyph-move-all",
        "glyph-move-preview", "glyph-move-preview-status",
        "glyph-ideal-preview", "glyph-ideal-geometry",
        "glyph-ideal-circle", "glyph-ideal-stem", "glyph-ideal-arch",
        "glyph-ideal-entry", "glyph-ideal-approach", "glyph-ideal-exit",
        "glyph-ideal-rounded", "glyph-ideal-rounded-left",
        "glyph-ideal-rounded-right", "glyph-ideal-c-loop",
        "glyph-ideal-e-bar", "glyph-ideal-i-stem", "glyph-ideal-i-dot",
        "glyph-ideal-accent",
        "glyph-update", "glyph-remove", "glyph-reverse",
        "glyph-reverse-path", "glyph-path-start", "glyph-path-close",
        "glyph-path-earlier", "glyph-path-later",
        "glyph-path-dx", "glyph-path-dy", "glyph-path-translate",
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
    get("move-x").value = "0";
    get("move-y").value = "0";
    get("path-dx").value = "0";
    get("path-dy").value = "0";
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
                /(?:1200|matching|already continuous)/);
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
        ["ñ", 1550, 2450, 1100, 250, "#123456"],
        ["o", -1, 2450, 1100, 250, "#123456"],
        ["o", 200, 2450, 1100, 250, "#123456"],
        ["a", 1550, 2450, 0, 250, "#123456"],
        ["o", 1550, 2450, 1100.25, 250, "#123456"],
        ["o", 1550, 2450, 1100, 79, "#123456"],
        ["o", 1550, 2450, 1100, 250, "red"],
    ]) assert.throws(() => geometricStarter(...bad));
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
    const oneStroke = source.slice(0, 1);
    assert.throws(() => composeAccentedGlyph(
        oneStroke, "i", "í", "#ABCDEF"), /clear upper dot/);
    assert.throws(() => composeAccentedGlyph([
        oneStroke[0], { ...source[1], y1: 1980, y2: 2060 },
    ], "i", "í", "#ABCDEF"), /clear upper dot/);
    assert.throws(() => composeAccentedGlyph([
        ...source, { ...source[0], x1: 2000, x2: 2000 },
    ], "i", "í", "#ABCDEF"), /clear upper dot/);
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

test("six basic math operators have exact bounded pen geometry", () => {
    const expected = {
        "+": [[780, 2450, 2320, 2450], [1550, 1680, 1550, 3220]],
        "-": [[780, 2450, 2320, 2450]],
        "−": [[780, 2450, 2320, 2450]],
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
        "≤": [[2155, 1790, 890, 2505], [890, 2505, 2155, 3110],
            [780, 3418, 2320, 3418]],
        "≥": [[945, 1790, 2210, 2505], [2210, 2505, 945, 3110],
            [780, 3418, 2320, 3418]],
    };
    for (const [glyph, segments] of Object.entries(expected)) {
        const lines = geometricStarter(glyph, 1550, 2450, 1100,
            250, "#243647");
        assert.deepEqual(lines.map(({ x1, y1, x2, y2 }) =>
            [x1, y1, x2, y2]), segments);
        assert.deepEqual(parseGlyphLibrary(encodeGlyphLibrary(lines)), lines);
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

test("calculus and comparison math SVG preserves ideal curve controls",
    async () => {
        const ui = mountEditor();
        try {
            const { editor, get, downloads } = ui;
            editor.start();
            for (const [glyph, quadratics, lifts] of [
                ["∑", 0, 1], ["∫", 3, 1],
                ["∂", 4, 1], ["Δ", 0, 1], ["≈", 4, 2],
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
        assert.match(get("status").textContent, /Filled 15 missing math/);
        assert.equal(downloads.length, 0);
        get("save").dispatch("click");
        const saved = parseGlyphLibrary(await downloads.at(-1).text());
        const missing = ["+", "-", "=", "×", "÷", "√", "π", "≠", "≤", "≥",
            "∑", "∫", "∂", "Δ", "≈"];
        assert.deepEqual(saved, [...originals, ...missing.flatMap(
            (glyph) => geometricStarter(glyph, 1550, 2450,
                1100, 250, "#243647"))]);
        get("seed-math").dispatch("click");
        assert.match(get("status").textContent, /All 17 math/);
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

test("eleven explicit fills author 130 exact glyphs", async () => {
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
        assert.equal(all.length, 3002);
        assert.equal(new Set(all.map((line) => line.grapheme)).size, 130);
        const differentMinuses = ["-", "−"].map((glyph) =>
            all.filter((line) => line.grapheme === glyph));
        assert.equal(differentMinuses[0].length, 1);
        assert.equal(differentMinuses[1].length, 1);
        assert.notEqual(differentMinuses[0][0].grapheme,
            differentMinuses[1][0].grapheme);
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
        "+", "-", "−", "=", "×", "÷",
        "√", "π", "∞", "≠", "≤", "≥", "∑", "∫", "∂", "Δ", "≈",
        "°", "%", ..."0123456789", ..."ABCDEFGHIJKLMNOPQRSTUVWXYZ",
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
    assert.equal(checked, 15984);
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
    for (const invalid of ["b", "n", "A", "∞"]) {
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
            assert.equal(undo.disabled, true);
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

test("slant history is one-level and clears on import or ink edits",
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
        assert.equal(get("slant-undo").disabled, true);
        get("slant-redo").dispatch("click");
        get("save").dispatch("click");
        assert.equal(await downloads.at(-1).text(),
            encodeGlyphLibrary(second), "redo reapplies latest transform");
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
        assert.equal(get("slant-undo").disabled, true);
        assert.equal(get("slant-redo").disabled, true);
        get("slant-undo").dispatch("click");
        assert.equal(get("save").disabled, false);
        assert.equal(get("status").textContent.startsWith("0 unsaved"),
            true, "stale undo does not restore a removed stroke");
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
            // Ordinary edits invalidate stale recovery, even on a sibling.
            add("∞", [1650, 2300, 1750, 2400], 251);
            assert.equal(undo.disabled, true);
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
