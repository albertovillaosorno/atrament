// Copyright © 2026 Alberto Villa Osorno. SPDX-License-Identifier: LicenseRef-Proprietary
// Local notebook form adapter: native Rust JSON admission, no ambient state.
import assert from "node:assert/strict";
import test from "node:test";
import { createNotebookProfileEditor } from "../../../../src/browser/workspace/adapter-inbound/generated/paper-profile.js";

const ids = ["notebook-date-style", "notebook-date-placement",
    "notebook-date-text", "notebook-radius", "notebook-radius-range",
    "notebook-page-numbers", "notebook-printed-sheet", "notebook-load",
    "notebook-use-paper",
    "notebook-edit-paper",
    "notebook-default", "notebook-save", "notebook-status",
    "notebook-local-name", "notebook-local-load", "notebook-local-save",
    "notebook-export-pdf", "notebook-export-png", "notebook-preview",
    "notebook-preview-frame", "notebook-preview-image",
    "notebook-pdf-pages", "notebook-png-dpi", "notebook-png-page"];
const paper = {
    schema: "atrament.paper.v1", style: "grid", width_um: 210_000,
    height_um: 297_000, spacing_um: 5_000, margin_top_um: 22_000,
    margin_bottom_um: 16_000, margin_left_um: 20_000,
    margin_right_um: 14_000, line_color: "#C5D4E7",
};
const defaults = {
    schema: "atrament.notebook.v1", paper,
    date_style: "none", date_placement: "none", date_text: "",
    border_radius_um: 0,
    page_numbers: false, printed_sheet: false,
};

