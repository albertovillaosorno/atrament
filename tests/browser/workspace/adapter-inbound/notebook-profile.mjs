// Copyright © 2026 Alberto Villa Osorno. SPDX-License-Identifier: MIT
// Local notebook form adapter: native Rust JSON admission, no ambient state.
import assert from "node:assert/strict";
import test from "node:test";
import { createNotebookProfileEditor } from "../../../../src/browser/workspace/adapter-inbound/generated/paper-profile.js";

const ids = ["notebook-date-style", "notebook-date-placement",
    "notebook-radius", "notebook-radius-range", "notebook-page-numbers",
    "notebook-printed-sheet", "notebook-load", "notebook-use-paper",
    "notebook-edit-paper",
    "notebook-default", "notebook-save", "notebook-status",
    "notebook-export-pdf", "notebook-export-png",
    "notebook-pdf-pages", "notebook-png-dpi", "notebook-png-page"];
const paper = {
    schema: "atrament.paper.v1", style: "grid", width_um: 210_000,
    height_um: 297_000, spacing_um: 5_000, margin_top_um: 22_000,
    margin_bottom_um: 16_000, margin_left_um: 20_000,
    margin_right_um: 14_000, line_color: "#C5D4E7",
};
const defaults = {
    schema: "atrament.notebook.v1", paper,
    date_style: "none", date_placement: "none", border_radius_um: 0,
    page_numbers: false, printed_sheet: false,
};

class Control {
    value = "";
    checked = false;
    disabled = true;
    textContent = "";
    validity = { valid: true };
    files = null;
    callbacks = new Map();
    addEventListener(type, callback) { this.callbacks.set(type, callback); }
    dispatch(type) { this.callbacks.get(type)?.(); }
}

const settle = () => new Promise((resolve) => setImmediate(resolve));

