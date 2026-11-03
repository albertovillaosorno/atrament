// Copyright © 2026 Alberto Villa Osorno. SPDX-License-Identifier: MIT
// Browser authored Unicode stroke authoring and session-safe TSV contracts.
import assert from "node:assert/strict";
import test from "node:test";
const { parseGlyphLibrary, encodeGlyphLibrary, createGlyphStrokeEditor,
    geometricStarter, composeAccentedGlyph } =
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
        "glyph-form", "glyph-construct", "glyph-circle-x",
        "glyph-circle-y", "glyph-circle-radius", "glyph-accent-shift",
        "glyph-accent-lift", "glyph-ideal-preview",
        "glyph-ideal-circle", "glyph-ideal-stem", "glyph-ideal-arch",
        "glyph-ideal-accent",
        "glyph-update", "glyph-remove", "glyph-reverse",
        "glyph-earlier", "glyph-later",
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
        assert.equal(get("reverse").disabled, true);
        assert.equal(get("earlier").disabled, true);
        assert.equal(get("later").disabled, true);
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
        assert.match(get("status").textContent, /Choose matching o, a, ó or á/);
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