class Control {
    value = "";
    checked = false;
    disabled = true;
    textContent = "";
    src = "";
    alt = "";
    hidden = false;
    validity = { valid: true };
    files = null;
    callbacks = new Map();
    addEventListener(type, callback) { this.callbacks.set(type, callback); }
    removeAttribute(name) { if (name === "src") this.src = ""; }
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
    elements.get("notebook-local-name").value = "my-notebook.json";
    const downloads = [];
    const revoked = [];
    const requests = [];
    const adoptedPapers = [];
    let diskRevision = null;
    let diskProfile = null;
    let revisionStep = 0;
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
    URL.revokeObjectURL = (url) => { revoked.push(url); };
    globalThis.fetch = async (url, init) => {
        requests.push({ url, init });
        assert.equal(init.headers.Authorization, `Bearer ${secret}`);
        if (url === "./api/notebook/file") {
            assert.equal(init.headers["X-Atrament-Profile-Name"],
                "my-notebook.json");
            assert.equal(init.credentials, "omit");
            assert.equal(init.cache, "no-store");
            assert.equal(init.mode, "same-origin");
            assert.equal(init.redirect, "error");
            if (init.method === "GET") {
                return diskRevision === null
                    ? Response.json({ error: "profile_file_missing" },
                        { status: 404 })
                    : Response.json({
                        revision: diskRevision, profile: diskProfile,
                    });
            }
            assert.equal(init.headers["Content-Type"], "application/json");
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
        elements.get("notebook-date-text").value = "2026-11-03";
        elements.get("notebook-date-text").dispatch("input");
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
        assert.equal(first.date_text, "2026-11-03");
        assert.equal(first.page_numbers, true);
        assert.equal(first.printed_sheet, true);
        elements.get("notebook-save").dispatch("click");
        await settle();
        assert.deepEqual(downloads, ["atrament-notebook.json"]);
        assert.match(elements.get("notebook-status").textContent,
            /Validated notebook JSON downloaded/);
        elements.get("notebook-local-save").dispatch("click");
        await settle();
        await settle();
        assert.equal(requests.at(-1).url, "./api/notebook/file");
        assert.equal(requests.at(-1).init.headers[
            "X-Atrament-Profile-Revision"], "new");
        assert.equal(diskProfile.page_numbers, true);
        assert.equal(diskProfile.date_text, "2026-11-03");
        assert.match(elements.get("notebook-status").textContent,
            /saved in selected local directory/);
        elements.get("notebook-local-load").dispatch("click");
        await settle();
        await settle();
        assert.match(elements.get("notebook-status").textContent,
            /reopened from local Rust/);
        assert.equal(elements.get("notebook-radius").value, "3.5");
        assert.equal(elements.get("notebook-radius-range").value, "3.5");
        assert.equal(elements.get("notebook-date-style").value, "box");
        assert.equal(elements.get("notebook-date-placement").value,
            "top-right");
        assert.equal(elements.get("notebook-date-text").value, "2026-11-03");
        assert.equal(elements.get("notebook-page-numbers").checked, true);
        // The same browser form also retains Spain/Mexico day-first dates.
        elements.get("notebook-date-text").value = "03/11/2026";
        elements.get("notebook-date-text").dispatch("input");
        elements.get("notebook-printed-sheet").checked = false;
        elements.get("notebook-local-save").dispatch("click");
        await settle();
        await settle();
        assert.equal(diskProfile.printed_sheet, false);
        assert.equal(diskProfile.date_text, "03/11/2026");
        assert.equal(requests.at(-1).init.headers[
            "X-Atrament-Profile-Revision"], "sha256:" + "a".repeat(64));
        diskRevision = "sha256:" + "c".repeat(64);
        elements.get("notebook-local-save").dispatch("click");
        await settle();
        await settle();
        assert.match(elements.get("notebook-status").textContent,
            /changed; reopen/);
        elements.get("notebook-printed-sheet").checked = true;
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
            /no handwriting or images/);
        const beforePreview = requests.length;
        const previewFrame = elements.get("notebook-preview-frame");
        const previewImage = elements.get("notebook-preview-image");
        assert.equal(previewFrame.hidden, true);
        elements.get("notebook-preview").dispatch("click");
        await settle();
        await settle();
        assert.equal(requests.length, beforePreview + 2,
            "preview requests canonical metadata and native Rust PNG");
        assert.equal(previewFrame.hidden, false);
        assert.equal(previewImage.src, "blob:notebook-local");
        assert.equal(previewImage.alt, "Rendered notebook paper, page 10");
        assert.deepEqual(downloads, ["atrament-notebook.json",
            "atrament-notebook-paper.pdf", "atrament-notebook-page-10.png"],
            "preview does not download a file");
        elements.get("notebook-radius-range").value = "4";
        elements.get("notebook-radius-range").dispatch("input");
        assert.equal(previewFrame.hidden, true,
            "changing radius invalidates the native image preview");
        assert.equal(previewImage.src, "");
        assert.ok(revoked.includes("blob:notebook-local"));
        const beforeBadPng = requests.length;
        elements.get("notebook-png-page").value = "33";
        elements.get("notebook-export-png").dispatch("click");
        await settle();
        assert.equal(requests.length, beforeBadPng,
            "invalid selected PNG page refused before network use");
        elements.get("notebook-preview").dispatch("click");
        await settle();
        assert.equal(requests.length, beforeBadPng,
            "invalid preview page refused without transport");
        assert.equal(previewFrame.hidden, true);
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
        const validImport = JSON.stringify({
            ...defaults, date_style: "box", date_placement: "top-right",
            date_text: "04/11/2026",
        });
        const corrupt = new TextEncoder().encode(validImport);
        const dateToken = '"date_text":"04/11/2026"';
        const dateOffset = validImport.indexOf(dateToken);
        assert.ok(dateOffset >= 0);
        corrupt[dateOffset + '"date_text":"'.length] = 0xFF;
        const beforeCorrupt = requests.length;
        elements.get("notebook-load").files = {
            item: () => new Blob([corrupt]),
        };
        elements.get("notebook-load").dispatch("change");
        await settle();
        assert.equal(requests.length, beforeCorrupt,
            "invalid UTF-8 must not be forwarded as replacement text");
        assert.equal(elements.get("notebook-date-text").value, "03/11/2026",
            "invalid file leaves the prior notebook form untouched");
        elements.get("notebook-load").files = {
            item: () => new Blob([new TextEncoder().encode(validImport)]),
        };
        elements.get("notebook-load").dispatch("change");
        await settle();
        assert.equal(elements.get("notebook-date-text").value, "04/11/2026",
            "valid UTF-8 still imports the selected day-first date");
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
        assert.equal(elements.get("notebook-preview").disabled, true);
        assert.equal(previewFrame.hidden, true);
    } finally {
        controller.abort();
        globalThis.document = saved.document;
        globalThis.fetch = saved.fetch;
        globalThis.window = saved.window;
        URL.createObjectURL = saved.create;
        URL.revokeObjectURL = saved.revoke;
    }
});
