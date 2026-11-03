// Copyright © 2026 Alberto Villa Osorno. SPDX-License-Identifier: MIT
// Authenticated Rust handwriting statistics form, without ambient storage.
import assert from "node:assert/strict";
import test from "node:test";
const { createHandwritingProfileEditor } = await import(new URL(
    "../../../../src/browser/workspace/adapter-inbound/generated/"
    + "handwriting-profile.js", import.meta.url,
).href);

const dimensions = [
    ["baseline_offset", "micrometres", -20000, 20000, 0],
    ["flattening", "parts_per_thousand", 0, 1000, 0],
    ["height", "micrometres", 100, 30000, 3000],
    ["ink_thickness", "micrometres", 20, 5000, 300],
    ["letter_spacing", "micrometres", -10000, 30000, 350],
    ["pressure", "parts_per_thousand", 0, 1000, 500],
    ["roundness", "parts_per_thousand", 0, 1000, 500],
    ["slant", "millidegrees", -85000, 85000, 0],
    ["texture", "parts_per_thousand", 0, 1000, 0],
    ["width", "micrometres", 100, 30000, 2200],
];
const catalog = {
    dimensions: dimensions.map(([key, unit, minimum, maximum]) => ({
        key, unit, minimum, maximum,
    })),
};
const parameter = ([dimension, , , , mean]) => ({
    dimension, minimum: mean, maximum: mean, mean, median: mean,
    standard_deviation: 0,
});
const defaultProfile = () => ({
    version: "atrament.handwriting.statistics.v1",
    seed: 0,
    globals: dimensions.map(parameter),
    glyphs: [],
});
const fields = ["minimum", "maximum", "mean", "median",
    "standard_deviation"];
const ids = ["handwriting-scope", "handwriting-dimension",
    "handwriting-glyph", "handwriting-new-glyph", "handwriting-add-glyph",
    "handwriting-remove-glyph", "handwriting-add-override",
    "handwriting-remove-override", "handwriting-seed", "handwriting-load",
    "handwriting-reset", "handwriting-save", "handwriting-status",
    "handwriting-local-name", "handwriting-local-load",
    "handwriting-local-save",
    "handwriting-preview", ...fields.flatMap((field) => [
        `handwriting-${field}`, `handwriting-${field}-range`])];

class FakeControl {
    value = "";
    disabled = true;
    textContent = "";
    min = "";
    max = "";
    files = null;
    children = [];
    listeners = new Map();
    addEventListener(type, callback) { this.listeners.set(type, callback); }
    dispatch(type) { this.listeners.get(type)?.(); }
    replaceChildren(...children) { this.children = [...children]; }
    append(child) { this.children.push(child); }
}

