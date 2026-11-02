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
import { readFile, readdir } from "node:fs/promises";
import test from "node:test";
import { runInNewContext as runVmContext } from "node:vm";

// A browser owns these abort primitives. Node VM contexts omit them by
// default; expose the real primitives so extracted adapter tests also check
// that request abort signals are well-formed.
function runInNewContext(source, context) {
    return runVmContext(source, { AbortController, AbortSignal, ...context });
}

const INDEX_HTML = new URL(
    "../../../../src/browser/workspace/adapter-inbound/index.html",
    import.meta.url,
);
const MAIN_MODULE = new URL(
    "../../../../src/browser/workspace/adapter-inbound/generated/main.js",
    import.meta.url,
);
// Privacy review covers both authored TypeScript and generated JavaScript.
// Standalone Node tests can run before the TypeScript is rebuilt.
const REVIEWED_MODULES = [
    "main.js",
    "session-diagnostic.js",
    "session-draft.js",
    "session-fragment.js",
    "session-handshake.js",
];
const REVIEWED_TYPESCRIPT = REVIEWED_MODULES.map((filename) =>
    filename.replace(/\.js$/u, ".ts"));

async function browserModuleSources() {
    const generatedDirectory = new URL(".", MAIN_MODULE);
    const sourceDirectory = new URL("../", MAIN_MODULE);
    const generated = (await readdir(generatedDirectory)).sort();
    const typescript = (await readdir(sourceDirectory))
        .filter((filename) => filename.endsWith(".ts")).sort();
    assert.deepEqual(generated, REVIEWED_MODULES,
        "generated module inventory requires privacy review");
    assert.deepEqual(typescript, REVIEWED_TYPESCRIPT,
        "TypeScript module inventory requires privacy review");
    const moduleUrls = [
        ...generated.map((filename) =>
            [filename, new URL(filename, generatedDirectory)]),
        ...typescript.map((filename) =>
            [filename, new URL(filename, sourceDirectory)]),
    ];
    return Promise.all(moduleUrls.map(async ([filename, url]) => ({
        filename,
        source: await readFile(url, "utf8"),
    })));
}

const {
    isDraftReadContentType,
    decodeDraftReadResponse,
} = await import(
    new URL("session-draft.js", MAIN_MODULE).href,
);
const { parseBoundedJsonResponse } = await import(
    new URL("session-diagnostic.js", MAIN_MODULE).href,
);
const { parseHandshakePayload } = await import(
    new URL("session-handshake.js", MAIN_MODULE).href,
);

