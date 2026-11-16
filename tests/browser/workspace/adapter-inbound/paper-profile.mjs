// Copyright © 2026 Alberto Villa Osorno. SPDX-License-Identifier: MIT
// Paper-only form adapter: exercises authenticated Rust requests and
// local files.
import assert from "node:assert/strict";
import test from "node:test";
// jig-ignore-next-line: Static ESM imports need an intact literal path.
import { createPaperProfileEditor, readBoundedPaperExport } from "../../../../src/browser/workspace/adapter-inbound/generated/paper-profile.js";

const numericIds = ["paper-width", "paper-height", "paper-spacing",
    "paper-upper", "paper-bottom", "paper-left", "paper-right"];
const otherIds = ["paper-preset", "paper-pattern", "paper-color", "paper-load",
    "paper-reset", "paper-save", "paper-export", "paper-export-png",
    "paper-png-dpi", "paper-png-dpi-range", "paper-status"];
const defaults = {
    schema: "atrament.paper.v1",
    style: "grid",
    width_um: 210_000,
    height_um: 297_000,
    spacing_um: 5_000,
    margin_top_um: 22_000,
    margin_bottom_um: 16_000,
    margin_left_um: 20_000,
    margin_right_um: 14_000,
    line_color: "#C5D4E7",
};

class FakeControl {
    disabled = true;
    value = "";
    textContent = "";
    validity = { valid: true };
    files = null;
    callbacks = new Map();
    addEventListener(type, callback) { this.callbacks.set(type, callback); }
    dispatch(type) { this.callbacks.get(type)?.(); }
}

function setupControls() {
    const elements = new Map();
    for (const id of [...otherIds, ...numericIds,
        ...numericIds.map((id) => `${id}-range`)]) {
        elements.set(id, new FakeControl());
    }
    elements.get("paper-png-dpi").value = "150";
    elements.get("paper-png-dpi-range").value = "150";
    return elements;
}

