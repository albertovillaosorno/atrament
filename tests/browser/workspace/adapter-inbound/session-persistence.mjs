// Copyright:
//   - Copyright © 2026 Alberto Villa Osorno.
// SPDX-License-Identifier:
//   - MIT
// Confidential:
//   - false
// License-File:
//   - LICENSE-MIT
//
// Boundary-Contract:
// - Owns:
//   - Regression evidence for browser persistence and page-exit disposal
//     policy.
// - Must-Not:
//   - Execute browser storage, perform network requests, or inspect user data.
// - Allows:
//   - Inputs: The tracked generated workspace JavaScript module.
//   - Outputs: Assertions that persistence-capable browser APIs are absent.
//   - Side effects: Reads one repository test artifact.
// - Split-When:
//   - Browser persistence policy gains independently admitted storage classes.
// - Merge-When:
//   - Another executable browser policy fixture subsumes this static evidence.
// - Summary:
//   - Guards browser persistence absence and mandatory page-exit cleanup.
// - Description:
//   - Fails when generated workspace code starts using persistent browser APIs.
// - Usage:
//   - Execute through the repository frontend test script after generation.
// - Defaults:
//   - Treats Web Storage, IndexedDB, cookies, caches, and workers as forbidden.
//
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const MAIN_MODULE = new URL(
    "../../../../src/browser/workspace/adapter-inbound/generated/main.js",
    import.meta.url,
);

test("workspace module contains no browser persistence API", async () => {
    const source = await readFile(MAIN_MODULE, "utf8");
    const forbidden = [
        "localStorage",
        "sessionStorage",
        "indexedDB",
        "document.cookie",
        "serviceWorker",
        "CacheStorage",
        "caches.open",
    ];
    for (const capability of forbidden) {
        assert.equal(
            source.includes(capability),
            false,
            `generated workspace uses ${capability}`,
        );
    }
});


test("page exit invalidates credential, work, and session text", async () => {
    const source = await readFile(MAIN_MODULE, "utf8");
    const start = source.indexOf(
        'window.addEventListener("pagehide", (event) => {',
    );
    assert.notEqual(start, -1, "generated workspace must handle pagehide");
    const end = source.indexOf("\n});", start);
    assert.notEqual(end, -1, "pagehide handler must have a bounded body");
    const handler = source.slice(start, end);
    for (const required of [
        "sessionSecret = null;",
        "sessionRequests.abort();",
        "invalidateClipboardRequests();",
        "invalidateDraftSync();",
        "clearSessionText();",
        "if (event.persisted)",
        "scrubBfcacheSubtree(workspace);",
    ]) {
        assert.equal(
            handler.includes(required),
            true,
            `pagehide handler must retain ${required}`,
        );
    }
});


test("page session network requests are cancellable on exit", async () => {
    const source = await readFile(MAIN_MODULE, "utf8");
    const signalUses = source.match(/signal: sessionRequests\.signal,/gu) ?? [];
    assert.equal(
        signalUses.length,
        2,
        "handshake and draft requests must share the page-session signal",
    );
    const draftStart = source.indexOf("async function syncDraftField(");
    const draftEnd = source.indexOf("function bindDraftSync(", draftStart);
    assert.notEqual(draftStart, -1, "draft sync function must exist");
    assert.notEqual(draftEnd, -1, "draft sync function must be bounded");
    const draftSync = source.slice(draftStart, draftEnd);
    const offlineStatus = draftSync.indexOf('"Draft offline · retry edit"');
    const staleGuard = draftSync.lastIndexOf(
        "draftSyncGeneration === generation",
        offlineStatus,
    );
    assert.notEqual(offlineStatus, -1, "draft fetch failure status must exist");
    assert.notEqual(
        staleGuard,
        -1,
        "draft fetch failure must ignore invalidated generations",
    );
    const start = source.indexOf(
        'window.addEventListener("pagehide", (event) => {',
    );
    const end = source.indexOf("\n});", start);
    const handler = source.slice(start, end);
    assert.ok(
        handler.indexOf("sessionRequests.abort();")
            < handler.indexOf("clearSessionText();"),
        "page exit must abort requests before clearing session text",
    );
});


test("session credential fragment is one-time browser handoff", async () => {
    const source = await readFile(MAIN_MODULE, "utf8");
    const urlStart = source.indexOf("function fragmentFreeLocalUrl() {");
    assert.notEqual(urlStart, -1, "workspace must derive a fragment-free URL");
    const handoffStart = source.indexOf(
        "function consumeSessionSecretFragment() {",
    );
    assert.notEqual(
        handoffStart,
        -1,
        "workspace must consume launch credential",
    );
    const handoffEnd = source.indexOf(
        "\nlet sessionSecret = consumeSessionSecretFragment();",
        handoffStart,
    );
    assert.notEqual(
        handoffEnd,
        -1,
        "credential handoff must have bounded body",
    );
    const urlHelper = source.slice(urlStart, handoffStart);
    assert.equal(urlHelper.includes("window.location.pathname"), true);
    assert.equal(urlHelper.includes("window.location.search"), true);
    assert.equal(urlHelper.includes("window.location.hash"), false);

    const handoff = source.slice(handoffStart, handoffEnd);
    for (const required of [
        'const hash = window.location.hash;',
        'hash.startsWith("#session=")',
        "const localUrl = fragmentFreeLocalUrl();",
        'window.history.replaceState(window.history.state, "", localUrl);',
        "window.location.replace(localUrl);",
        "return sessionSecret;",
    ]) {
        assert.equal(
            handoff.includes(required),
            true,
            `credential handoff must retain ${required}`,
        );
    }
});
