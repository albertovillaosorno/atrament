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
        "glyph-form", "glyph-construct", "glyph-seed-lowercase",
        "glyph-circle-x",
        "glyph-circle-y", "glyph-circle-radius", "glyph-accent-shift",
        "glyph-accent-lift", "glyph-ideal-preview",
        "glyph-ideal-circle", "glyph-ideal-stem", "glyph-ideal-arch",
        "glyph-ideal-rounded", "glyph-ideal-rounded-left",
        "glyph-ideal-rounded-right", "glyph-ideal-c-loop",
        "glyph-ideal-e-bar", "glyph-ideal-i-stem", "glyph-ideal-i-dot",
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

test("all geometric starters respect pen margins over a boundary grid", () => {
    const forms = ["o", "a", "ó", "á", "b", "d", "p", "q",
        "g", "n", "u", "h", "m", "r", "s", "l", "t", "f", "v", "w",
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
    assert.equal(checked, 4176);
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