test("paper editor exports Rust PDF and PNG", async () => {
    const original = {
        document: globalThis.document,
        fetch: globalThis.fetch,
        window: globalThis.window,
        createObjectURL: URL.createObjectURL,
        revokeObjectURL: URL.revokeObjectURL,
    };
    const elements = setupControls();
    const links = [];
    const requests = [];
    const urls = [];
    let secret = "local-session";
    const controller = new AbortController();
    globalThis.document = {
        getElementById: (id) => elements.get(id) ?? null,
        createElement: (tag) => {
            assert.equal(tag, "a");
            const link = { href: "", download: "", rel: "", click() {
                links.push({ href: this.href, download: this.download });
            } };
            return link;
        },
    };
    globalThis.window = { setTimeout: () => 0 };
    URL.createObjectURL = () => {
        const url = `blob:local-paper-${urls.length}`;
        urls.push(url);
        return url;
    };
    URL.revokeObjectURL = (url) => { urls.push(`revoke:${url}`); };
    globalThis.fetch = async (endpoint, init) => {
        requests.push({ endpoint, ...init });
        assert.equal(init.headers.Authorization, `Bearer ${secret}`);
        assert.equal(init.credentials, "omit");
        assert.equal(init.mode, "same-origin");
        if (init.method === "GET") {
            return new Response(JSON.stringify(defaults), {
                headers: { "Content-Type": "application/json" },
            });
        }
        assert.equal(init.headers["Content-Type"], "application/json");
        const profile = JSON.parse(init.body);
        assert.equal(profile.schema, defaults.schema);
        if (endpoint === "./api/paper/profile") {
            return new Response(JSON.stringify(profile), {
                headers: { "Content-Type": "application/json" },
            });
        }
        if (endpoint === "./api/paper/png") {
            assert.equal(init.headers["X-Atrament-Png-Dpi"], "200");
            return new Response(Uint8Array.of(137, 80, 78, 71), {
                headers: { "Content-Type": "image/png" },
            });
        }
        assert.equal(endpoint, "./api/paper/pdf");
        return new Response(new TextEncoder().encode("%PDF-1.4\n"), {
            headers: { "Content-Type": "application/pdf" },
        });
    };
    try {
        const editor = createPaperProfileEditor({
            currentSecret: () => secret,
            sessionSignal: controller.signal,
            unauthorized() { throw Error("unexpected 401"); },
        });
        assert.equal(elements.get("paper-export").disabled, true);
        assert.equal(elements.get("paper-export-png").disabled, true);
        await editor.start(secret);
        assert.equal(elements.get("paper-export").disabled, false);
        assert.equal(elements.get("paper-export-png").disabled, false);
        assert.equal(elements.get("paper-width").value, "210");
        assert.equal(elements.get("paper-width-range").value, "210");
        elements.get("paper-preset").value = "letter";
        elements.get("paper-preset").dispatch("change");
        assert.equal(elements.get("paper-width").value, "215.9");
        assert.equal(elements.get("paper-width-range").value, "215.9");
        elements.get("paper-spacing-range").value = "4.5";
        elements.get("paper-spacing-range").dispatch("input");
        assert.equal(elements.get("paper-spacing").value, "4.5");
        elements.get("paper-save").dispatch("click");
        await new Promise((resolve) => setImmediate(resolve));
        assert.equal(links[0].download, "atrament-paper.json");
        assert.equal(JSON.parse(requests.at(-1).body).spacing_um, 4500);
        const paperRequests = requests.length;
        const paperDownloads = links.length;
        for (const [field, raw] of [
            ["paper-width", "2.159e2"],
            ["paper-height", "0x117"],
            ["paper-spacing", "+4.5"],
            ["paper-upper", " 22"],
            ["paper-bottom", "16 "],
            ["paper-left", "2e1"],
            ["paper-right", "0xE"],
            ["paper-width", "215."],
            ["paper-height", ".5"],
            ["paper-spacing", "4.5000"],
            ["paper-upper", "22.0001"],
            ["paper-bottom", "16..0"],
            ["paper-left", "20.1234"],
            ["paper-right", "14."],
        ]) {
            const input = elements.get(field);
            const previous = input.value;
            input.value = raw;
            elements.get("paper-save").dispatch("click");
            await new Promise((resolve) => setImmediate(resolve));
            assert.equal(requests.length, paperRequests,
                `coerced millimetres ${field}=${raw} reached Rust`);
            assert.equal(links.length, paperDownloads);
            input.value = previous;
        }
        elements.get("paper-export").dispatch("click");
        await new Promise((resolve) => setImmediate(resolve));
        assert.equal(links[1].download, "atrament-paper.pdf");
        assert.equal(requests.at(-1).endpoint, "./api/paper/pdf");
        elements.get("paper-png-dpi-range").value = "200";
        elements.get("paper-png-dpi-range").dispatch("input");
        assert.equal(elements.get("paper-png-dpi").value, "200");
        elements.get("paper-export-png").dispatch("click");
        await new Promise((resolve) => setImmediate(resolve));
        assert.equal(links[2].download, "atrament-paper.png");
        assert.equal(requests.at(-1).endpoint, "./api/paper/png");
        assert.equal(requests.at(-1).headers["X-Atrament-Png-Dpi"],
            "200");
        const pngRequests = requests.length;
        elements.get("paper-png-dpi").value = "999";
        elements.get("paper-export-png").dispatch("click");
        await new Promise((resolve) => setImmediate(resolve));
        assert.equal(requests.length, pngRequests);
        for (const raw of ["+200", "2e2", "0xC8", "200.0", " 200",
            "200 "]) {
            elements.get("paper-png-dpi").value = raw;
            elements.get("paper-export-png").dispatch("click");
            await new Promise((resolve) => setImmediate(resolve));
            assert.equal(requests.length, pngRequests,
                `nondecimal DPI ${JSON.stringify(raw)} reached Rust`);
            assert.equal(links.length, 3,
                "invalid DPI cannot download another PNG");
        }
        elements.get("paper-png-dpi").value = "200";
        const fileText = JSON.stringify({ ...defaults, style: "ruled" });
        const corrupt = new TextEncoder().encode(fileText);
        corrupt[fileText.indexOf("ruled") + 2] = 0xFF;
        const beforeCorrupt = requests.length;
        elements.get("paper-load").files = {
            item: () => new Blob([corrupt]),
        };
        elements.get("paper-load").dispatch("change");
        await new Promise((resolve) => setImmediate(resolve));
        assert.equal(requests.length, beforeCorrupt,
            "invalid UTF-8 must not reach the Rust paper endpoint");
        assert.equal(elements.get("paper-pattern").value, "grid",
            "failed upload preserves the previously admitted paper");
        const prefixed = Uint8Array.of(0xEF, 0xBB, 0xBF,
            ...new TextEncoder().encode(fileText));
        elements.get("paper-load").files = {
            item: () => new Blob([prefixed]),
        };
        elements.get("paper-load").dispatch("change");
        await new Promise((resolve) => setImmediate(resolve));
        assert.equal(requests.length, beforeCorrupt,
            "native paper header refuses an initial UTF-8 BOM");
        assert.equal(elements.get("paper-pattern").value, "grid");
        elements.get("paper-load").files = {
            item: () => new Blob([new TextEncoder().encode(fileText)]),
        };
        elements.get("paper-load").dispatch("change");
        await new Promise((resolve) => setImmediate(resolve));
        assert.equal(elements.get("paper-pattern").value, "ruled");
        const notebookPaper = { ...defaults, style: "dot",
            spacing_um: 7000 };
        assert.equal(await editor.adoptJson(JSON.stringify(notebookPaper)),
            true, "notebook sheet admitted by Rust paper endpoint");
        assert.equal(elements.get("paper-pattern").value, "dot");
        assert.equal(elements.get("paper-spacing").value, "7");
        assert.equal(requests.at(-1).endpoint, "./api/paper/profile");
        const accepted = requests.length;
        assert.equal(await editor.adoptJson(JSON.stringify({
            ...notebookPaper, schema: "wrong",
        })), false, "forged paper response cannot alter visual editor");
        assert.equal(requests.length, accepted + 1);
        assert.equal(elements.get("paper-pattern").value, "dot");
        editor.reset();
        assert.equal(await editor.adoptJson(JSON.stringify(notebookPaper)),
            false, "ended session cannot load notebook paper");
        assert.equal(elements.get("paper-export").disabled, true);
        assert.equal(elements.get("paper-export-png").disabled, true);
        assert.equal(elements.get("paper-png-dpi").value, "150");
        assert.ok(urls.some((url) => url.startsWith("revoke:")));
        assert.equal(requests.filter(
            (r) => r.endpoint === "./api/paper/pdf",
        ).length, 1);
        const pngDownloads = requests.filter(
            (r) => r.endpoint === "./api/paper/png",
        );
        assert.equal(pngDownloads.length, 1);
    } finally {
        globalThis.document = original.document;
        globalThis.fetch = original.fetch;
        globalThis.window = original.window;
        URL.createObjectURL = original.createObjectURL;
        URL.revokeObjectURL = original.revokeObjectURL;
    }
});