function mountBackend(implementation = {}) {
    const original = {
        document: globalThis.document,
        window: globalThis.window,
        fetch: globalThis.fetch,
        createObjectURL: URL.createObjectURL,
        revokeObjectURL: URL.revokeObjectURL,
    };
    const elements = new Map(ids.map((id) => [id, new FakeControl()]));
    const links = [];
    const revoked = [];
    const requests = [];
    let secret = "private-session-credential";
    const controller = new AbortController();
    elements.get("handwriting-scope").value = "global";
    elements.get("handwriting-local-name").value = "my-handwriting.json";
    globalThis.document = {
        getElementById: (id) => elements.get(id) ?? null,
        createElement: (tag) => {
            assert.ok(["option", "a"].includes(tag));
            const element = new FakeControl();
            if (tag === "a") {
                element.click = () => links.push({
                    download: element.download, href: element.href,
                });
            }
            return element;
        },
    };
    globalThis.window = { setTimeout() {} };
    URL.createObjectURL = () => "blob:handwriting-profile";
    URL.revokeObjectURL = (url) => { revoked.push(url); };
    globalThis.fetch = async (endpoint, init) => {
        requests.push({ endpoint, ...init });
        assert.equal(init.headers.Authorization, `Bearer ${secret}`);
        assert.equal(init.credentials, "omit");
        assert.equal(init.mode, "same-origin");
        assert.equal(init.cache, "no-store");
        assert.equal(init.referrerPolicy, "origin");
        assert.equal(init.redirect, "error");
        if (implementation.fetch) return implementation.fetch(endpoint, init);
        if (endpoint === "./api/handwriting/dimensions") {
            assert.equal(init.method, "GET");
            return Response.json(catalog);
        }
        if (init.method === "GET") return Response.json(defaultProfile());
        assert.equal(init.method, "POST");
        assert.equal(init.headers["Content-Type"], "application/json");
        return Response.json(JSON.parse(init.body));
    };
    const editor = createHandwritingProfileEditor({
        currentSecret: () => secret,
        sessionSignal: controller.signal,
        unauthorized() { implementation.unauthorized?.(); },
    });
    return {
        elements, links, revoked, requests, editor, controller,
        changeSecret(value) { secret = value; },
        restore() {
            globalThis.document = original.document;
            globalThis.window = original.window;
            globalThis.fetch = original.fetch;
            URL.createObjectURL = original.createObjectURL;
            URL.revokeObjectURL = original.revokeObjectURL;
        },
    };
}
const tick = () => new Promise((resolve) => setImmediate(resolve));

function edit(elements, id, value, event = "input") {
    const control = elements.get(id);
    control.value = value;
    control.dispatch(event);
}

test("handwriting retains exact NFD glyph edits", async () => {
    const b = mountBackend();
    try {
        assert.equal(b.elements.get("handwriting-save").disabled, true);
        await b.editor.start("private-session-credential");
        assert.equal(b.elements.get("handwriting-save").disabled, false);
        assert.equal(
            b.elements.get("handwriting-dimension").children.length, 10);
        edit(b.elements, "handwriting-dimension", "width", "change");
        edit(b.elements, "handwriting-minimum", "2000");
        edit(b.elements, "handwriting-maximum", "2500");
        edit(b.elements, "handwriting-mean-range", "2250");
        edit(b.elements, "handwriting-median", "2250");
        edit(b.elements, "handwriting-standard_deviation", "100");
        assert.equal(b.elements.get("handwriting-mean").value, "2250");
        edit(b.elements, "handwriting-dimension", "height", "change");
        edit(b.elements, "handwriting-dimension", "width", "change");
        assert.equal(b.elements.get("handwriting-minimum").value, "2000");
        edit(b.elements, "handwriting-scope", "glyph", "change");
        assert.equal(b.elements.get("handwriting-mean").disabled, true);
        edit(b.elements, "handwriting-new-glyph", "a\u0301");
        b.elements.get("handwriting-add-glyph").dispatch("click");
        assert.equal(b.elements.get("handwriting-mean").disabled, false);
        assert.match(b.elements.get("handwriting-preview").textContent,
            /Exact glyph a\u0301/);
        edit(b.elements, "handwriting-dimension", "height", "change");
        assert.equal(b.elements.get("handwriting-mean").disabled, true);
        assert.match(b.elements.get("handwriting-preview").textContent,
            /inherits global/);
        b.elements.get("handwriting-add-override").dispatch("click");
        edit(b.elements, "handwriting-minimum", "2800");
        edit(b.elements, "handwriting-maximum", "3400");
        edit(b.elements, "handwriting-mean", "3100");
        edit(b.elements, "handwriting-median", "3100");
        edit(b.elements, "handwriting-standard_deviation-range", "150");
        edit(b.elements, "handwriting-seed", "17");
        b.elements.get("handwriting-save").dispatch("click");
        await tick();
        assert.equal(b.links.length, 1);
        assert.equal(b.links[0].download, "atrament-handwriting.json");
        const sent = JSON.parse(b.requests.at(-1).body);
        assert.equal(sent.seed, 17);
        assert.equal(sent.globals.find(
            (p) => p.dimension === "width").mean, 2250);
        assert.equal(sent.globals.find(
            (p) => p.dimension === "width").minimum, 2000);
        assert.equal(sent.glyphs[0].grapheme, "a\u0301");
        assert.equal(sent.glyphs[0].parameters.find(
            (p) => p.dimension === "height").mean, 3100);
        // Downloads require explicit user actions, not ambient browser storage.
        assert.equal(b.requests.filter((r) => r.method === "POST").length, 1);
        b.editor.reset();
        assert.equal(b.elements.get("handwriting-save").disabled, true);
        assert.deepEqual(b.revoked, ["blob:handwriting-profile"]);
    } finally {
        b.restore();
    }
});

