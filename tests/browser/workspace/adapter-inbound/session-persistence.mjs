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
import { runInNewContext } from "node:vm";

const INDEX_HTML = new URL(
    "../../../../src/browser/workspace/adapter-inbound/index.html",
    import.meta.url,
);
const MAIN_MODULE = new URL(
    "../../../../src/browser/workspace/adapter-inbound/generated/main.js",
    import.meta.url,
);
const {
    isDraftReadContentType,
    decodeDraftReadResponse,
} = await import(
    new URL("session-draft.js", MAIN_MODULE).href,
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

test(
    "workspace module has no undeclared outbound browser transport",
    async () => {
    const source = await readFile(MAIN_MODULE, "utf8");
    const fetchCalls = source.match(/\bfetch\(/gu) ?? [];
    assert.equal(
        fetchCalls.length,
        4,
        "only handshake, draft, and session end fetches exist",
    );

    for (const required of [
        'cache: "no-store"',
        'credentials: "omit"',
        'mode: "same-origin"',
        'redirect: "error"',
    ]) {
        const occurrences = source.split(required).length - 1;
        assert.equal(
            occurrences,
            4,
            `all admitted fetches must retain ${required}`,
        );
    }

    assert.ok(source.includes('fetch("./api/session/end"'),
        "session end remains same-origin and explicit");
    assert.equal(source.includes('keepalive: true'), false,
        "session end must require live document authority");

    assert.equal(
        source.split('referrerPolicy: "origin"').length - 1, 3,
        "all credentialed POST requests disclose only the loopback origin",
    );
    assert.equal(
        source.split('referrerPolicy: "no-referrer"').length - 1, 1,
        "read-only hydration never sends a referrer",
    );

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
            const retrySession = { hidden: true, disabled: true };
            const sessionStatus = { textContent: "" };
            const sessionSecret = "secret";
            const draftSyncGeneration = 0;
            const failedDraftFields = new Set();
            const retryableDraftFields = new Set();
            const syncingDraftFields = new Set();
            const sessionRequests = { signal: {} };
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
                const sessionRequests = { signal: {} };
                const failedDraftFields = new Set();
                const retryableDraftFields = new Set();
                const syncingDraftFields = new Set();
                const failures = [];
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
        const result = await runInNewContext(`
            (async () => {
                const sessionSecret = "secret";
                const sessionRequests = { signal: {} };
                const sessionStatus = { textContent: "" };
                const sessionHandshake = {
                    handshakeHeaders: () => ({}),
                    parseHandshakePayload: () => ({ kind: "compatible" }),
                    isHandshakeContentType: (value) =>
                        value === "application/json; charset=utf-8",
                };
                function hideSessionRetry() {}
                function showSessionRetry() {}
                function setTextIfChanged(element, value) {
                    element.textContent = value;
                }
                function invalidateUnauthorizedSession() {}
                async function hydrateSessionDraft() { hydrate(); }
                ${handshake}
                await completeSessionHandshake("secret");
                return sessionStatus.textContent;
            })()
        `, {
            hydrate: () => { hydrations += 1; },
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
        if (!allow) {
            assert.equal(result,
                "Invalid backend handshake · editing disabled");
        }
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
                const sessionRequests = { signal: {} };
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
            const sessionRequests = { signal: {} };
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
                const sessionRequests = { signal: {} };
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
