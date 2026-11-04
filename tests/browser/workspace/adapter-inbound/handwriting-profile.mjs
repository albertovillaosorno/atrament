// Copyright © 2026 Alberto Villa Osorno. SPDX-License-Identifier: LicenseRef-Proprietary
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
    "handwriting-preview", "handwriting-sample-glyph",
    "handwriting-sample-occurrence", "handwriting-sample",
    "handwriting-sample-result", ...fields.flatMap((field) => [
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
    elements.get("handwriting-sample-glyph").value = "a";
    elements.get("handwriting-sample-occurrence").value = "0";
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
        if (endpoint === "./api/handwriting/preview") {
            return Response.json({ value: 3000 });
        }
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

test("native numeric preview uses unsaved glyph edits and is not persisted", async () => {
    const b = mountBackend();
    try {
        await b.editor.start("private-session-credential");
        edit(b.elements, "handwriting-dimension", "height", "change");
        edit(b.elements, "handwriting-minimum", "2500");
        edit(b.elements, "handwriting-maximum", "3500");
        edit(b.elements, "handwriting-mean", "3200");
        edit(b.elements, "handwriting-median", "3200");
        edit(b.elements, "handwriting-standard_deviation", "150");
        edit(b.elements, "handwriting-seed", "73");
        edit(b.elements, "handwriting-sample-glyph", "ñ");
        edit(b.elements, "handwriting-sample-occurrence", "12");
        b.elements.get("handwriting-sample").dispatch("click");
        await tick();
        const request = b.requests.at(-1);
        assert.equal(request.endpoint, "./api/handwriting/preview");
        assert.equal(request.method, "POST");
        const payload = JSON.parse(request.body);
        assert.equal(payload.profile.seed, 73);
        assert.equal(payload.profile.globals.find(
            (p) => p.dimension === "height").mean, 3200);
        assert.equal(payload.grapheme, "ñ");
        assert.equal(payload.dimension, "height");
        assert.equal(payload.occurrence, 12);
        assert.match(b.elements.get("handwriting-sample-result").textContent,
            /3000 micrometres.*no glyph ink/);
        assert.equal(b.links.length, 0);
        edit(b.elements, "handwriting-mean", "3300");
        assert.equal(b.elements.get("handwriting-sample-result").textContent,
            "No numeric sample requested.");
        edit(b.elements, "handwriting-sample-occurrence", "-1");
        const count = b.requests.length;
        b.elements.get("handwriting-sample").dispatch("click");
        await tick();
        assert.equal(b.requests.length, count);
        b.editor.reset();
        assert.equal(b.elements.get("handwriting-sample").disabled, true);
    } finally {
        b.restore();
    }
});

test("native numeric preview refuses malformed response without retaining output", async () => {
    const b = mountBackend({
        fetch(endpoint, init) {
            if (endpoint === "./api/handwriting/dimensions") {
                return Response.json(catalog);
            }
            if (init.method === "GET") return Response.json(defaultProfile());
            assert.equal(endpoint, "./api/handwriting/preview");
            return Response.json({ value: "not-a-number" });
        },
    });
    try {
        await b.editor.start("private-session-credential");
        b.elements.get("handwriting-sample").dispatch("click");
        await tick();
        assert.equal(b.elements.get("handwriting-sample-result").textContent,
            "No numeric sample requested.");
        assert.match(b.elements.get("handwriting-status").textContent,
            /no file saved/);
        b.editor.reset();
        assert.equal(b.elements.get("handwriting-sample").disabled, true);
        assert.equal(b.elements.get("handwriting-sample-result").textContent,
            "No numeric sample requested.");
    } finally {
        b.restore();
    }
});

test("handwriting refuses non-native glyph identity before draft and sample",
    async () => {
    const b = mountBackend();
    try {
        await b.editor.start("private-session-credential");
        edit(b.elements, "handwriting-scope", "glyph", "change");
        const count = b.requests.length;
        for (const glyph of ["ab", "\uD800", "\uDC00", "\u0007"]) {
            edit(b.elements, "handwriting-new-glyph", glyph);
            b.elements.get("handwriting-add-glyph").dispatch("click");
            assert.equal(b.elements.get("handwriting-glyph").children.length,
                0, "invalid exact glyph must not enter the editable draft");
            edit(b.elements, "handwriting-sample-glyph", glyph);
            b.elements.get("handwriting-sample").dispatch("click");
            await tick();
            assert.equal(b.requests.length, count,
                "invalid sample cannot reach the native preview endpoint");
        }
        edit(b.elements, "handwriting-new-glyph", "🖋");
        b.elements.get("handwriting-add-glyph").dispatch("click");
        assert.equal(b.elements.get("handwriting-glyph").children[0].value,
            "🖋", "valid supplementary scalar stays exact");
        b.editor.reset();
    } finally {
        b.restore();
    }
});

test("handwriting rejects invalid native profile glyph", async () => {
    for (const grapheme of ["ab", "\uD800", "\u0007"]) {
        const b = mountBackend({
            fetch(endpoint, init) {
                if (endpoint === "./api/handwriting/dimensions") {
                    return Response.json(catalog);
                }
                if (init.method === "GET") {
                    const profile = defaultProfile();
                    profile.glyphs = [{
                        grapheme, parameters: [parameter(dimensions[0])],
                    }];
                    return Response.json(profile);
                }
                throw new Error("invalid profile must not be forwarded");
            },
        });
        try {
            await b.editor.start("private-session-credential");
            assert.equal(b.elements.get("handwriting-save").disabled, true);
            assert.equal(b.elements.get("handwriting-glyph").children.length,
                0, "untrusted response cannot populate editable overrides");
            assert.match(b.elements.get("handwriting-status").textContent,
                /unavailable|unsupported/);
        } finally {
            b.restore();
        }
    }
});

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
            item: () => new Blob([new TextEncoder().encode(json)]),
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

test("handwriting import refuses invalid UTF-8 before Rust admission",
    async () => {
    const b = mountBackend();
    try {
        await b.editor.start("private-session-credential");
        const profile = defaultProfile();
        profile.glyphs.push({
            grapheme: "a", parameters: [parameter(dimensions[2])],
        });
        const source = JSON.stringify(profile);
        const bytes = new TextEncoder().encode(source);
        const start = source.indexOf('"grapheme":"a"');
        assert.ok(start >= 0);
        bytes[start + '"grapheme":"'.length] = 0xFF;
        const before = b.requests.length;
        b.elements.get("handwriting-load").files = {
            item: () => new Blob([bytes]),
        };
        b.elements.get("handwriting-load").dispatch("change");
        await tick();
        assert.equal(b.requests.length, before,
            "corrupt bytes cannot become replacement glyph in Rust POST");
        assert.equal(b.elements.get("handwriting-glyph").children.length, 0,
            "rejected bytes cannot replace the current ink statistics");
        b.elements.get("handwriting-load").files = {
            item: () => new Blob([new TextEncoder().encode(source)]),
        };
        b.elements.get("handwriting-load").dispatch("change");
        await tick();
        assert.equal(b.elements.get("handwriting-glyph").children[0].value,
            "a", "valid native glyph bytes remain available");
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

// Use raw JSON tokens: Response.json would round large u64 values in Node.
const nativeProfileJson = (seed, profile = defaultProfile()) =>
    JSON.stringify(profile).replace('"seed":0', '"seed":' + seed);
const nativeResponse = (body) => new Response(body, {
    headers: { "Content-Type": "application/json" },
});

test("handwriting editor round-trips full u64 seeds without JS rounding", async () => {
    const maximum = "18446744073709551615";
    const b = mountBackend({
        fetch(endpoint, init) {
            if (endpoint === "./api/handwriting/dimensions") {
                return Response.json(catalog);
            }
            if (init.method === "GET") {
                return nativeResponse(nativeProfileJson(maximum));
            }
            if (endpoint === "./api/handwriting/preview") {
                return Response.json({value: 3000});
            }
            return nativeResponse(init.body);
        },
    });
    try {
        await b.editor.start("private-session-credential");
        assert.equal(b.elements.get("handwriting-seed").value, maximum);
        assert.equal(b.elements.get("handwriting-save").disabled, false);
        b.elements.get("handwriting-save").dispatch("click");
        await tick();
        assert.equal(b.links.length, 1);
        const saved = b.requests.at(-1).body;
        assert.ok(saved.includes('"seed":' + maximum));
        assert.ok(!saved.includes('"seed":"' + maximum + '"'));
        edit(b.elements, "handwriting-sample-occurrence", maximum);
        b.elements.get("handwriting-sample").dispatch("click");
        await tick();
        const sample = b.requests.at(-1);
        assert.equal(sample.endpoint, "./api/handwriting/preview");
        assert.ok(sample.body.includes('"seed":' + maximum));
        assert.ok(sample.body.includes('"occurrence":' + maximum));
        assert.match(b.elements.get("handwriting-sample-result").textContent,
            /#18446744073709551615:/);
        edit(b.elements, "handwriting-seed", "00017");
        b.elements.get("handwriting-save").dispatch("click");
        await tick();
        assert.ok(b.requests.at(-1).body.includes('"seed":17,'));
        assert.equal(b.elements.get("handwriting-seed").value, "17");
    } finally {
        b.restore();
    }
});

test("handwriting editor refuses out-of-range u64 and untrusted seed tokens", async () => {
    const badSeeds = ["18446744073709551616", "-1", "1e3", "1.5", "", "a"];
    for (const raw of badSeeds) {
        const b = mountBackend();
        try {
            await b.editor.start("private-session-credential");
            edit(b.elements, "handwriting-seed", raw);
            const requestCount = b.requests.length;
            b.elements.get("handwriting-save").dispatch("click");
            await tick();
            assert.equal(b.requests.length, requestCount, raw);
            assert.equal(b.links.length, 0);
            assert.match(b.elements.get("handwriting-status").textContent,
                /no file saved/);
        } finally { b.restore(); }
    }
    for (const badToken of ['18446744073709551616', '1e5', '"7"']) {
        const b = mountBackend({
            fetch(endpoint) {
                if (endpoint === "./api/handwriting/dimensions") {
                    return Response.json(catalog);
                }
                return nativeResponse(nativeProfileJson(badToken));
            },
        });
        try {
            await b.editor.start("private-session-credential");
            assert.equal(b.elements.get("handwriting-save").disabled, true);
        } finally { b.restore(); }
    }
});

test("local handwriting file restores exact maximum u64 seed", async () => {
    const maximum = "18446744073709551615";
    const revision = "sha256:" + "a".repeat(64);
    let stored = null;
    const b = mountBackend({
        fetch(endpoint, init) {
            if (endpoint === "./api/handwriting/dimensions") {
                return Response.json(catalog);
            }
            if (endpoint === "./api/handwriting/file") {
                if (init.method === "POST") {
                    stored = init.body;
                    return Response.json({revision});
                }
                return nativeResponse('{"profile":' + stored
                    + ',"revision":"' + revision + '"}');
            }
            if (init.method === "GET") {
                return nativeResponse(nativeProfileJson(maximum));
            }
            return nativeResponse(init.body);
        },
    });
    try {
        await b.editor.start("private-session-credential");
        b.elements.get("handwriting-local-save").dispatch("click");
        await tick();
        await tick();
        assert.ok(stored.includes('"seed":' + maximum));
        edit(b.elements, "handwriting-seed", "37");
        b.elements.get("handwriting-local-load").dispatch("click");
        await tick();
        await tick();
        assert.equal(b.elements.get("handwriting-seed").value, maximum);
        assert.equal(b.elements.get("handwriting-status").textContent,
            "Handwriting reopened from local Rust file.");
    } finally { b.restore(); }
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
        edit(b.elements, "handwriting-dimension", "width", "change");
        edit(b.elements, "handwriting-minimum", "2000");
        edit(b.elements, "handwriting-maximum", "2600");
        edit(b.elements, "handwriting-mean-range", "2300");
        edit(b.elements, "handwriting-median", "2300");
        edit(b.elements, "handwriting-standard_deviation-range", "100");
        assert.equal(b.elements.get("handwriting-mean").value, "2300");
        assert.match(b.elements.get("handwriting-preview").textContent,
            /range 2000…2600, mean 2300, median 2300, deviation 100/);
        edit(b.elements, "handwriting-seed", "12");
        b.elements.get("handwriting-local-save").dispatch("click");
        await tick();
        await tick();
        assert.equal(b.requests.at(-1).headers["X-Atrament-Profile-Revision"],
            "sha256:" + "a".repeat(64));
        assert.equal(diskProfile.seed, 12);
        const storedWidth = diskProfile.globals.find(
            (entry) => entry.dimension === "width");
        assert.deepEqual([storedWidth.minimum, storedWidth.maximum,
            storedWidth.mean, storedWidth.median,
            storedWidth.standard_deviation], [2000, 2600, 2300, 2300, 100]);
        b.elements.get("handwriting-local-load").dispatch("click");
        await tick();
        await tick();
        edit(b.elements, "handwriting-dimension", "width", "change");
        assert.equal(b.elements.get("handwriting-mean").value, "2300");
        assert.equal(b.elements.get("handwriting-mean-range").value, "2300");
        assert.match(b.elements.get("handwriting-preview").textContent,
            /range 2000…2600, mean 2300, median 2300, deviation 100/);
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
