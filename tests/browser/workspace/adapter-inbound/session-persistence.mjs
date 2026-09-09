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
//   - Inputs: The tracked static workspace HTML and generated JavaScript
//     module.
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

const INDEX_HTML = new URL(
    "../../../../src/browser/workspace/adapter-inbound/index.html",
    import.meta.url,
);
const MAIN_MODULE = new URL(
    "../../../../src/browser/workspace/adapter-inbound/generated/main.js",
    import.meta.url,
);

test("static workspace starts session controls disabled", async () => {
    const source = await readFile(INDEX_HTML, "utf8");
    for (const id of [
        "copy-prompt",
        "task-input",
        "source-input",
        "profile-select",
        "paper-select",
        "style-select",
        "output-select",
        "prompt-output",
        "candidate-input",
        "zoom-out",
        "zoom-reset",
        "zoom-in",
    ]) {
        const idIndex = source.indexOf(`id="${id}"`);
        assert.notEqual(idIndex, -1, `static workspace must contain #${id}`);
        const tagStart = source.lastIndexOf("<", idIndex);
        const tagEnd = source.indexOf(">", idIndex);
        assert.ok(tagStart >= 0 && tagEnd > idIndex, `#${id} tag must close`);
        const tag = source.slice(tagStart, tagEnd + 1);
        assert.equal(
            tag.includes("disabled"),
            true,
            `#${id} must be disabled before JavaScript starts`,
        );
    }
});