test("static workspace starts session controls disabled", async () => {
    const source = await readFile(INDEX_HTML, "utf8");
    for (const id of [
        "copy-prompt",
        "end-session",
        "retry-session",
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

test("browser source and generated modules reject undeclared transport",
    async () => {
    for (const { filename, source } of await browserModuleSources()) {
        const main = filename === "main.ts" || filename === "main.js";
        const fetchCalls = source.match(/\bfetch\s*\(/gu) ?? [];
        assert.equal(fetchCalls.length, main ? 4 : 0,
            `${filename} must retain only four reviewed fetch sites`);
        if (main) {
            for (const required of [
                'cache: "no-store"',
                'credentials: "omit"',
                'mode: "same-origin"',
                'redirect: "error"',
            ]) {
                const occurrences = source.split(required).length - 1;
                assert.equal(occurrences, 4,
                    `${filename} must retain ${required} on all fetches`);
            }
            assert.ok(source.includes('fetch("./api/session/end"'),
                `${filename}: session end stays explicit and same-origin`);
            assert.equal(source.includes('keepalive: true'), false,
                `${filename}: session end requires live page authority`);
            assert.equal(
                source.split('referrerPolicy: "origin"').length - 1, 3,
                `${filename}: POST referrers disclose only the origin`,
            );
            assert.equal(
                source.split('referrerPolicy: "no-referrer"').length - 1, 1,
                `${filename}: draft reads never send referrers`,
            );
        }
        for (const forbidden of [
            "http://",
            "https://",
            "sendBeacon",
            "XMLHttpRequest",
            "WebSocket",
            "EventSource",
            "WebTransport",
            "RTCPeerConnection",
        ]) {
            assert.equal(source.includes(forbidden), false,
                `${filename} uses undeclared transport ${forbidden}`);
        }
    }
});

test("browser source and generated modules reject persistence APIs",
    async () => {
    const forbidden = [
        "localStorage",
        "sessionStorage",
        "indexedDB",
        "document.cookie",
        "serviceWorker",
        "CacheStorage",
        "caches.",
        "navigator.storage",
        "window.name",
    ];
    for (const { filename, source } of await browserModuleSources()) {
        for (const capability of forbidden) {
            assert.equal(source.includes(capability), false,
                `${filename} uses ${capability}`);
        }
    }
});


test("browser modules reject unreviewed cross-context authority", async () => {
    // Workers and cross-tab channels can outlive a session page or perform
    // network activity outside the four reviewed client fetch sites.
    const forbidden = [
        "Worker",
        "SharedWorker",
        "BroadcastChannel",
        "MessageChannel",
        "MessagePort",
        "postMessage",
        "window.open(",
    ];
    for (const { filename, source } of await browserModuleSources()) {
        for (const capability of forbidden) {
            assert.equal(source.includes(capability), false,
                `${filename} uses unreviewed cross-context ${capability}`);
        }
    }
});

// Browser modules need the user's clipboard only to write the deliberate
// Copy prompt result. Reads of clipboard contents, device identifiers or
// permission-bearing host resources are not part of the admitted session.
// This is an exact source inventory, not a browser permission sandbox.
function usesUnreviewedBrowserHostInput(source, main) {
    const hostProperty = String.raw`(?:geolocation|mediaDevices|credentials|` +
        String.raw`hardwareConcurrency|deviceMemory|userAgent(?:Data)?|` +
        String.raw`connection|getBattery|getGamepads|bluetooth|usb|` +
        String.raw`serial|hid|gpu|permissions)`;
    const ambientRead = new RegExp(
        String.raw`\bnavigator\s*(?:(?:\?\.|\.)\s*|(?:\?\.)?\[\s*['"])` +
            hostProperty + String.raw`\b`,
        "u",
    );
    if (ambientRead.test(source)
        || /\breadText\b/u.test(source)
        || /\bclipboard\s*(?:\?\.|\.)\s*read\b/u.test(source)
        || /\bclipboard\s*(?:\?\.)?\[\s*['"]read['"]\s*\]/u
            .test(source)) {
        return true;
    }
    const clipboardAccesses = [...source.matchAll(new RegExp(
        String.raw`\bnavigator\s*(?:\?\.|\.)\s*clipboard\b|` +
            String.raw`\bnavigator\s*(?:\?\.)?\[\s*` +
            String.raw`['"]clipboard['"]\s*\]`,
        "gu",
    ))];
    const clipboardWrites = [...source.matchAll(
        /\bclipboard\s*\?\.\s*writeText\b/gu,
    )];
    return clipboardAccesses.length !== (main ? 1 : 0)
        || clipboardWrites.length !== (main ? 1 : 0);
}

test("browser host-input inventory refuses unreviewed reads", () => {
    const allowedWrite = "const clipboard = navigator.clipboard; " +
        "const writeText = clipboard?.writeText;";
    assert.equal(usesUnreviewedBrowserHostInput(allowedWrite, true), false);
    assert.equal(usesUnreviewedBrowserHostInput("export const x = 1;", false),
        false);
    for (const candidate of [
        "navigator.clipboard.readText()",
        "clipboard.readText()",
        "clipboard?.read()",
        "clipboard['read']()",
        "clipboard?.['read']()",
        "navigator['clipboard'].read()",
        "navigator.geolocation.getCurrentPosition(callback)",
        "navigator?.mediaDevices.getUserMedia({ audio: true })",
        "navigator.hardwareConcurrency",
        "navigator['hardwareConcurrency']",
        "navigator?.['hardwareConcurrency']",
        "navigator . userAgentData",
        "navigator.deviceMemory",
        "navigator.connection",
        "navigator.getBattery()",
        "navigator.permissions.query({ name: 'camera' })",
        "navigator.credentials.get({ password: true })",
        "navigator.gpu.requestAdapter()",
        "navigator.usb.requestDevice({ filters: [] })",
    ]) {
        assert.equal(usesUnreviewedBrowserHostInput(
            allowedWrite + candidate, true), true, candidate);
        assert.equal(usesUnreviewedBrowserHostInput(candidate, false),
            true, candidate);
    }
});

test("browser source and emitted modules reject unreviewed host reads",
    async () => {
    for (const { filename, source } of await browserModuleSources()) {
        const main = filename === "main.ts" || filename === "main.js";
        assert.equal(usesUnreviewedBrowserHostInput(source, main), false,
            `${filename} gained clipboard or ambient device read authority`);
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


test("bfcache restore cannot resume an invalidated page session", async () => {
    const source = await readFile(MAIN_MODULE, "utf8");
    const start = source.indexOf(
        'window.addEventListener("pageshow", (event) => {',
    );
    assert.notEqual(start, -1, "generated workspace must handle pageshow");
    const end = source.indexOf("\n});", start);
    assert.notEqual(end, -1, "pageshow handler must have a bounded body");
    const handler = source.slice(start, end);
    const persisted = handler.indexOf("if (event.persisted)");
    const reload = handler.indexOf("window.location.reload();", persisted);
    const returnAfterReload = handler.indexOf("return;", reload);
    const ordinaryRestore = handler.indexOf(
        "const hadLocalNavigationFragment",
        returnAfterReload,
    );
    assert.ok(
        persisted !== -1
            && reload > persisted
            && returnAfterReload > reload
            && ordinaryRestore > returnAfterReload,
        "persisted pages must reload before ordinary pageshow restoration",
    );
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
            4,
            "handshake, hydration, draft-write, and end authorization "
                + "loss must invalidate",
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
        const responseJson = draftSync.indexOf(
            "await parseBoundedJsonResponse(response);",
        );
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


test("end-session warning survives drafts that settle afterward", async () => {
    const source = await readFile(MAIN_MODULE, "utf8");
    const extract = (start, end) => {
        const first = source.indexOf(start);
        const last = source.indexOf(end, first);
        assert.ok(first >= 0 && last > first, `missing ${start}`);
        return source.slice(first, last);
    };
    const failure = extract(
        "function recordDraftSyncFailure(",
        "function invalidateUnauthorizedSession()",
    );
    const sync = extract(
        "async function syncDraftField(",
        "function bindDraftSync(",
    );
    const end = extract(
        "let endSessionConfirmed = false;",
        'window.addEventListener("pagehide"',
    );
    for (const [phase, draftStatus, expected] of [
        ["armed", 204, "Ending discards this notebook"],
        ["armed", 503, "Ending discards this notebook"],
        ["armed", 413, "Ending discards this notebook"],
        ["failed", 204, "End session failed · retry"],
        ["failed", 503, "End session failed · retry"],
    ]) {
        const result = await runInNewContext(`
            (async () => {
                let sessionSecret = "secret";
                const sessionRequests = new AbortController();
                const draftSyncGeneration = 0;
                const failedDraftFields = new Set();
                const retryableDraftFields = new Set();
                const syncingDraftFields = new Set();
                const sessionStatus = { textContent: "Session ready" };
                const draftInput = { value: "in-flight private draft" };
                const handlers = {};
                const endSession = {
                    disabled: false,
                    textContent: "End session",
                    addEventListener(type, handler) {
                        handlers[type] = handler;
                    },
                };
                let finishDraft;
                const pendingDraft = new Promise((resolve) => {
                    finishDraft = resolve;
                });
                const fetch = (url) => url.endsWith("/task")
                    ? pendingDraft
                    : Promise.resolve(new Response(null, { status: 503 }));
                const sessionDraft = {
                    draftMutationTarget: () => "./api/session/task",
                    draftMutationHeaders: () => ({}),
                    draftReadHeaders: () => ({}),
                    isDraftDiagnosticContentType: () => true,
                    isResourceLimit: () => false,
                };
                function draftSyncIsCurrent(secret, generation) {
                    return sessionSecret === secret
                        && draftSyncGeneration === generation;
                }
                function setTextIfChanged(element, value) {
                    element.textContent = value;
                }
                function showSessionRetry() {}
                function hideSessionRetry() {}
                function hideResolvedDraftRetry() {}
                function discardResponseBody() {}
                function invalidateUnauthorizedSession() {}
                function invalidateClipboardRequests() {}
                function invalidateDraftSync() {}
                function clearSessionText() {}
                const parseBoundedJsonResponse = async () => null;
                ${failure}
                ${sync}
                ${end}
                const write = syncDraftField("task", draftInput, "secret");
                handlers.click();
                if (mode === "failed") {
                    handlers.click();
                }
                for (let step = 0; step < 20; step += 1) {
                    await Promise.resolve();
                }
                const before = sessionStatus.textContent;
                finishDraft(new Response(null, { status: writeStatus }));
                await write;
                const after = sessionStatus.textContent;
                if (mode === "armed") {
                    handlers.blur();
                }
                return {
                    before,
                    after,
                    afterDisarm: sessionStatus.textContent,
                    button: endSession.textContent,
                };
            })()
        `, {
            Response,
            mode: phase,
            writeStatus: draftStatus,
        });
        assert.equal(result.before, expected, `${phase} initial warning`);
        assert.equal(result.after, expected,
            `${phase} warning after draft HTTP ${draftStatus}`);
        if (phase === "armed") {
            const disarmedStatus = draftStatus === 204
                ? "Session ready" : draftStatus === 413
                    ? "Invalid draft diagnostic"
                    : "Draft sync rejected · retry edit";
            assert.equal(result.afterDisarm, disarmedStatus,
                `disarming must reveal draft HTTP ${draftStatus} state`);
        }
    }
});

test("retry control follows remaining retryable draft failures", async () => {
    const source = await readFile(MAIN_MODULE, "utf8");
    function extract(start, end) {
        const first = source.indexOf(start);
        const last = source.indexOf(end, first);
        assert.ok(first >= 0 && last > first, `missing ${start}`);
        return source.slice(first, last);
    }
    const retryControls = extract(
        "function showSessionRetry(purpose) {",
        "function clearSessionText() {",
    );
    const recordFailure = extract(
        "function hideResolvedDraftRetry() {",
        "function invalidateUnauthorizedSession() {",
    );
    const synchronize = extract(
        "async function syncDraftField(",
        "function bindDraftSync(",
    );
    for (const secondary of [
        { retryable: false, message: "Draft too large" },
        { retryable: true, message: "Draft offline source" },
    ]) {
        const result = await runInNewContext(`
        (async () => {
            let retryPurpose = null;
            const endSessionConfirmed = false;
            const endSessionFailurePending = false;
            const retrySession = { hidden: true, disabled: true };
            const sessionStatus = { textContent: "" };
            const sessionSecret = "secret";
            const draftSyncGeneration = 0;
            const failedDraftFields = new Set();
            const retryableDraftFields = new Set();
            const syncingDraftFields = new Set();
            const sessionRequests = new AbortController();
            const sessionDraft = {
                draftMutationTarget: () => "./api/session/task",
                draftMutationHeaders: () => ({}),
            };
            function setTextIfChanged(element, value) {
                element.textContent = value;
            }
            function draftSyncIsCurrent(secret, generation) {
                return sessionSecret === secret
                    && draftSyncGeneration === generation;
            }
            ${retryControls}
            ${recordFailure}
            ${synchronize}
            recordDraftSyncFailure("task", "Draft offline", true);
            recordDraftSyncFailure(
                "source", secondary.message, secondary.retryable,
            );
            const visibleBefore = !retrySession.hidden;
            await syncDraftField("task", { value: "recovered" }, "secret");
            return JSON.stringify({
                visibleBefore,
                visibleAfter: !retrySession.hidden,
                retryableFields: [...retryableDraftFields],
                failedFields: [...failedDraftFields],
                status: sessionStatus.textContent,
            });
        })()
        `, {
            secondary,
            fetch: async () => ({ status: 204 }),
        });
        assert.deepEqual(JSON.parse(result), {
            visibleBefore: true,
            visibleAfter: secondary.retryable,
            retryableFields: secondary.retryable ? ["source"] : [],
            failedFields: ["source"],
            status: secondary.message,
        });
    }
});

test("draft 413 media refusal precedes diagnostic body parsing", async () => {
    const source = await readFile(MAIN_MODULE, "utf8");
    const start = source.indexOf("async function syncDraftField(");
    const end = source.indexOf("function bindDraftSync(", start);
    assert.ok(start >= 0 && end > start, "draft sync must be extractable");
    const syncDraftField = source.slice(start, end);
    for (const [mediaType, admitted] of [
        ["text/html; charset=utf-8", false],
        ["text/plain; charset=utf-8", false],
        [null, false],
        ["application/json; charset=utf-8", true],
    ]) {
        let bodyReads = 0;
        const result = await runInNewContext(`
            (async () => {
                const sessionSecret = "secret";
                const draftSyncGeneration = 0;
                const sessionRequests = new AbortController();
                const failedDraftFields = new Set();
                const retryableDraftFields = new Set();
                const syncingDraftFields = new Set();
                const failures = [];
                const parseBoundedJsonResponse = (response) => response.json();
                const sessionDraft = {
                    draftMutationTarget: () => "./api/session/task",
                    draftMutationHeaders: () => ({}),
                    isResourceLimit: () => true,
                    isDraftDiagnosticContentType: (value) =>
                        value === "application/json; charset=utf-8",
                };
                function draftSyncIsCurrent(secret, generation) {
                    return sessionSecret === secret
                        && draftSyncGeneration === generation;
                }
                function recordDraftSyncFailure(field, message) {
                    failedDraftFields.add(field);
                    failures.push(message);
                }
                function hideResolvedDraftRetry() {}
                function hideSessionRetry() {}
                function setTextIfChanged() {}
                function invalidateUnauthorizedSession() {}
                function discardResponseBody() {}
                ${syncDraftField}
                await syncDraftField("task", { value: "private" }, "secret");
                return JSON.stringify(failures);
            })()
        `, {
            fetch: async () => ({
                status: 413,
                headers: { get: () => mediaType },
                json: async () => {
                    bodyReads += 1;
                    return { error: "resource_limit" };
                },
            }),
        });
        assert.deepEqual(JSON.parse(result), [admitted
            ? "Draft too large · reduce"
            : "Invalid draft diagnostic"]);
        assert.equal(bodyReads, admitted ? 1 : 0,
            `untrusted body must remain unread: ${mediaType}`);
    }
});

test("page session network requests are cancellable on exit", async () => {
    const source = await readFile(MAIN_MODULE, "utf8");
    const signalUses = source.match(
        /sessionRequests\.signal, requestAbort\.signal,/gu,
    ) ?? [];
    assert.equal(signalUses.length, 4,
        "handshake, draft reads, writes, and explicit End retain the "
            + "page-session abort alongside each per-request abort");
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
        const bodyRead = handshake.indexOf(
            "await parseBoundedJsonResponse(response);",
        );
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


test("transient handshake HTTP 408 offers retry without parsing", async () => {
    const source = await readFile(MAIN_MODULE, "utf8");
    const start = source.indexOf(
        "async function completeSessionHandshake(secret) {",
    );
    const end = source.indexOf("const failedDraftFields", start);
    assert.ok(start >= 0 && end > start);
    const handshake = source.slice(start, end);
    for (const [status, expectedRetry] of [
        [408, true],
        [500, true],
        [403, false],
    ]) {
        const result = await runInNewContext(`
            (async () => {
                const sessionSecret = "secret";
                const sessionRequests = new AbortController();
                const sessionStatus = { textContent: "" };
                const retries = [];
                const sessionHandshake = {
                    handshakeHeaders: () => ({}),
                };
                function hideSessionRetry() {}
                function showSessionRetry(reason) { retries.push(reason); }
                function setTextIfChanged(element, value) {
                    element.textContent = value;
                }
                function invalidateUnauthorizedSession() {
                    throw new Error("unexpected authorization loss");
                }
                async function hydrateSessionDraft() {
                    throw new Error("unexpected draft hydration");
                }
                function discardResponseBody() {}
                ${handshake}
                await completeSessionHandshake("secret");
                return JSON.stringify({
                    message: sessionStatus.textContent,
                    retries,
                });
            })()
        `, {
            fetch: async () => ({
                status,
                headers: { get: () => { throw new Error("body read"); } },
            }),
        });
        assert.deepEqual(JSON.parse(result), {
            message: "Invalid backend handshake · editing disabled",
            retries: expectedRetry ? ["connection"] : [],
        }, `HTTP ${status}`);
    }
});

test("handshake response media is checked before JSON admission", async () => {
    const source = await readFile(MAIN_MODULE, "utf8");
    const start = source.indexOf(
        "async function completeSessionHandshake(secret) {",
    );
    const end = source.indexOf("const failedDraftFields", start);
    assert.ok(start >= 0 && end > start, "handshake must be extractable");
    const handshake = source.slice(start, end);
    const compatible = {
        result: "compatible",
        versions: {
            capability: "atrament.capability/1",
            product: "0.1.0",
            profile: "atrament.profile/1",
            prompt: "atrament.prompt/1",
            protocol: "atrament.runtime/1",
            renderer: "atrament.renderer/1",
        },
    };
    for (const [status, mediaType, allow] of [
        [200, "text/html; charset=utf-8", false],
        [200, null, false],
        [409, "text/plain; charset=utf-8", false],
        [409, "application/json; charset=latin1", false],
        [200, "application/json; charset=utf-8", true],
    ]) {
        let bodyReads = 0;
        let hydrations = 0;
        const retries = [];
        const result = await runInNewContext(`
            (async () => {
                const sessionSecret = "secret";
                const sessionRequests = new AbortController();
                const sessionStatus = { textContent: "" };
                const parseBoundedJsonResponse = (response) => response.json();
                const sessionHandshake = {
                    handshakeHeaders: () => ({}),
                    parseHandshakePayload: () => ({ kind: "compatible" }),
                    isHandshakeContentType: (value) =>
                        value === "application/json; charset=utf-8",
                };
                function hideSessionRetry() {}
                function showSessionRetry(reason) { onRetry(reason); }
                function setTextIfChanged(element, value) {
                    element.textContent = value;
                }
                function invalidateUnauthorizedSession() {}
                async function hydrateSessionDraft() { hydrate(); }
                function discardResponseBody() {}
                ${handshake}
                await completeSessionHandshake("secret");
                return sessionStatus.textContent;
            })()
        `, {
            hydrate: () => { hydrations += 1; },
            onRetry: (reason) => retries.push(reason),
            fetch: async () => ({
                status,
                headers: { get: () => mediaType },
                json: async () => {
                    bodyReads += 1;
                    return compatible;
                },
            }),
        });
        assert.equal(hydrations, allow ? 1 : 0,
            `status=${status} media=${mediaType}`);
        assert.equal(bodyReads, allow ? 1 : 0,
            "misdeclared handshake bodies must not be parsed");
        assert.deepEqual(retries, allow ? [] : ["connection"],
            "invalid handshake must allow in-tab retry without admission");
        if (!allow) {
            assert.equal(result,
                "Invalid backend handshake · editing disabled");
        }
    }
});

test("typed incompatibility on HTTP 409 remains terminal", async () => {
    const source = await readFile(MAIN_MODULE, "utf8");
    const start = source.indexOf(
        "async function completeSessionHandshake(secret) {",
    );
    const end = source.indexOf("const failedDraftFields", start);
    assert.ok(start >= 0 && end > start);
    const handshake = source.slice(start, end);
    const incompatible = {
        result: "incompatible",
        diagnostics: {
            version: "atrament.diagnostic/1",
            completeness: "complete",
            items: [{
                code: "atrament.handshake.version-mismatch",
                dimension: "prompt",
                expected: "atrament.prompt/1",
            }],
        },
    };
    for (const [httpStatus, payload, expectedStatus, expectedRetries] of [
        [409, incompatible,
            "Incompatible prompt · expected atrament.prompt/1", []],
        [200, incompatible, "Invalid backend handshake · editing disabled",
            ["connection"]],
        [409, { result: "incompatible" },
            "Invalid backend handshake · editing disabled", ["connection"]],
    ]) {
        const retries = [];
        let hydrations = 0;
        const response = new Response(JSON.stringify(payload), {
            status: httpStatus,
            headers: { "Content-Type": "application/json; charset=utf-8" },
        });
        const status = await runInNewContext(`
            (async () => {
                const sessionSecret = "secret";
                const sessionRequests = new AbortController();
                const sessionStatus = { textContent: "" };
                const sessionHandshake = {
                    handshakeHeaders: () => ({}),
                    parseHandshakePayload: parseProvidedHandshake,
                    isHandshakeContentType: (value) =>
                        value === "application/json; charset=utf-8",
                };
                function hideSessionRetry() {}
                function showSessionRetry(reason) { onRetry(reason); }
                function setTextIfChanged(element, value) {
                    element.textContent = value;
                }
                function invalidateUnauthorizedSession() {}
                async function hydrateSessionDraft() { hydrate(); }
                function discardResponseBody() {}
                ${handshake}
                await completeSessionHandshake("secret");
                return sessionStatus.textContent;
            })()
        `, {
            fetch: async () => response,
            parseBoundedJsonResponse,
            parseProvidedHandshake: parseHandshakePayload,
            hydrate: () => { hydrations += 1; },
            onRetry: (reason) => retries.push(reason),
        });
        assert.equal(status, expectedStatus);
        assert.deepEqual(retries, expectedRetries);
        assert.equal(hydrations, 0,
            "version conflict must never authorize private draft reads");
    }
});

test("handshake overlong JSON cannot authorize draft hydration", async () => {
    const source = await readFile(MAIN_MODULE, "utf8");
    const start = source.indexOf(
        "async function completeSessionHandshake(secret) {",
    );
    const end = source.indexOf("const failedDraftFields", start);
    assert.ok(start >= 0 && end > start);
    const handshake = source.slice(start, end);
    const valid = JSON.stringify({
        result: "compatible",
        versions: {
            capability: "atrament.capability/1",
            product: "0.1.0",
            profile: "atrament.profile/1",
            prompt: "atrament.prompt/1",
            protocol: "atrament.runtime/1",
            renderer: "atrament.renderer/1",
        },
    });
    for (const [payload, allowed] of [
        [valid, true],
        [valid + " ".repeat(16_385), false],
        ["{invalid-json", false],
    ]) {
        let hydrations = 0;
        const retries = [];
        const response = new Response(payload, {
            status: 200,
            headers: { "Content-Type": "application/json; charset=utf-8" },
        });
        const status = await runInNewContext(`
            (async () => {
                const sessionSecret = "secret";
                const sessionRequests = new AbortController();
                const sessionStatus = { textContent: "" };
                const sessionHandshake = {
                    handshakeHeaders: () => ({}),
                    parseHandshakePayload: (value) =>
                        value?.result === "compatible"
                            ? { kind: "compatible" }
                            : { kind: "invalid" },
                    isHandshakeContentType: (value) =>
                        value === "application/json; charset=utf-8",
                };
                function hideSessionRetry() {}
                function showSessionRetry(reason) { onRetry(reason); }
                function setTextIfChanged(element, value) {
                    element.textContent = value;
                }
                function invalidateUnauthorizedSession() {}
                async function hydrateSessionDraft() { hydrate(); }
                function discardResponseBody() {}
                ${handshake}
                await completeSessionHandshake("secret");
                return sessionStatus.textContent;
            })()
        `, {
            fetch: async () => response,
            hydrate: () => { hydrations += 1; },
            onRetry: (reason) => retries.push(reason),
            parseBoundedJsonResponse,
        });
        assert.equal(hydrations, allowed ? 1 : 0);
        assert.deepEqual(retries, allowed ? [] : ["connection"],
            "bounded JSON failure must expose in-tab recovery");
        if (!allowed) {
            assert.equal(
                status, "Invalid backend handshake · editing disabled",
            );
        }
    }
});

test("draft 413 oversized JSON cannot fake a resource limit", async () => {
    const source = await readFile(MAIN_MODULE, "utf8");
    const start = source.indexOf("async function syncDraftField(");
    const end = source.indexOf("function bindDraftSync(", start);
    assert.ok(start >= 0 && end > start);
    const draftSync = source.slice(start, end);
    const valid = JSON.stringify({ error: "resource_limit" });
    for (const [payload, expected] of [
        [valid, "Draft too large · reduce"],
        [valid + " ".repeat(16_385), "Invalid draft diagnostic"],
    ]) {
        const response = new Response(payload, {
            status: 413,
            headers: { "Content-Type": "application/json; charset=utf-8" },
        });
        const result = await runInNewContext(`
            (async () => {
                const sessionSecret = "secret";
                const draftSyncGeneration = 0;
                const sessionRequests = new AbortController();
                const failedDraftFields = new Set();
                const retryableDraftFields = new Set();
                const syncingDraftFields = new Set();
                const failures = [];
                const sessionDraft = {
                    draftMutationTarget: () => "./api/session/task",
                    draftMutationHeaders: () => ({}),
                    isResourceLimit: (value) =>
                        value?.error === "resource_limit",
                    isDraftDiagnosticContentType: (value) =>
                        value === "application/json; charset=utf-8",
                };
                function draftSyncIsCurrent(secret, generation) {
                    return sessionSecret === secret
                        && draftSyncGeneration === generation;
                }
                function recordDraftSyncFailure(field, message) {
                    failedDraftFields.add(field);
                    failures.push(message);
                }
                function hideResolvedDraftRetry() {}
                function hideSessionRetry() {}
                function setTextIfChanged() {}
                function invalidateUnauthorizedSession() {}
                function discardResponseBody() {}
                ${draftSync}
                await syncDraftField("task", { value: "private" }, "secret");
                return JSON.stringify(failures);
            })()
        `, { fetch: async () => response, parseBoundedJsonResponse });
        assert.deepEqual(JSON.parse(result), [expected]);
    }
});

test("draft hydration rejects wrong media without body read", async () => {
    const source = await readFile(MAIN_MODULE, "utf8");
    const start = source.indexOf("async function readDraftField(");
    const end = source.indexOf("async function hydrateSessionDraft(", start);
    assert.ok(start >= 0 && end > start, "draft read must be extractable");
    const readDraftField = source.slice(start, end);
    for (const mediaType of [
        "text/html; charset=utf-8",
        "application/json; charset=utf-8",
        "text/plain; charset=iso-8859-1",
        null,
        "text/plain; charset=utf-8",
    ]) {
        let reads = 0;
        const result = await runInNewContext(`
            (async () => {
                const sessionSecret = "secret";
                const draftSyncGeneration = 0;
                const sessionRequests = new AbortController();
                function draftSyncIsCurrent(secret, generation) {
                    return sessionSecret === secret
                        && draftSyncGeneration === generation;
                }
                const sessionDraft = {
                    draftReadTarget: () => "./api/session/task",
                    draftReadHeaders: () => ({}),
                    isDraftReadContentType,
                    decodeDraftReadResponse,
                };
                function discardResponseBody() {}
                ${readDraftField}
                return readDraftField("task", "secret", 0);
            })()
        `, {
            isDraftReadContentType,
            decodeDraftReadResponse,
            fetch: async () => ({
                status: 200,
                headers: {
                    get: (name) => name === "content-type" ? mediaType : null,
                },
                arrayBuffer: async () => {
                    reads += 1;
                    return new TextEncoder().encode("private draft").buffer;
                },
            }),
        });
        if (mediaType === "text/plain; charset=utf-8") {
            assert.equal(result.kind, "available");
            assert.equal(result.value, "private draft");
            assert.equal(reads, 1);
        } else {
            assert.equal(result.kind, "rejected", String(mediaType));
            assert.equal(reads, 0, "untrusted body must remain unread");
        }
    }
});

test(
    "draft hydration refuses a response beyond backend field limit",
    async () => {
    const source = await readFile(MAIN_MODULE, "utf8");
    const start = source.indexOf("async function readDraftField(");
    const end = source.indexOf("async function hydrateSessionDraft(", start);
    assert.ok(start >= 0 && end > start);
    const readDraftField = source.slice(start, end);
    const oversized = new TextEncoder().encode("x".repeat(1_048_577));
    const result = await runInNewContext(`
        (async () => {
            const sessionSecret = "secret";
            const draftSyncGeneration = 0;
            const sessionRequests = new AbortController();
            function draftSyncIsCurrent(secret, generation) {
                return sessionSecret === secret
                    && draftSyncGeneration === generation;
            }
            const sessionDraft = {
                draftReadTarget: () => "./api/session/task",
                draftReadHeaders: () => ({}),
                isDraftReadContentType,
                decodeDraftReadResponse,
            };
            function discardResponseBody() {}
            ${readDraftField}
            return readDraftField("task", "secret", 0);
        })()
    `, {
        isDraftReadContentType,
        decodeDraftReadResponse,
        fetch: async () => ({
            status: 200,
            headers: {
                get: (name) => name === "content-type"
                    ? "text/plain; charset=utf-8" : null,
            },
            arrayBuffer: async () => oversized.buffer,
        }),
    });
    assert.equal(result.kind, "rejected");
    },
);

test("draft hydration preserves BOM and rejects invalid UTF-8", async () => {
    const source = await readFile(MAIN_MODULE, "utf8");
    const start = source.indexOf("async function readDraftField(");
    const end = source.indexOf("async function hydrateSessionDraft(", start);
    assert.ok(start >= 0 && end > start, "draft read must be extractable");
    const readDraftField = source.slice(start, end);
    for (const [body, expected] of [
        [Uint8Array.of(0xef, 0xbb, 0xbf, 0x54), {
            kind: "available", value: "\uFEFFT",
        }],
        [Uint8Array.of(0xc3, 0x28), { kind: "rejected" }],
    ]) {
        const result = await runInNewContext(`
            (async () => {
                const sessionSecret = "secret";
                const draftSyncGeneration = 0;
                const sessionRequests = new AbortController();
                function draftSyncIsCurrent(secret, generation) {
                    return sessionSecret === secret
                        && draftSyncGeneration === generation;
                }
                const sessionDraft = {
                    draftReadTarget: () => "./api/session/task",
                    draftReadHeaders: () => ({}),
                    isDraftReadContentType,
                    decodeDraftReadResponse,
                };
                function discardResponseBody() {}
                ${readDraftField}
                return readDraftField("task", "secret", 0);
            })()
        `, {
            isDraftReadContentType,
            decodeDraftReadResponse,
            fetch: async () => ({
                status: 200,
                headers: {
                    get: (name) => name === "content-type"
                        ? "text/plain; charset=utf-8" : null,
                },
                text: () => new Response(body).text(),
                arrayBuffer: () => new Response(body).arrayBuffer(),
            }),
        });
        assert.deepEqual(JSON.parse(JSON.stringify(result)), expected);
    }
});

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


test(
    "fragment-free refresh cannot silently resume the backend session",
    async () => {
    const source = await readFile(MAIN_MODULE, "utf8");
    const startup = source.slice(source.lastIndexOf("setPreviewZoom(100);"));
    const missing = startup.indexOf("if (sessionSecret === null)");
    const unavailable = startup.indexOf(
        '"Frontend ready · credential unavailable"',
        missing,
    );
    const compatible = startup.indexOf(
        "void completeSessionHandshake(sessionSecret);",
        unavailable,
    );
    assert.ok(
        missing !== -1
            && unavailable > missing
            && compatible > unavailable,
        "fragment-free startup must remain disabled instead of rejoining",
    );
        assert.equal(
            startup.indexOf("completeSessionHandshake(sessionSecret)", 0),
            compatible + "void ".length,
            "startup must have no alternate handshake path without "
                + "a credential",
        );
    },
);


test("fragment cleanup wipes pre-existing history state", async () => {
    const source = await readFile(MAIN_MODULE, "utf8");
    const helperStart = source.indexOf("function fragmentFreeLocalUrl() {");
    const handoffStart = source.indexOf(
        "function consumeSessionSecretFragment() {",
    );
    const handoffEnd = source.indexOf(
        "\nlet sessionSecret = consumeSessionSecretFragment();",
        handoffStart,
    );
    const localStart = source.indexOf(
        "function discardLocalNavigationFragment() {",
    );
    const localEnd = source.indexOf(
        '\nwindow.addEventListener("pageshow"', localStart,
    );
    assert.ok(
        helperStart >= 0 && handoffStart > helperStart
            && handoffEnd > handoffStart
            && localStart > handoffEnd && localEnd > localStart,
    );
    const helper = source.slice(helperStart, handoffStart);
    const handoff = source.slice(handoffStart, handoffEnd);
    const localNavigation = source.slice(localStart, localEnd);
    const secret = "a".repeat(64);
    const replaced = [];
    const fakeWindow = {
        location: {
            hash: `#session=${secret}`,
            pathname: "/index.html",
            search: "?view=1",
            replace() { throw Error("unexpected navigation fallback"); },
        },
        history: {
            state: { private: "never-persist-this" },
            replaceState(state, title, url) {
                replaced.push({ state, title, url });
            },
        },
    };
    const accepted = runInNewContext(
        `${helper}${handoff}\nconsumeSessionSecretFragment();`,
        {
            window: fakeWindow,
            sessionSecretFromFragment: () => secret,
        },
    );
    assert.equal(accepted, secret);
    assert.deepEqual(replaced, [{
        state: null,
        title: "",
        url: "/index.html?view=1",
    }]);

    replaced.length = 0;
    Object.defineProperty(fakeWindow.history, "state", {
        get() { throw Error("history state must not be read"); },
    });
    const noHistoryRead = runInNewContext(
        `${helper}${handoff}\nconsumeSessionSecretFragment();`,
        {
            window: fakeWindow,
            sessionSecretFromFragment: () => secret,
        },
    );
    assert.equal(noHistoryRead, secret);
    assert.equal(replaced[0]?.state, null);

    replaced.length = 0;
    fakeWindow.location.hash = "#source-panel";
    const discarded = runInNewContext(
        `${helper}${localNavigation}\ndiscardLocalNavigationFragment();`,
        {
            window: fakeWindow,
            document: { getElementById: () => ({}) },
        },
    );
    assert.equal(discarded, true);
    assert.deepEqual(replaced, [{
        state: null,
        title: "",
        url: "/index.html?view=1",
    }]);
});

test("encoded credential aliases scrub without login", async () => {
    const source = await readFile(MAIN_MODULE, "utf8");
    const start = source.indexOf("function fragmentFreeLocalUrl() {");
    const end = source.indexOf(
        "\nlet sessionSecret = consumeSessionSecretFragment();", start,
    );
    assert.ok(start >= 0 && end > start);
    const handoff = source.slice(start, end);
    const secret = "a".repeat(64);
    const { sessionSecretFromFragment } = await import(new URL(
        "../../../../src/browser/workspace/adapter-inbound/generated/"
            + "session-fragment.js",
        import.meta.url,
    ));
    const aliases = [
        `#session%3D${secret}`,
        `#session%3d${secret}`,
        `#%73ession=${secret}`,
        `#sess%69on=${secret}`,
        `#%73ession%3D${secret}%zz`,
        `#SESSION=${secret}`,
    ];
    for (const hash of aliases) {
        assert.equal(sessionSecretFromFragment(hash), null);
        const replaced = [];
        const location = {
            hash, pathname: "/index.html", search: "?view=1",
            replace() { throw Error("unexpected fallback navigation"); },
        };
        const window = {
            location,
            history: {
                replaceState(state, title, url) {
                    replaced.push({ state, title, url });
                },
            },
        };
        const admitted = runInNewContext(
            `${handoff}\nconsumeSessionSecretFragment();`,
            { window, sessionSecretFromFragment },
        );
        assert.equal(admitted, null, `noncanonical alias ${hash}`);
        assert.deepEqual(replaced, [{
            state: null, title: "", url: "/index.html?view=1",
        }], `credential-like fragment must be scrubbed: ${hash}`);
    }

    for (const hash of ["#source-panel", "#other%3Dvalue", "#invalid%zz"]) {
        const replaced = [];
        const window = {
            location: {
                hash, pathname: "/index.html", search: "",
                replace() { throw Error("unexpected navigation"); },
            },
            history: { replaceState(...args) { replaced.push(args); } },
        };
        const admitted = runInNewContext(
            `${handoff}\nconsumeSessionSecretFragment();`,
            { window, sessionSecretFromFragment },
        );
        assert.equal(admitted, null);
        assert.deepEqual(replaced, [], `unrelated fragment ${hash}`);
    }
});

test("post-load credential hashes are scrubbed without authority", async () => {
    const source = await readFile(MAIN_MODULE, "utf8");
    const start = source.indexOf("function fragmentFreeLocalUrl() {");
    const end = source.indexOf(
        "const sessionRequests = new AbortController();", start,
    );
    assert.ok(start >= 0 && end > start);
    const handoff = source.slice(start, end);
    const { sessionSecretFromFragment } = await import(new URL(
        "../../../../src/browser/workspace/adapter-inbound/generated/"
            + "session-fragment.js",
        import.meta.url,
    ));
    const secret = "a".repeat(64);
    const replaced = [];
    const listeners = new Map();
    const location = {
        hash: "", pathname: "/index.html", search: "?view=1",
        replace() { throw Error("unexpected redirect"); },
    };
    const window = {
        location,
        history: {
            replaceState(state, title, url) {
                replaced.push({ state, title, url });
                location.hash = "";
            },
        },
        addEventListener(name, callback) { listeners.set(name, callback); },
    };
    const state = runInNewContext(
        `${handoff}\n({ sessionSecret: () => sessionSecret })`,
        { window, sessionSecretFromFragment },
    );
    assert.equal(state.sessionSecret(), null);
    const navigate = listeners.get("hashchange");
    assert.equal(typeof navigate, "function",
        "post-startup credential navigation must have a scrub handler");
    for (const hash of [
        `#session=${secret}`,
        `#session%3D${secret}`,
        `#%73ession=${secret}`,
        `#SESSION=${secret}`,
    ]) {
        location.hash = hash;
        navigate();
        assert.equal(location.hash, "", `fragment retained: ${hash}`);
        assert.equal(state.sessionSecret(), null,
            "post-load credential fragments must never grant authority");
    }
    assert.deepEqual(replaced, Array.from({ length: 4 }, () => ({
        state: null, title: "", url: "/index.html?view=1",
    })));
    location.hash = "#source-panel";
    navigate();
    assert.equal(location.hash, "#source-panel",
        "ordinary editor navigation must remain intact");
    assert.equal(replaced.length, 4);
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

    const handoff = source.slice(urlStart, handoffEnd);
    for (const required of [
        'const hash = window.location.hash;',
        'hash.startsWith("#session=")',
        "const localUrl = fragmentFreeLocalUrl();",
        'window.history.replaceState(null, "", localUrl);',
        "window.location.replace(localUrl);",
        "return scrubSessionLikeFragment(hash) ? sessionSecret : null;",
    ]) {
        assert.equal(
            handoff.includes(required),
            true,
            `credential handoff must retain ${required}`,
        );
    }
});

function rejectedResponseWithPendingBody(status, contentType) {
    let cancellationCount = 0;
    const body = new ReadableStream({
        cancel() { cancellationCount += 1; },
    });
    const headers = contentType === null ? {} : {
        "Content-Type": contentType,
    };
    const response = new Response(body, { status, headers });
    return { response, cancellationCount: () => cancellationCount };
}

function discardedResponseHelper(source) {
    const start = source.indexOf("function discardResponseBody(");
    const end = source.indexOf("\n}", start);
    assert.ok(start >= 0 && end > start,
        "production rejected-response disposal helper must exist");
    return source.slice(start, end + 2);
}

function recordingAbortController(abortCalls) {
    return class extends AbortController {
        abort() {
            abortCalls.push(true);
            super.abort();
        }
    };
}

test("explicit session end joins page-lifetime cancellation", async () => {
    const source = await readFile(MAIN_MODULE, "utf8");
    const start = source.indexOf('fetch("./api/session/end"');
    const end = source.indexOf('window.addEventListener("pagehide"', start);
    assert.ok(start >= 0 && end > start);
    const request = source.slice(start, end);
    const signalStart = request.indexOf("signal: AbortSignal.any([");
    const signalEnd = request.indexOf("]),", signalStart);
    assert.ok(signalStart >= 0 && signalEnd > signalStart);
    const signals = request.slice(signalStart, signalEnd);
    for (const required of [
        "sessionRequests.signal",
        "requestAbort.signal",
        "requestDeadline",
    ]) {
        assert.ok(signals.includes(required),
            `explicit End must retain ${required}`);
    }
});

test("every browser session fetch has a finite request deadline", async () => {
    const source = await readFile(MAIN_MODULE, "utf8");
    assert.equal(source.split("AbortSignal.timeout(").length - 1, 4,
        "handshake, draft GET/POST, and explicit end must all time out");
});

function stalledTypedResponse(
    contentType, prefix, deadlines, aborts, status = 200,
) {
    const timeoutSignal = {
        any: (signals) => AbortSignal.any(signals),
        timeout(milliseconds) {
            deadlines.push(milliseconds);
            const controller = new AbortController();
            queueMicrotask(() => controller.abort());
            return controller.signal;
        },
    };
    const fetch = async (_url, options) => {
        const body = new ReadableStream({
            start(controller) {
                controller.enqueue(new TextEncoder().encode(prefix));
                options.signal.addEventListener("abort", () => {
                    aborts.push(true);
                    controller.error(new Error("synthetic socket timeout"));
                }, { once: true });
            },
        });
        return new Response(body, {
            status,
            headers: { "Content-Type": contentType },
        });
    };
    return { timeoutSignal, fetch };
}

test("stalled valid-media handshake rejects partial JSON and offers retry",
    { timeout: 2_000 }, async () => {
    const source = await readFile(MAIN_MODULE, "utf8");
    const handshake = source.slice(
        source.indexOf("async function completeSessionHandshake(secret) {"),
        source.indexOf("const failedDraftFields"),
    );
    const deadlines = [];
    const aborts = [];
    const retries = [];
    const fixture = stalledTypedResponse(
        "application/json; charset=utf-8", '{"result":', deadlines, aborts,
    );
    const result = await runInNewContext(`
        (async () => {
            const sessionSecret = "secret";
            const sessionRequests = new AbortController();
            const sessionStatus = { textContent: "" };
            const sessionHandshake = {
                handshakeHeaders: () => ({}),
                isHandshakeContentType: (v) =>
                    v === "application/json; charset=utf-8",
                parseHandshakePayload: () => ({ kind: "invalid" }),
            };
            function hideSessionRetry() {}
            function showSessionRetry(purpose) { onRetry(purpose); }
            function setTextIfChanged(element, value) {
                element.textContent = value;
            }
            function invalidateUnauthorizedSession() {}
            async function hydrateSessionDraft() {
                throw Error("incomplete JSON must not hydrate");
            }
            const parseBoundedJsonResponse = parseJson;
            ${discardedResponseHelper(source)}
            ${handshake}
            await completeSessionHandshake("secret");
            return sessionStatus.textContent;
        })()
    `, {
        fetch: fixture.fetch,
        AbortSignal: fixture.timeoutSignal,
        parseJson: parseBoundedJsonResponse,
        onRetry: (purpose) => retries.push(purpose),
    });
    assert.equal(result, "Invalid backend handshake · editing disabled");
    assert.deepEqual(retries, ["connection"]);
    assert.deepEqual(aborts, [true]);
    assert.equal(deadlines.length, 1);
    assert.ok(deadlines[0] >= 2_000 && deadlines[0] <= 20_000);
});

test("stalled valid-media draft read never hydrates partial text",
    { timeout: 2_000 }, async () => {
    const source = await readFile(MAIN_MODULE, "utf8");
    const readDraft = source.slice(
        source.indexOf("async function readDraftField("),
        source.indexOf("async function hydrateSessionDraft("),
    );
    const deadlines = [];
    const aborts = [];
    const fixture = stalledTypedResponse(
        "text/plain; charset=utf-8", "private partial", deadlines, aborts,
    );
    const result = await runInNewContext(`
        (async () => {
            const sessionSecret = "secret";
            const draftSyncGeneration = 0;
            const sessionRequests = new AbortController();
            const sessionDraft = {
                draftReadTarget: () => "./api/session/task",
                draftReadHeaders: () => ({}),
                isDraftReadContentType: (v) =>
                    v === "text/plain; charset=utf-8",
                decodeDraftReadResponse: decodeDraft,
            };
            function draftSyncIsCurrent(secret, generation) {
                return secret === sessionSecret
                    && generation === draftSyncGeneration;
            }
            ${discardedResponseHelper(source)}
            ${readDraft}
            return readDraftField("task", "secret", 0);
        })()
    `, {
        fetch: fixture.fetch,
        AbortSignal: fixture.timeoutSignal,
        decodeDraft: decodeDraftReadResponse,
    });
    assert.equal(result.kind, "unavailable");
    assert.deepEqual(aborts, [true]);
    assert.equal(deadlines.length, 1);
    assert.ok(deadlines[0] >= 2_000 && deadlines[0] <= 20_000);
});

test("stalled draft limit diagnostic retains an explicit edit retry",
    { timeout: 2_000 }, async () => {
    const source = await readFile(MAIN_MODULE, "utf8");
    const syncDraft = source.slice(
        source.indexOf("async function syncDraftField("),
        source.indexOf("function bindDraftSync("),
    );
    const deadlines = [];
    const aborts = [];
    const failures = [];
    const fixture = stalledTypedResponse(
        "application/json; charset=utf-8", '{"error":',
        deadlines, aborts, 413,
    );
    await runInNewContext(`
        (async () => {
            const sessionSecret = "secret";
            const draftSyncGeneration = 0;
            const sessionRequests = new AbortController();
            const failedDraftFields = new Set();
            const retryableDraftFields = new Set();
            const syncingDraftFields = new Set();
            const sessionDraft = {
                draftMutationTarget: () => "./api/session/task",
                draftMutationHeaders: () => ({}),
                isDraftDiagnosticContentType: (v) =>
                    v === "application/json; charset=utf-8",
            };
            function draftSyncIsCurrent(secret, generation) {
                return secret === sessionSecret
                    && generation === draftSyncGeneration;
            }
            function recordDraftSyncFailure(field, message, retryable) {
                failedDraftFields.add(field);
                onFailure(message, retryable);
            }
            function hideResolvedDraftRetry() {}
            function hideSessionRetry() {}
            function setTextIfChanged() {}
            function invalidateUnauthorizedSession() {}
            const parseBoundedJsonResponse = parseJson;
            ${discardedResponseHelper(source)}
            ${syncDraft}
            await syncDraftField("task", { value: "private" }, "secret");
        })()
    `, {
        fetch: fixture.fetch,
        AbortSignal: fixture.timeoutSignal,
        parseJson: parseBoundedJsonResponse,
        onFailure: (...args) => failures.push(args),
    });
    assert.deepEqual(failures, [["Draft offline · retry edit", true]]);
    assert.deepEqual(aborts, [true]);
    assert.equal(deadlines.length, 1);
});

test(
    "rejected handshake responses cancel unread bodies before retry",
    async () => {
    const source = await readFile(MAIN_MODULE, "utf8");
    const discard = discardedResponseHelper(source);
    const start = source.indexOf(
        "async function completeSessionHandshake(secret) {",
    );
    const end = source.indexOf("const failedDraftFields", start);
    assert.ok(start >= 0 && end > start);
    const handshake = source.slice(start, end);
    for (const [status, contentType, expectRetry] of [
        [200, "text/html; charset=utf-8", true],
        [409, "text/html; charset=utf-8", true],
        [408, null, true],
        [500, null, true],
        [403, null, false],
    ]) {
        const probe = rejectedResponseWithPendingBody(status, contentType);
        const abortCalls = [];
        const retries = [];
        const result = await runInNewContext(`
            (async () => {
                const sessionSecret = "secret";
                const sessionRequests = new AbortController();
                const sessionStatus = { textContent: "" };
                const sessionHandshake = {
                    handshakeHeaders: () => ({}),
                    isHandshakeContentType: (media) =>
                        media === "application/json; charset=utf-8",
                };
                function hideSessionRetry() {}
                function showSessionRetry(purpose) { onRetry(purpose); }
                function setTextIfChanged(element, value) {
                    element.textContent = value;
                }
                function invalidateUnauthorizedSession() {}
                async function hydrateSessionDraft() {
                    throw Error("rejected handshake cannot hydrate");
                }
                const parseBoundedJsonResponse = () => {
                    throw Error("rejected body cannot be parsed");
                };
                ${discard}
                ${handshake}
                await completeSessionHandshake("secret");
                return sessionStatus.textContent;
            })()
        `, {
            fetch: async () => probe.response,
            onRetry: (purpose) => retries.push(purpose),
            AbortController: recordingAbortController(abortCalls),
        });
        assert.equal(result, "Invalid backend handshake · editing disabled");
        assert.deepEqual(retries, expectRetry ? ["connection"] : []);
        assert.equal(probe.cancellationCount(), 1,
            `HTTP ${status} ${contentType} body must be canceled`);
        assert.equal(abortCalls.length, 1,
            `HTTP ${status} ${contentType} fetch must be aborted`);
    }
});

test("rejected draft reads cancel unread bodies", async () => {
    const source = await readFile(MAIN_MODULE, "utf8");
    const discard = discardedResponseHelper(source);
    const start = source.indexOf("async function readDraftField(");
    const end = source.indexOf("async function hydrateSessionDraft(", start);
    assert.ok(start >= 0 && end > start);
    const readDraftField = source.slice(start, end);
    for (const [status, contentType] of [
        [200, "text/html; charset=utf-8"],
        [403, "text/plain; charset=utf-8"],
        [408, "text/plain; charset=utf-8"],
    ]) {
        const probe = rejectedResponseWithPendingBody(status, contentType);
        const abortCalls = [];
        const outcome = await runInNewContext(`
            (async () => {
                const sessionSecret = "secret";
                const draftSyncGeneration = 0;
                const sessionRequests = new AbortController();
                function draftSyncIsCurrent(secret, generation) {
                    return sessionSecret === secret
                        && draftSyncGeneration === generation;
                }
                const sessionDraft = {
                    draftReadTarget: () => "./api/session/task",
                    draftReadHeaders: () => ({}),
                    isDraftReadContentType: (value) =>
                        value === "text/plain; charset=utf-8",
                };
                ${discard}
                ${readDraftField}
                return readDraftField("task", "secret", 0);
            })()
        `, {
            fetch: async () => probe.response,
            AbortController: recordingAbortController(abortCalls),
        });
        assert.equal(outcome.kind, "rejected");
        assert.equal(probe.cancellationCount(), 1,
            `HTTP ${status} ${contentType} body must be canceled`);
        assert.equal(abortCalls.length, 1,
            `HTTP ${status} ${contentType} fetch must be aborted`);
    }
});

test(
    "rejected draft writes cancel unread diagnostic and error bodies",
    async () => {
    const source = await readFile(MAIN_MODULE, "utf8");
    const discard = discardedResponseHelper(source);
    const start = source.indexOf("async function syncDraftField(");
    const end = source.indexOf("function bindDraftSync(", start);
    assert.ok(start >= 0 && end > start);
    const syncDraftField = source.slice(start, end);
    for (const [status, contentType] of [
        [413, "text/html; charset=utf-8"],
        [403, "application/json; charset=utf-8"],
        [408, "application/json; charset=utf-8"],
        [500, null],
    ]) {
        const probe = rejectedResponseWithPendingBody(status, contentType);
        const abortCalls = [];
        const failures = [];
        await runInNewContext(`
            (async () => {
                const sessionSecret = "secret";
                const draftSyncGeneration = 0;
                const sessionRequests = new AbortController();
                const failedDraftFields = new Set();
                const retryableDraftFields = new Set();
                const syncingDraftFields = new Set();
                const sessionDraft = {
                    draftMutationTarget: () => "./api/session/task",
                    draftMutationHeaders: () => ({}),
                    isDraftDiagnosticContentType: (value) =>
                        value === "application/json; charset=utf-8",
                };
                function draftSyncIsCurrent(secret, generation) {
                    return sessionSecret === secret
                        && draftSyncGeneration === generation;
                }
                function recordDraftSyncFailure(field, message, retryable) {
                    failedDraftFields.add(field);
                    onFailure(message, retryable);
                }
                function hideResolvedDraftRetry() {}
                function hideSessionRetry() {}
                function setTextIfChanged() {}
                function invalidateUnauthorizedSession() {}
                const parseBoundedJsonResponse = () => {
                    throw Error("rejected diagnostic body cannot be parsed");
                };
                ${discard}
                ${syncDraftField}
                await syncDraftField("task", { value: "private" }, "secret");
            })()
        `, {
            fetch: async () => probe.response,
            onFailure: (...args) => failures.push(args),
            AbortController: recordingAbortController(abortCalls),
        });
        assert.equal(failures.length, 1,
            `HTTP ${status} must leave a visible draft failure`);
        assert.equal(probe.cancellationCount(), 1,
            `HTTP ${status} ${contentType} body must be canceled`);
        assert.equal(abortCalls.length, 1,
            `HTTP ${status} ${contentType} fetch must be aborted`);
    }
});