test("handwriting editor rejects unsafe input and loads JSON", async () => {
    let rejects = true;
    const b = mountBackend({
        fetch(endpoint, init) {
            if (endpoint === "./api/handwriting/dimensions") {
                return Response.json(catalog);
            }
            if (init.method === "GET") return Response.json(defaultProfile());
            return rejects ? new Response("{}", { status: 400 })
                : Response.json(JSON.parse(init.body));
        },
    });
    try {
        await b.editor.start("private-session-credential");
        const initial = b.requests.length;
        edit(b.elements, "handwriting-mean", "not-a-number");
        b.elements.get("handwriting-save").dispatch("click");
        await tick();
        assert.equal(b.requests.length, initial);
        assert.equal(b.links.length, 0);
        edit(b.elements, "handwriting-mean", "0");
        b.elements.get("handwriting-save").dispatch("click");
        await tick();
        assert.equal(b.requests.at(-1).method, "POST");
        assert.equal(b.links.length, 0);
        assert.match(b.elements.get("handwriting-status").textContent,
            /no file saved/);
        rejects = false;
        const loaded = defaultProfile();
        loaded.glyphs.push({
            grapheme: "ñ", parameters: [parameter(dimensions[2])],
        });
        const json = JSON.stringify(loaded);
        b.elements.get("handwriting-load").files = {
            item: () => ({ size: json.length, text: async () => json }),
        };
        b.elements.get("handwriting-load").dispatch("change");
        await tick();
        edit(b.elements, "handwriting-scope", "glyph", "change");
        assert.equal(
            b.elements.get("handwriting-glyph").children[0].value, "ñ");
        assert.equal(b.elements.get("handwriting-glyph").value, "ñ");
        edit(b.elements, "handwriting-dimension", "height", "change");
        assert.equal(b.elements.get("handwriting-mean").value, "3000");
        b.elements.get("handwriting-remove-override").dispatch("click");
        assert.equal(b.elements.get("handwriting-glyph").children.length, 0);
        b.elements.get("handwriting-save").dispatch("click");
        await tick();
        assert.equal(b.links.length, 1);
        assert.equal(JSON.parse(b.requests.at(-1).body).glyphs.length, 0);
    } finally {
        b.restore();
    }
});

test("handwriting download refuses invalidated sessions", async () => {
    let release;
    const pending = new Promise((resolve) => { release = resolve; });
    const b = mountBackend({
        fetch(endpoint, init) {
            if (endpoint === "./api/handwriting/dimensions") {
                return Response.json(catalog);
            }
            if (init.method === "GET") return Response.json(defaultProfile());
            return pending;
        },
    });
    try {
        await b.editor.start("private-session-credential");
        b.elements.get("handwriting-save").dispatch("click");
        b.changeSecret(null);
        b.editor.reset();
        release(Response.json(defaultProfile()));
        await tick();
        assert.equal(b.links.length, 0);
        assert.equal(b.elements.get("handwriting-save").disabled, true);
    } finally {
        b.restore();
    }
});