test(
    "paper export streams exact binary bytes without Response.blob",
    async () => {
    const bytes = Uint8Array.of(0x89, 80, 78, 71, 0, 255, 128, 4);
    const reply = new Response(new ReadableStream({
        start(stream) {
            stream.enqueue(bytes.slice(0, 3));
            stream.enqueue(bytes.slice(3, 5));
            stream.enqueue(bytes.slice(5));
            stream.close();
        },
    }), { headers: { "Content-Length": "8" } });
    reply.blob = () => { throw Error("Response.blob must not be called"); };
    const image = await readBoundedPaperExport(reply, "image/png");
    assert.equal(image.type, "image/png");
    assert.deepEqual(new Uint8Array(await image.arrayBuffer()), bytes);
    const pdf = await readBoundedPaperExport(new Response("%PDF-1.7"),
        "application/pdf");
    assert.equal(pdf.type, "application/pdf");
    assert.equal(await pdf.text(), "%PDF-1.7");
});


test(
    "paper export rejects malformed declared length and incomplete payload",
    async () => {
    for (const length of ["0", "-1", "+4", "1e3", "4.0", "24000001",
        "18446744073709551615"]) {
        await assert.rejects(readBoundedPaperExport(new Response("four", {
            headers: { "Content-Length": length },
        }), "application/pdf"), /Invalid native paper export byte count/,
        "refused declared length " + length);
    }
    for (const length of ["3", "5"]) {
        await assert.rejects(readBoundedPaperExport(new Response("four", {
            headers: { "Content-Length": length },
        }), "application/pdf"), length === "3"
            ? /exceeds declared byte count/
            : /Incomplete native paper export response/);
    }
    await assert.rejects(readBoundedPaperExport(
        new Response(null), "image/png",
    ),
        /has no body/);
    await assert.rejects(readBoundedPaperExport(
        new Response(new Uint8Array()), "image/png"),
    /Incomplete native paper export response/);
});