test("notebook form validates page metadata and downloads native JSON", async () => {
    const saved = {
        document: globalThis.document,
        fetch: globalThis.fetch,
        window: globalThis.window,
        create: URL.createObjectURL,
        revoke: URL.revokeObjectURL,
    };
    const elements = new Map(ids.map((id) => [id, new Control()]));
    const downloads = [];
    const requests = [];
    const adoptedPapers = [];
    let secret = "session-local";
    let paperDraft = { ...paper, style: "ruled" };
    const controller = new AbortController();
    globalThis.document = {
        getElementById: (id) => elements.get(id) ?? null,
        createElement: () => ({ href: "", download: "", rel: "",
            click() { downloads.push(this.download); } }),
    };
    globalThis.window = { setTimeout: () => 0 };
    URL.createObjectURL = () => "blob:notebook-local";
    URL.revokeObjectURL = () => {};
    globalThis.fetch = async (url, init) => {
        requests.push({ url, init });
        assert.equal(init.headers.Authorization, `Bearer ${secret}`);
        if (url.startsWith("./api/notebook/paper/")) {
            assert.equal(init.method, "POST");
            assert.equal(init.headers["Content-Type"], "application/json");
            assert.equal(JSON.parse(init.body).printed_sheet, true);
            const pdf = url.endsWith("/pdf");
            assert.equal(
                pdf ? init.headers["X-Atrament-Pdf-Pages"]
                    : init.headers["X-Atrament-Png-Dpi"],
                pdf ? "3" : "72",
            );
            if (!pdf) {
                assert.equal(init.headers["X-Atrament-Png-Page"], "10");
            }
            return new Response(
                pdf ? "%PDF-1.7" : "\x89PNG",
                { headers: { "Content-Type":
                    pdf ? "application/pdf" : "image/png" } },
            );
        }
        assert.equal(url, "./api/notebook/profile");
        assert.equal(init.credentials, "omit");
        assert.equal(init.mode, "same-origin");
        assert.equal(init.cache, "no-store");
        assert.equal(init.redirect, "error");
        if (init.method === "GET") {
            return new Response(JSON.stringify(defaults), {
                headers: { "Content-Type": "application/json" },
            });
        }
        assert.equal(init.method, "POST");
        assert.equal(init.headers["Content-Type"], "application/json");
        const proposed = JSON.parse(init.body);
        // Rust returns the canonical representation in the real runtime.
        return new Response(JSON.stringify(proposed), {
            headers: { "Content-Type": "application/json" },
        });
    };
    try {
        const editor = createNotebookProfileEditor({
            currentSecret: () => secret,
            sessionSignal: controller.signal,
            unauthorized() { throw new Error("unexpected 401"); },
            paperDraft: () => JSON.stringify(paperDraft),
            adoptPaper: async (json) => {
                adoptedPapers.push(JSON.parse(json));
                return true;
            },
        });
        assert.equal(elements.get("notebook-save").disabled, true);
        await editor.start(secret);
        assert.equal(elements.get("notebook-save").disabled, false);
        assert.equal(elements.get("notebook-radius").value, "0");
        elements.get("notebook-radius-range").value = "3.5";
        elements.get("notebook-radius-range").dispatch("input");
        elements.get("notebook-date-style").value = "box";
        elements.get("notebook-date-placement").value = "top-right";
        elements.get("notebook-page-numbers").checked = true;
        elements.get("notebook-printed-sheet").checked = true;
        elements.get("notebook-edit-paper").dispatch("click");
        await settle();
        assert.equal(adoptedPapers.length, 1);
        assert.equal(adoptedPapers[0].style, "grid");
        assert.match(elements.get("notebook-status").textContent,
            /Use current paper/);
        elements.get("notebook-use-paper").dispatch("click");
        await settle();
        elements.get("notebook-edit-paper").dispatch("click");
        await settle();
        assert.equal(adoptedPapers[1].style, "ruled");
        const first = JSON.parse(requests.at(-1).init.body);
        assert.equal(first.paper.style, "ruled");
        assert.equal(first.border_radius_um, 3500);
        assert.equal(first.date_style, "box");
        assert.equal(first.date_placement, "top-right");
        assert.equal(first.page_numbers, true);
        assert.equal(first.printed_sheet, true);
        elements.get("notebook-save").dispatch("click");
        await settle();
        assert.deepEqual(downloads, ["atrament-notebook.json"]);
        assert.match(elements.get("notebook-status").textContent,
            /Validated notebook JSON downloaded/);
        const beforeExport = requests.length;
        elements.get("notebook-pdf-pages").value = "3";
        elements.get("notebook-png-dpi").value = "72";
        elements.get("notebook-png-page").value = "10";
        elements.get("notebook-export-pdf").dispatch("click");
        await settle();
        await settle();
        assert.equal(requests.length, beforeExport + 2,
            "PDF export canonicalizes notebook then calls native renderer");
        assert.deepEqual(downloads, ["atrament-notebook.json",
            "atrament-notebook-paper.pdf"]);
        elements.get("notebook-export-png").dispatch("click");
        await settle();
        await settle();
        assert.deepEqual(downloads, ["atrament-notebook.json",
            "atrament-notebook-paper.pdf", "atrament-notebook-page-10.png"]);
        assert.match(elements.get("notebook-status").textContent,
            /page numbers downloaded/);
        assert.match(elements.get("notebook-status").textContent,
            /PNG sheet 10/);
        assert.match(elements.get("notebook-status").textContent,
            /no date text, handwriting or images/);
        const beforeBadPng = requests.length;
        elements.get("notebook-png-page").value = "33";
        elements.get("notebook-export-png").dispatch("click");
        await settle();
        assert.equal(requests.length, beforeBadPng,
            "invalid selected PNG page refused before network use");
        assert.match(elements.get("notebook-status").textContent,
            /nothing saved/);
        const beforeInvalid = requests.length;
        elements.get("notebook-pdf-pages").value = "33";
        elements.get("notebook-export-pdf").dispatch("click");
        await settle();
        assert.equal(requests.length, beforeInvalid,
            "invalid export count refused before network effect");
        assert.equal(downloads.length, 3);
        const file = { size: 8193, text: async () => "{}" };
        elements.get("notebook-load").files = { item: () => file };
        elements.get("notebook-load").dispatch("change");
        assert.match(elements.get("notebook-status").textContent,
            /exceeds 8192 bytes/);
        assert.equal(downloads.length, 3);
        const count = requests.length;
        elements.get("notebook-date-style").value = "none";
        elements.get("notebook-save").dispatch("click");
        await settle();
        // Local mock accepts this, but native Rust rejects contradictory
        // date choices. The browser must never claim to own admission.
        assert.equal(requests.length, count + 1);
        secret = null;
        editor.reset();
        assert.equal(elements.get("notebook-save").disabled, true);
    } finally {
        controller.abort();
        globalThis.document = saved.document;
        globalThis.fetch = saved.fetch;
        globalThis.window = saved.window;
        URL.createObjectURL = saved.create;
        URL.revokeObjectURL = saved.revoke;
    }
});