test("handwriting editor rejects imprecise u64 seed", async () => {
    const b = mountBackend({
        fetch(endpoint) {
            if (endpoint === "./api/handwriting/dimensions") {
                return Response.json(catalog);
            }
            const json = JSON.stringify(defaultProfile()).replace(
                '"seed":0', '"seed":18446744073709551615');
            return new Response(json, {
                headers: { "Content-Type": "application/json" },
            });
        },
    });
    try {
        await b.editor.start("private-session-credential");
        assert.equal(b.elements.get("handwriting-save").disabled, true);
        assert.match(b.elements.get("handwriting-status").textContent,
            /seed too large/);
    } finally {
        b.restore();
    }
});


test("handwriting local file stores checked revisions and declines stale saves", async () => {
    let diskProfile = null;
    let diskRevision = null;
    let revisionStep = 0;
    const b = mountBackend({
        fetch(endpoint, init) {
            if (endpoint === "./api/handwriting/dimensions") {
                return Response.json(catalog);
            }
            if (endpoint === "./api/handwriting/file") {
                assert.equal(init.headers["X-Atrament-Profile-Name"],
                    "my-handwriting.json");
                assert.equal(init.credentials, "omit");
                assert.equal(init.cache, "no-store");
                assert.equal(init.mode, "same-origin");
                if (init.method === "GET") {
                    return diskRevision === null
                        ? Response.json({error: "profile_file_missing"},
                            {status: 404})
                        : Response.json({
                            profile: diskProfile, revision: diskRevision,
                        });
                }
                const revision = init.headers["X-Atrament-Profile-Revision"];
                if (diskRevision === null ? revision !== "new"
                    : revision !== diskRevision) {
                    return Response.json({ error: "stale_profile_file" },
                        { status: 409 });
                }
                diskProfile = JSON.parse(init.body);
                revisionStep += 1;
                diskRevision = "sha256:" + String.fromCharCode(96 + revisionStep).repeat(64);
                return Response.json({ revision: diskRevision });
            }
            if (init.method === "GET") return Response.json(defaultProfile());
            return Response.json(JSON.parse(init.body));
        },
    });
    try {
        await b.editor.start("private-session-credential");
        edit(b.elements, "handwriting-seed", "11");
        b.elements.get("handwriting-local-save").dispatch("click");
        await tick();
        await tick();
        assert.equal(b.requests.at(-1).endpoint, "./api/handwriting/file");
        assert.equal(b.requests.at(-1).headers["X-Atrament-Profile-Revision"],
            "new");
        assert.equal(diskProfile.seed, 11);
        assert.match(b.elements.get("handwriting-status").textContent,
            /saved in local directory/);
        b.elements.get("handwriting-local-load").dispatch("click");
        await tick();
        await tick();
        assert.equal(b.elements.get("handwriting-seed").value, "11");
        edit(b.elements, "handwriting-seed", "12");
        b.elements.get("handwriting-local-save").dispatch("click");
        await tick();
        await tick();
        assert.equal(b.requests.at(-1).headers["X-Atrament-Profile-Revision"],
            "sha256:" + "a".repeat(64));
        assert.equal(diskProfile.seed, 12);
        diskRevision = "sha256:" + "c".repeat(64);
        edit(b.elements, "handwriting-seed", "13");
        b.elements.get("handwriting-local-save").dispatch("click");
        await tick();
        await tick();
        assert.equal(diskProfile.seed, 12);
        assert.match(b.elements.get("handwriting-status").textContent,
            /changed; reopen first/);
        b.elements.get("handwriting-local-name").value = "../unsafe.json";
        b.elements.get("handwriting-local-name").dispatch("input");
        const before = b.requests.length;
        b.elements.get("handwriting-local-save").dispatch("click");
        await tick();
        assert.equal(b.requests.length, before,
            "unsafe browser-selected names never reach local Rust file API");
        b.editor.reset();
        assert.equal(b.elements.get("handwriting-local-load").disabled, true);
    } finally {
        b.restore();
    }
});