test("paper export cancels oversized and rejects broken streams", async () => {
    let cancelled = false;
    const oversized = new Response(new ReadableStream({
        pull(stream) { stream.enqueue(new Uint8Array(24_000_001)); },
        cancel() { cancelled = true; },
    }));
    await assert.rejects(readBoundedPaperExport(oversized, "image/png"),
        /exceeds byte limit/);
    assert.equal(cancelled, true, "oversized stream must be cancelled");
    const broken = new Response(new ReadableStream({
        start(stream) {
            stream.enqueue(Uint8Array.of(1, 2, 3));
            stream.error(new Error("truncated native transport"));
        },
    }));
    await assert.rejects(readBoundedPaperExport(broken, "application/pdf"),
        /truncated native transport/);
});


test(
    "paper export cancels bad announced length before reading bytes",
    async () => {
    let pulls = 0;
    let cancelled = false;
    const reply = new Response(new ReadableStream({
        pull(stream) {
            pulls += 1;
            stream.enqueue(Uint8Array.of(7));
        },
        cancel() { cancelled = true; },
    }), { headers: { "Content-Length": "24000001" } });
    await assert.rejects(readBoundedPaperExport(reply, "image/png"),
        /Invalid native paper export byte count/);
    assert.equal(cancelled, true);
    assert.equal(pulls, 0, "declared overlong body is not consumed");
});

test(
    "paper export allows intermittent empty chunks after real progress",
    async () => {
    let cancelled = false;
    const bytes = Uint8Array.from({ length: 9 }, (_, index) => index + 1);
    let step = 0;
    const reply = new Response(new ReadableStream({
        pull(stream) {
            if (step === bytes.length * 9) {
                stream.close();
                return;
            }
            if (step % 9 === 8) {
                stream.enqueue(bytes.subarray(Math.floor(step / 9),
                    Math.floor(step / 9) + 1));
            } else {
                stream.enqueue(new Uint8Array());
            }
            step += 1;
        },
        cancel() { cancelled = true; },
    }), { headers: { "Content-Length": String(bytes.length) } });
    const result = await readBoundedPaperExport(reply, "image/png");
    assert.deepEqual(new Uint8Array(await result.arrayBuffer()), bytes);
    assert.equal(cancelled, false);
    assert.equal(step, bytes.length * 9,
        "empty chunks across successful reads cannot exhaust a stall budget");
});

test(
    "paper export refuses infinite empty chunks within a finite budget",
    async () => {
    let cancelled = false;
    let pulls = 0;
    const reply = new Response(new ReadableStream({
        pull(stream) {
            pulls += 1;
            stream.enqueue(new Uint8Array());
        },
        cancel() { cancelled = true; },
    }));
    await assert.rejects(readBoundedPaperExport(reply, "application/pdf"),
        /stalled on empty chunks/);
    assert.equal(cancelled, true);
    assert.ok(pulls <= 67, "empty data cannot spin indefinitely");
});
