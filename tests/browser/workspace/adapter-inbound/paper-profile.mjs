// Copyright © 2026 Alberto Villa Osorno. SPDX-License-Identifier: MIT
// Paper-only form adapter: exercises authenticated Rust requests and local files.
import assert from "node:assert/strict";
import test from "node:test";
import { createPaperProfileEditor } from "../../../../src/browser/workspace/adapter-inbound/generated/paper-profile.js";

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
        const fileText = JSON.stringify({ ...defaults, style: "ruled" });
        elements.get("paper-load").files = { item: () => ({
            size: fileText.length, text: async () => fileText,
        }) };
        elements.get("paper-load").dispatch("change");
        await new Promise((resolve) => setImmediate(resolve));
        assert.equal(elements.get("paper-pattern").value, "ruled");
        editor.reset();
        assert.equal(elements.get("paper-export").disabled, true);
        assert.equal(elements.get("paper-export-png").disabled, true);
        assert.equal(elements.get("paper-png-dpi").value, "150");
        assert.ok(urls.some((url) => url.startsWith("revoke:")));
        assert.equal(requests.filter((r) => r.endpoint === "./api/paper/pdf").length, 1);
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