test(
    "workspace module has no undeclared outbound browser transport",
    async () => {
    const source = await readFile(MAIN_MODULE, "utf8");
    const fetchCalls = source.match(/\bfetch\(/gu) ?? [];
    assert.equal(
        fetchCalls.length,
        3,
        "only handshake and draft fetches exist",
    );

    for (const required of [
        'cache: "no-store"',
        'credentials: "omit"',
        'mode: "same-origin"',
        'redirect: "error"',
        'referrerPolicy: "no-referrer"',
    ]) {
        const occurrences = source.split(required).length - 1;
        assert.equal(
            occurrences,
            3,
            `all admitted fetches must retain ${required}`,
        );
    }

    for (const forbidden of [
        "http://",
        "https://",
        "navigator.sendBeacon",
        "XMLHttpRequest",
        "new WebSocket",
        "new EventSource",
    ]) {
        assert.equal(
            source.includes(forbidden),
            false,
            `generated workspace uses undeclared transport ${forbidden}`,
        );
    }
    },
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


test(
    "authorization loss invalidates the complete browser session",
    async () => {
        const source = await readFile(MAIN_MODULE, "utf8");
        const start = source.indexOf(
            "function invalidateUnauthorizedSession() {",
        );
        const end = source.indexOf("\n}", start);
        assert.notEqual(
            start,
            -1,
            "authorization invalidation helper must exist",
        );
        assert.notEqual(
            end,
            -1,
            "authorization invalidation helper must be bounded",
        );
        const invalidation = source.slice(start, end);
        const required = [
            "sessionSecret = null;",
            "sessionRequests.abort();",
            "invalidateClipboardRequests();",
            "invalidateDraftSync();",
            "clearSessionText();",
            'setTextIfChanged(sessionStatus, "Authorization failed");',
        ];
        let prior = -1;
        for (const operation of required) {
            const position = invalidation.indexOf(operation);
            assert.ok(
                position > prior,
                `authorization loss must perform ${operation}`,
            );
            prior = position;
        }
        const calls =
            source.match(/invalidateUnauthorizedSession\(\);/gu) ?? [];
        assert.equal(
            calls.length,
            3,
            "handshake, hydration, and draft-write authorization loss "
                + "must invalidate",
        );
    },
);


test(
    "draft sync status stays stale-safe and failure-aware",
    async () => {
        const source = await readFile(MAIN_MODULE, "utf8");
        const start = source.indexOf("async function syncDraftField(");
        const end = source.indexOf("function bindDraftSync(", start);
        assert.notEqual(start, -1, "draft sync function must exist");
        assert.notEqual(end, -1, "draft sync function must be bounded");
        const draftSync = source.slice(start, end);
        const responseJson = draftSync.indexOf("await response.json();");
        const jsonCatch = draftSync.indexOf("catch {", responseJson);
        const catchCurrent = draftSync.indexOf(
            "if (draftSyncIsCurrent(secret, generation))",
            jsonCatch,
        );
        const staleAfterJson = draftSync.indexOf(
            "if (!draftSyncIsCurrent(secret, generation))",
            jsonCatch,
        );
        const resourceStatus = draftSync.indexOf(
            '"Draft too large · reduce"',
            responseJson,
        );
        assert.ok(
            responseJson < jsonCatch
                && jsonCatch < catchCurrent
                && catchCurrent < staleAfterJson
                && staleAfterJson < resourceStatus,
            "413 completion paths must recheck page-session freshness",
        );
        assert.equal(
            draftSync.includes("failedDraftFields.delete(field);"),
            true,
            "a successful field sync must clear only that field failure",
        );
        assert.equal(
            draftSync.includes("failedDraftFields.size === 0"),
            true,
            "ready status must require no outstanding field failures",
        );
        assert.equal(
            draftSync.includes("syncingDraftFields.size === 0"),
            true,
            "ready status must require all field syncs to settle",
        );
    },
);


test("page session network requests are cancellable on exit", async () => {
    const source = await readFile(MAIN_MODULE, "utf8");
    const signalUses = source.match(/signal: sessionRequests\.signal,/gu) ?? [];
    assert.equal(
        signalUses.length,
        3,
        "handshake, draft reads, and writes must share the page-session signal",
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


test(
    "handshake rejects status before reading an irrelevant body",
    async () => {
        const source = await readFile(MAIN_MODULE, "utf8");
        const start = source.indexOf(
            "async function completeSessionHandshake(secret) {",
        );
        const end = source.indexOf(
            "const failedDraftFields",
            start,
        );
        assert.notEqual(start, -1, "handshake function must exist");
        assert.notEqual(end, -1, "handshake function must be bounded");
        const handshake = source.slice(start, end);
        const unauthorized = handshake.indexOf(
            "if (response.status === 401)",
        );
        const admittedStatuses = handshake.indexOf(
            "response.status !== 200 && response.status !== 409",
        );
        const bodyRead = handshake.indexOf("await response.json();");
        assert.ok(
            unauthorized !== -1
                && admittedStatuses !== -1
                && bodyRead !== -1
                && unauthorized < bodyRead
                && admittedStatuses < bodyRead,
            "401 and unrelated statuses must settle before JSON body parsing",
        );
    },
);


test("draft hydration is atomic and stale-page guarded", async () => {
    const source = await readFile(MAIN_MODULE, "utf8");
    const start = source.indexOf(
        "async function hydrateSessionDraft(secret) {",
    );
    const end = source.indexOf("async function syncDraftField(", start);
    assert.notEqual(start, -1, "draft hydration function must exist");
    assert.notEqual(end, -1, "draft hydration function must be bounded");
    const hydration = source.slice(start, end);
    const snapshotPush = hydration.indexOf("snapshot.push(");
    const snapshotApply = hydration.indexOf("for (const [input, count, value]");
    const enable = hydration.indexOf("enableCompatibleEditing();");
    assert.notEqual(snapshotPush, -1, "hydration must stage draft text");
    assert.ok(
        snapshotPush < snapshotApply && snapshotApply < enable,
        "all draft values must stage before DOM commit and editing enablement",
    );
    assert.equal(
        hydration.includes("sessionSecret !== secret"),
        true,
        "hydration must reject stale credentials",
    );
    assert.equal(
        hydration.includes("draftSyncGeneration !== generation"),
        true,
        "hydration must reject invalidated page generations",
    );
    const handshakeStart = source.indexOf(
        "async function completeSessionHandshake(secret) {",
    );
    const handshakeEnd = source.indexOf(
        "const syncingDraftFields",
        handshakeStart,
    );
    assert.notEqual(handshakeStart, -1, "handshake function must exist");
    assert.notEqual(handshakeEnd, -1, "handshake function must be bounded");
    const handshake = source.slice(handshakeStart, handshakeEnd);
    assert.equal(
        handshake.includes("await hydrateSessionDraft(secret);"),
        true,
        "compatible handshake must await draft hydration before editing",
    );
    assert.equal(
        handshake.includes("enableCompatibleEditing();"),
        false,
        "handshake must not enable editing before hydration commits",
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
