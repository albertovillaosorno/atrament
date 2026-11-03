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
    "paper-profile.js",
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

// The HTML shell and stylesheet execute before the reviewed workspace
// module. Pin their literal request-capable declarations independently of
// the browser's CSP enforcement: a future asset needs explicit review.
const STYLESHEET = new URL(
    "../../../../src/browser/workspace/adapter-inbound/workspace.css",
    import.meta.url,
);

function usesUnreviewedMarkupRequests(source) {
    let remaining = source;
    for (const tag of [
        '<link rel="icon" href="data:,">',
        '<link rel="stylesheet" href="./workspace.css">',
        '<a class="skip-link" href="#llm-editor-title">',
        '<a class="skip-link" href="#human-editor-title">',
        '<script type="module" src="./generated/main.js">',
    ]) {
        if (remaining.split(tag).length - 1 !== 1) {
            return true;
        }
        remaining = remaining.replace(tag, "");
    }
    // This one explicitly selected paper JSON input is bounded by Rust;
    // any additional picker remains an unreviewed request capability.
    const paperPicker = `<input id="paper-load" type="file" accept=".json,application/json"
                aria-label="Load paper-only profile JSON" disabled>`;
    if (remaining.split(paperPicker).length - 1 !== 1) {
        return true;
    }
    remaining = remaining.replace(paperPicker, "");
    return [
        /<\s*(?:a|link|script|base|form|iframe|frame|portal|img)\b/iu,
        /<\s*(?:source|picture|video|audio|track|embed|object)\b/iu,
        /<\s*input\b[^>]*\btype\s*=\s*(?:["']file["']|file\b)/iu,
        /<\s*input\b[^>]*\b(?:webkitdirectory|directory|capture)\b/iu,
        /\b(?:href|src|srcset|srcdoc|action|formaction|poster)\s*=/iu,
        /\b(?:ping|background|manifest)\s*=/iu,
        /\b(?:style|on[a-z]+)\s*=/iu,
        /<\s*meta\b[^>]*\bhttp-equiv\s*=\s*['"]?refresh\b/iu,
    ].some((pattern) => pattern.test(remaining));
}

function usesUnreviewedStylesheetRequests(source) {
    return [
        /@\s*(?:import|font-face)\b/iu,
        /\b(?:url|image-set|cross-fade)\s*\(/iu,
        /-moz-binding\s*:/iu,
    ].some((pattern) => pattern.test(source));
}

test("static HTML and CSS admit only reviewed resource declarations",
    async () => {
    const html = await readFile(INDEX_HTML, "utf8");
    const css = await readFile(STYLESHEET, "utf8");
    assert.equal(usesUnreviewedMarkupRequests(html), false,
        "workspace HTML gained a resource or navigation declaration");
    assert.equal(usesUnreviewedStylesheetRequests(css), false,
        "workspace CSS gained a resource request declaration");
});

test("static asset inventory rejects unreviewed markup and CSS requests",
    async () => {
    const html = await readFile(INDEX_HTML, "utf8");
    const css = await readFile(STYLESHEET, "utf8");
    for (const candidate of [
        '<img src="https://example.invalid/pixel" alt="">',
        '<a href="https://example.invalid/leave">Leave</a>',
        '<link rel="prefetch" href="/extra">',
        '<script type="module" src="/extra.js"></script>',
        '<form action="/submit"></form>',
        '<iframe srcdoc="<p>foreign</p>"></iframe>',
        '<meta http-equiv="refresh" content="0;url=/leave">',
        '<div style="background: url(/image)"></div>',
        '<div onclick="location.assign(42)"></div>',
        '<svg><image href="/extra.svg" /></svg>',
        '<source srcset="/extra.webp 1x">',
        '<input type="file" id="unreviewed">',
        "<input type='FILE' accept='image/png'>",
        '<input type=file multiple>',
        '<input webkitdirectory>',
    ]) {
        assert.equal(usesUnreviewedMarkupRequests(
            html.replace("</body>", `${candidate}</body>`)), true,
        candidate);
    }
    for (const candidate of [
        '@import "https://example.invalid/extra.css";',
        'div { background-image: url("/extra.svg"); }',
        'div { background: image-set("/extra.png" 1x); }',
        '@font-face { font-family: extra; src: local("Extra"); }',
    ]) {
        assert.equal(usesUnreviewedStylesheetRequests(
            `${css}\n${candidate}`), true, candidate);
    }
});

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

// Count all literal fetch references, not just direct fetch(...) calls.
// Captured or bracket-indexed fetch values can be invoked later under an
// unrelated identifier. Unlike fetch(), fetchLater() can dispatch after the
// page is disposed, bypassing page-lifetime cancellation entirely.
// Full-line comments are not executable authority.
function hasUnreviewedFetchReference(source, main, paper = false) {
    const executable = source.replace(/^\s*\/\/[^\n]*$/gmu, "");
    const references = executable.match(/\bfetch\b/gu) ?? [];
    return references.length !== (main ? 4 : paper ? 1 : 0)
        || /\bfetchLater\b/u.test(executable);
}

test("browser transport review rejects first-class fetch references", () => {
    const reviewed = "fetch(0); fetch(1); fetch(2); fetch(3);";
    assert.equal(hasUnreviewedFetchReference(reviewed, true), false);
    assert.equal(hasUnreviewedFetchReference("// fetch only", false), false);
    for (const candidate of [
        "const send = fetch;",
        "const send = window.fetch;",
        'const send = globalThis["fetch"];',
        'const send = window?.["fetch"];',
        "const { fetch: send } = globalThis;",
        "fetch.call(window, target);",
        "fetch /* review */ (target);",
    ]) {
        assert.equal(hasUnreviewedFetchReference(
            `${reviewed}\n${candidate}`, true), true, candidate);
        assert.equal(hasUnreviewedFetchReference(candidate, false),
            true, candidate);
    }
});

test("browser transport review denies post-exit deferred fetch", () => {
    const reviewed = "fetch(0); fetch(1); fetch(2); fetch(3);";
    assert.equal(hasUnreviewedFetchReference(
        "// fetchLater is not executable", false), false);
    for (const candidate of [
        'fetchLater("/api/session/task");',
        'window.fetchLater("/api/session/task");',
        'const later = window.fetchLater;',
        'const later = globalThis["fetchLater"];',
        'const { fetchLater: later } = globalThis;',
        'window?.["fetchLater"]?.("/api/session/task");',
    ]) {
        assert.equal(hasUnreviewedFetchReference(
            `${reviewed}\n${candidate}`, true), true, candidate);
        assert.equal(hasUnreviewedFetchReference(candidate, false),
            true, candidate);
    }
});

test("browser source and generated modules reject undeclared transport",
    async () => {
    for (const { filename, source } of await browserModuleSources()) {
        const main = filename === "main.ts" || filename === "main.js";
        const paper = filename === "paper-profile.ts"
            || filename === "paper-profile.js";
        const fetchCalls = source.match(/\bfetch\s*\(/gu) ?? [];
        assert.equal(fetchCalls.length, main ? 4 : paper ? 1 : 0,
            `${filename} must retain only four reviewed fetch sites`);
        assert.equal(hasUnreviewedFetchReference(source, main, paper), false,
            `${filename} gained an unreviewed fetch reference`);
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

// Executable code loading can bypass the four reviewed fetch sites. The
// browser uses only statically declared modules; no runtime code compilation
// or dynamic import is admitted without a fresh authority review.
function hasUnreviewedExecutableCode(source, filename) {
    // All seven import declarations are fixed local module dependencies.
    // Review any new declaration or dynamic import before it can fetch code.
    const imports = {
        "main": [
            'import { parseBoundedJsonResponse } from ' +
                '"./session-diagnostic.js";',
            'import * as sessionDraft from "./session-draft.js";',
            'import * as sessionHandshake from "./session-handshake.js";',
            'import { sessionSecretFromFragment } from ' +
                '"./session-fragment.js";',
            'import { createPaperProfileEditor } from "./paper-profile.js";',
        ],
        "session-draft": [
            'import { decodeBoundedUtf8Response } from ' +
                '"./session-diagnostic.js";',
            'import { ownDataField, parseDiagnosticSet } from ' +
                '"./session-diagnostic.js";',
        ],
        "session-handshake": [
            'import { ownDataField, parseDiagnosticSet } from ' +
                '"./session-diagnostic.js";',
        ],
    };
    const moduleName = filename.replace(/\.(?:ts|js)$/u, "");
    let remaining = source.replace(/^\s*\/\/[^\n]*$/gmu, "");
    for (const site of imports[moduleName] ?? []) {
        if (remaining.split(site).length - 1 !== 1) {
            return true;
        }
        remaining = remaining.replace(site, "");
    }
    // Only two callback-based viewport timers are admitted in the editor.
    for (const site of [
        "window.setTimeout(resetLocalViewportState, 0)",
        "window.setTimeout(() => {",
    ]) {
        if (remaining.split(site).length - 1 !==
            ((moduleName === "main" || (moduleName === "paper-profile"
                && site === "window.setTimeout(() => {")) ? 1 : 0)) {
            return true;
        }
        remaining = remaining.replace(site, "");
    }
    const dynamicCode = new RegExp(
        String.raw`\b(?:import|eval|Function|AsyncFunction|GeneratorFunction|` +
            String.raw`AsyncGeneratorFunction|importScripts|WebAssembly|` +
            String.raw`setTimeout|setInterval)\b`, "u",
    );
    return dynamicCode.test(remaining);
}

test("browser runtime code loading requires explicit review", async () => {
    for (const { filename, source } of await browserModuleSources()) {
        assert.equal(hasUnreviewedExecutableCode(source, filename), false,
            `${filename} gained a code-loading or execution path`);
        for (const candidate of [
            'import("./unreviewed.js")',
            'import /* delayed */ ("./unreviewed.js")',
            'const moduleUrl = import.meta.url;',
            'eval(privateDraft)',
            'const execute = globalThis["eval"];',
            'new Function(privateDraft)()',
            'const make = globalThis["Function"];',
            'new AsyncFunction(privateDraft)',
            'new GeneratorFunction(privateDraft)',
            'new AsyncGeneratorFunction(privateDraft)',
            'importScripts("/unreviewed.js")',
            'WebAssembly.compile(bytes)',
            'setInterval(privateDraft, 0)',
            'window.setTimeout(privateDraft, 0)',
        ]) {
            assert.equal(hasUnreviewedExecutableCode(
                `${source}\n${candidate}`, filename), true,
            `${filename}: ${candidate} escaped executable-code review`);
        }
    }
});

// An alias to the entire global object can retrieve a browser capability
// without spelling its name in a direct-property audit. Preserve only the
// reviewed viewport, navigation cleanup, lifecycle and timer references.
function hasUnreviewedBrowserGlobalAuthority(source, main, paper = false) {
    // These exact expressions are the complete reviewed window authority.
    // Removing them before scanning also catches aliases to the root object.
    const allowed = [
        ["window.location.pathname", 1],
        ["window.location.search", 1],
        ["window.location.hash", 3],
        ['window.history.replaceState(null, "", localUrl)', 2],
        ["window.location.replace(localUrl)", 1],
        ['window.addEventListener("hashchange"', 1],
        ['window.addEventListener("pagehide"', 1],
        ['window.addEventListener("pageshow"', 1],
        ['window.addEventListener("blur"', 1],
        ['window.addEventListener("resize"', 1],
        ["window.location.reload()", 1],
        ["window.setTimeout(resetLocalViewportState, 0)", 1],
        ["window.setTimeout(() => {", 1],
        ["window.innerWidth", 1],
    ];
    let remaining = source.replace(/^\s*\/\/[^\n]*$/gmu, "");
    for (const [site, count] of allowed) {
        if (remaining.split(site).length - 1 !== (main ? count : paper && site === "window.setTimeout(() => {" ? 1 : 0)) {
            return true;
        }
        remaining = remaining.replaceAll(site, "");
    }
    return /\b(?:window|globalThis|self|top|parent|frames|opener)\b/u
        .test(remaining);
}

test("browser first-class global aliases require explicit review", async () => {
    const candidates = [
        "const root = window;",
        "const { navigator: host } = window;",
        "const root = globalThis;",
        'const host = globalThis?.["navigator"];',
        "const root = self;",
        "const root = top;",
        "const root = parent;",
        "const roots = frames;",
        "const sourceWindow = opener;",
        'const root = window["constructor"];',
        "const root = window?.navigator;",
        "window.addEventListener('message', onMessage);",
    ];
    for (const { filename, source } of await browserModuleSources()) {
        const main = filename === "main.ts" || filename === "main.js";
        assert.equal(hasUnreviewedBrowserGlobalAuthority(source, main,
            filename === "paper-profile.ts" || filename === "paper-profile.js"),
            false, `${filename} gained unreviewed browser global authority`);
        for (const candidate of candidates) {
            assert.equal(hasUnreviewedBrowserGlobalAuthority(
                `${source}\n${candidate}`, main,
                filename === "paper-profile.ts" || filename === "paper-profile.js"), true,
            `${filename}: ${candidate} bypassed the global-object review`);
        }
    }
});

// A DOM node can recover the full Document through ownerDocument, and a
// frame can expose another document even without an explicit global window.
// These references need the same first-class capability review as window.
function hasUnreviewedDocumentAuthority(source, main, typed, paper = false) {
    const allowed = [
        [typed ? "document.querySelector<T>(selector)"
            : "document.querySelector(selector)", 1],
        ["document.getElementById(targetId)", 1],
        ["document.documentElement.style", 2],
        ["document.activeElement", 4],
        ['document.addEventListener("visibilitychange"', 1],
        ["document.visibilityState", 1],
    ];
    let remaining = source.replace(/^\s*\/\/[^\n]*$/gmu, "");
    for (const [site, count] of allowed) {
        if (remaining.split(site).length - 1 !== (main ? count : 0)) {
            return true;
        }
        remaining = remaining.replaceAll(site, "");
    }
    // Exact documented paper editor control and one Blob download anchor.
    for (const site of [
        "document.getElementById(id)",
        'document.createElement("a")',
    ]) {
        if (remaining.split(site).length - 1 !== (paper ? 1 : 0)) {
            return true;
        }
        remaining = remaining.replace(site, "");
    }
    return new RegExp(
        String.raw`\b(?:document|Document|ownerDocument|` +
            String.raw`contentDocument|contentWindow|getRootNode)\b`, "u",
    ).test(remaining);
}

test("browser first-class Document authority requires explicit review",
    async () => {
    const probes = [
        "const root = document;",
        "const { cookie: privateValue } = document;",
        'const root = document["defaultView"];',
        "const forms = document.forms;",
        "const root = element.ownerDocument;",
        "const root = element?.ownerDocument;",
        'const root = element["ownerDocument"];',
        "const root = frame.contentDocument;",
        "const root = frame?.contentDocument;",
        "const root = frame.contentWindow;",
        "const root = element.getRootNode();",
        "const root = element.getRootNode({ composed: true });",
        "const constructor = Document;",
        'document.addEventListener("paste", onPaste);',
    ];
    for (const { filename, source } of await browserModuleSources()) {
        const main = filename === "main.ts" || filename === "main.js";
        const typed = filename.endsWith(".ts");
        assert.equal(hasUnreviewedDocumentAuthority(source, main, typed,
            filename === "paper-profile.ts" || filename === "paper-profile.js"),
            false, `${filename} gained unreviewed Document authority`);
        for (const candidate of probes) {
            assert.equal(hasUnreviewedDocumentAuthority(
                `${source}\n${candidate}`, main, typed,
                filename === "paper-profile.ts" || filename === "paper-profile.js"), true,
            `${filename}: ${candidate} bypassed Document review`);
        }
    }
});

// DevTools logging and browser timing/reporting APIs can expose private text
// or request metadata without invoking any of the four reviewed fetch calls.
// Require an explicit source review before adding those host surfaces.
function hasUnreviewedBrowserDiagnostics(source) {
    const executable = source.replace(/^\s*\/\/[^\n]*$/gmu, "");
    return new RegExp(
        String.raw`\b(?:console|Console|reportError|ReportingObserver|` +
            String.raw`ReportBody|performance|Performance[A-Za-z]*)\b`,
        "u",
    ).test(executable);
}

test("browser logging and timing authority requires explicit review",
    async () => {
    const probes = [
        "console.log(privateDraft);",
        "console.error(privateDraft);",
        "const log = console;",
        'const log = console["log"];',
        "const log = Console;",
        "reportError(new Error(privateDraft));",
        "const report = reportError;",
        "new ReportingObserver(handler);",
        "const report = ReportBody;",
        "performance.mark(privateDraft);",
        "performance.measure(privateDraft);",
        'const timeline = performance["getEntries"]();',
        "const timing = Performance;",
        "const timeline = PerformanceObserver;",
        "const entries = PerformanceResourceTiming;",
        "const navigation = PerformanceNavigationTiming;",
    ];
    for (const { filename, source } of await browserModuleSources()) {
        assert.equal(hasUnreviewedBrowserDiagnostics(source), false,
            `${filename} gained unreviewed browser diagnostic authority`);
        for (const candidate of probes) {
            assert.equal(hasUnreviewedBrowserDiagnostics(
                `${source}\n${candidate}`), true,
            `${filename}: ${candidate} bypassed diagnostic review`);
        }
    }
});

// A page can retain text through window.name, cookies, cache and shared
// browser storage APIs without Web Storage. Bracket access and first-class
// global references must not bypass this literal storage inventory.
function hasUnreviewedBrowserPersistenceReference(source) {
    const ambientStores = new RegExp(
        String.raw`\b(?:cookieStore|caches|sharedStorage|SharedStorage|` +
            String.raw`requestStorageAccess|requestStorageAccessFor|` +
            String.raw`hasStorageAccess|StorageAccessHandle|` +
            String.raw`storageBuckets|StorageBucket|StorageBucketManager|` +
            String.raw`StorageManager)\b`, "u",
    );
    const windowName = new RegExp(
        String.raw`\b(?:window|globalThis)\s*(?:(?:\?\.|\.)\s*name\b|` +
            String.raw`(?:\?\.)?\[\s*['"]name['"]\s*\])`, "u",
    );
    const documentCookie = new RegExp(
        String.raw`\bdocument\s*(?:(?:\?\.|\.)\s*cookie\b|` +
            String.raw`(?:\?\.)?\[\s*['"]cookie['"]\s*\])`, "u",
    );
    const destructuredWindowName =
        /\{\s*name\s*(?::\s*[\w$]+)?\s*\}\s*=\s*(?:window|globalThis)\b/u;
    const destructuredDocumentCookie =
        /\{\s*cookie\s*(?::\s*[\w$]+)?\s*\}\s*=\s*document\b/u;
    // Protocol handlers can outlive this disposable tab in the browser's
    // profile. An indexed or first-class method reference needs review too.
    const protocolRegistry =
        /\b(?:registerProtocolHandler|unregisterProtocolHandler)\b/u;
    const executable = source.replace(/^\s*\/\/[^\n]*$/gmu, "");
    return [ambientStores, windowName, documentCookie,
        destructuredWindowName, destructuredDocumentCookie]
        .some((pattern) => pattern.test(executable))
        || protocolRegistry.test(executable);
}

test("browser persistence review rejects indexed cookie, cache and name APIs",
    async () => {
    const allowed = "function f(name) { return name; }";
    assert.equal(hasUnreviewedBrowserPersistenceReference(allowed), false);
    assert.equal(hasUnreviewedBrowserPersistenceReference(
        "// registerProtocolHandler is not used\nexport const x = 1;"),
        false);
    const probes = [
        'window["name"] = privateText;',
        'globalThis?.["name"] = privateText;',
        'window . name = privateText;',
        'const { name } = window;',
        'const { name: prior } = globalThis;',
        'document["cookie"] = privateText;',
        'document?.["cookie"] = privateText;',
        'document . cookie;',
        'const { cookie: value } = document;',
        'cookieStore.get("session");',
        'const store = globalThis["cookieStore"];',
        'caches.open("workspace");',
        'const storage = window["caches"];',
        'navigator.registerProtocolHandler("web+sample", "/?url=%s");',
        'const register = navigator.registerProtocolHandler;',
        'const method = navigator["registerProtocolHandler"];',
        'const { registerProtocolHandler: register } = navigator;',
        'navigator.unregisterProtocolHandler("web+sample", "/?url=%s");',
        'const unregister = navigator?.unregisterProtocolHandler;',
        'const method = globalThis.navigator["unregisterProtocolHandler"];',
        'const { unregisterProtocolHandler: remove } = navigator;',
    ];
    for (const { filename, source } of await browserModuleSources()) {
        assert.equal(hasUnreviewedBrowserPersistenceReference(source), false,
            `${filename} gained an unreviewed persistent browser API`);
        for (const candidate of probes) {
            assert.equal(hasUnreviewedBrowserPersistenceReference(
                `${source}\n${candidate}`), true,
            `${filename}: ${candidate} escaped persistence review`);
        }
    }
});

// Storage Access can expose cookie state across contexts, while Shared
// Storage keeps origin data outside this disposable page's lifetime.
// First-class methods and literal-indexed aliases require owner review.
test("browser storage access and shared storage require review", async () => {
    assert.equal(hasUnreviewedBrowserPersistenceReference(
        "// sharedStorage is not used\nconst value = 1;"), false);
    const probes = [
        "document.requestStorageAccess()",
        "const request = document.requestStorageAccess;",
        'const request = document["requestStorageAccess"];',
        "document.requestStorageAccessFor(origin)",
        'const request = document["requestStorageAccessFor"];',
        "document.hasStorageAccess()",
        "const check = document.hasStorageAccess;",
        "void StorageAccessHandle;",
        "window.sharedStorage.set('key', 'value');",
        'const shared = globalThis["sharedStorage"];',
        "const shared = navigator.sharedStorage;",
        "void SharedStorage;",
    ];
    for (const { filename, source } of await browserModuleSources()) {
        assert.equal(hasUnreviewedBrowserPersistenceReference(source), false,
            `${filename} gained an unreviewed shared storage capability`);
        for (const candidate of probes) {
            assert.equal(hasUnreviewedBrowserPersistenceReference(
                `${source}\n${candidate}`), true,
            `${filename}: ${candidate} escaped storage access review`);
        }
    }
});

// Storage Buckets may retain origin data beyond a disposable tab. The bare
// navigator.storage spelling does not cover optional or indexed access to
// navigator.storageBuckets and its separate bucket interfaces.
test("browser storage buckets require explicit persistence review",
    async () => {
    const probes = [
        'navigator["storageBuckets"].open("drafts")',
        "navigator?.storageBuckets.open('drafts')",
        'const buckets = navigator?.["storageBuckets"];',
        'const buckets = globalThis.navigator["storageBuckets"];',
        "void StorageBucket;",
        "void StorageBucketManager;",
        "void StorageManager;",
    ];
    for (const { filename, source } of await browserModuleSources()) {
        assert.equal(hasUnreviewedBrowserPersistenceReference(source), false,
            `${filename} gained unreviewed origin storage`);
        for (const candidate of probes) {
            assert.equal(hasUnreviewedBrowserPersistenceReference(
                `${source}\n${candidate}`), true,
            `${filename}: ${candidate} escaped storage-bucket review`);
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


// Browser-managed attribution, interest groups and Private State Tokens can
// retain data or schedule reports outside the four reviewed fetch sites.
// Named constructors, options and first-class method references require
// review even without an explicit cross-origin request in workspace source.
function hasUnreviewedBrowserTrackingReference(source) {
    const executable = source.replace(/^\s*\/\/[^\n]*$/gmu, "");
    return new RegExp(
        String.raw`\b(?:attributionSrc|attributionsrc|attributionReporting|` +
            String.raw`setAttributionReporting|joinAdInterestGroup|` +
            String.raw`leaveAdInterestGroup|runAdAuction|` +
            String.raw`updateAdInterestGroups|hasTrustToken|` +
            String.raw`hasPrivateToken|hasRedemptionRecord|privateToken)\b`,
        "u",
    ).test(executable);
}

test("browser deferred reporting and private-token APIs require review",
    async () => {
    assert.equal(hasUnreviewedBrowserTrackingReference(
        "// privateToken is not used\nconst value = 1;"), false);
    const probes = [
        "navigator.joinAdInterestGroup({ owner: origin }, 1000);",
        'const join = navigator["joinAdInterestGroup"];',
        "navigator.leaveAdInterestGroup({ owner: origin, name: 'x' });",
        "navigator.updateAdInterestGroups();",
        "navigator.runAdAuction({ seller: origin });",
        'const auction = navigator?.["runAdAuction"];',
        "document.hasTrustToken(origin);",
        'const token = document["hasTrustToken"];',
        "document.hasPrivateToken(origin);",
        'const record = document?.["hasRedemptionRecord"];',
        "new Request(path, { privateToken: { operation: 'token-request' } });",
        "const hint = { attributionReporting: { eventSourceEligible: true } };",
        "const attribute = anchor.attributionSrc;",
        'anchor["attributionSrc"] = destination;',
        'const markup = "attributionsrc";',
        "xhr.setAttributionReporting(options);",
        'const report = xhr["setAttributionReporting"];',
    ];
    for (const { filename, source } of await browserModuleSources()) {
        assert.equal(hasUnreviewedBrowserTrackingReference(source), false,
            `${filename} gained browser-managed reporting or token storage`);
        for (const candidate of probes) {
            assert.equal(hasUnreviewedBrowserTrackingReference(
                `${source}\n${candidate}`), true,
            `${filename}: ${candidate} escaped browser-private data review`);
        }
    }
});

// A popup can navigate to an unrelated URL without using the reviewed
// fetch sites. The bare global open and window.open are both capturable;
// bracket indexing cannot be approved merely because it lacks a call.
function hasUnreviewedPopupOpen(source) {
    const executable = source.replace(/^\s*\/\/[^\n]*$/gmu, "");
    return /\bopen\b/u.test(executable);
}

test("browser cross-context review rejects popup opener aliases", () => {
    assert.equal(hasUnreviewedPopupOpen("// open is not used"), false);
    for (const candidate of [
        "open(target)",
        "window.open(target)",
        "globalThis.open(target)",
        'window["open"](target)',
        'globalThis?.["open"](target)',
        "const launch = open;",
        "const launch = window.open;",
        "const { open: launch } = globalThis;",
    ]) {
        assert.equal(hasUnreviewedPopupOpen(candidate), true, candidate);
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
        assert.equal(hasUnreviewedPopupOpen(source), false,
            `${filename} gained popup navigation authority`);
    }
});

// Inbound Window message and storage events let unrelated contexts influence
// a session without using fetch, popup creation, or outbound postMessage.
// Require review before any handler or first-class event constructor appears.
// This is a conservative literal inventory, not a browser message-flow proof.
function hasUnreviewedInboundCrossContextEvents(source) {
    const eventProperties =
        /\b(?:onmessage|onmessageerror|onstorage|MessageEvent|StorageEvent)\b/u;
    return eventProperties.test(source)
        || /(["'`])(?:message|messageerror|storage)\1/u.test(source);
}

test("browser inbound cross-context events require explicit review",
    async () => {
    const candidates = [
        'window.addEventListener("message", () => {});',
        "globalThis.addEventListener('messageerror', handler);",
        'document.body.addEventListener(`message`, handler);',
        'window.addEventListener("storage", handler);',
        "window.onmessage = handler;",
        'globalThis["onmessageerror"] = handler;',
        "const { onstorage: handler } = window;",
        "const factory = MessageEvent;",
        "const factory = StorageEvent;",
        'const eventType = "message";',
    ];
    assert.equal(hasUnreviewedInboundCrossContextEvents(
        'window.addEventListener("pagehide", handler);'), false);
    for (const { filename, source } of await browserModuleSources()) {
        assert.equal(hasUnreviewedInboundCrossContextEvents(source), false,
            `${filename} gained an unreviewed inbound cross-context event`);
        for (const candidate of candidates) {
            assert.equal(hasUnreviewedInboundCrossContextEvents(
                `${source}\n${candidate}`), true,
            `${filename}: ${candidate} escaped inbound event review`);
        }
    }
});

// Browser-selected files and origin-private filesystem directories are
// separate from the backend's explicit, selected-path import/export API.
// Require source review before new filesystem handles appear in the UI.
function hasUnreviewedBrowserFileAuthority(source) {
    const namedFileAuthority = new RegExp(
        String.raw`\b(?:showOpenFilePicker|showSaveFilePicker|` +
            String.raw`showDirectoryPicker|requestFileSystem|` +
            String.raw`webkitRequestFileSystem|chooseFileSystemEntries|` +
            String.raw`getAsFileSystemHandle|FileSystemHandle|` +
            String.raw`FileSystemFileHandle|FileSystemDirectoryHandle)\b`,
        "u",
    );
    const indexedStorage = new RegExp(
        String.raw`\bnavigator\s*(?:\?\.)?\[\s*['"]storage['"]\s*\]`,
        "u",
    );
    return namedFileAuthority.test(source) || indexedStorage.test(source);
}

test("browser filesystem authority inventory rejects unreviewed access", () => {
    assert.equal(hasUnreviewedBrowserFileAuthority(
        "const source = document.querySelector('textarea');"), false);
    for (const candidate of [
        "window.showOpenFilePicker()",
        'const select = globalThis["showSaveFilePicker"];',
        "showDirectoryPicker()",
        "window.webkitRequestFileSystem(type, size, ok, fail)",
        "requestFileSystem(type, size, ok, fail)",
        'window["chooseFileSystemEntries"]()',
        "item.getAsFileSystemHandle()",
        "const handle = FileSystemFileHandle;",
        "const directory = FileSystemDirectoryHandle;",
        "const handle = FileSystemHandle;",
        'navigator["storage"].getDirectory()',
        "navigator?.['storage']?.getDirectory()",
    ]) {
        assert.equal(hasUnreviewedBrowserFileAuthority(candidate), true,
            candidate);
    }
});

// Ordinary HTML file inputs, drag-and-drop FileList, and FileReader are
// distinct paths from the File System Access pickers reviewed above.
// Requiring review does not imply the user selected any local file.
function hasUnreviewedBrowserFileIntake(source, paper = false, typed = false) {
    let reviewed = source.replace(/^\s*\/\/[^\n]*$/gmu, "");
    // Counts restrict file input to one selected JSON and one Blob download.
    for (const [site, count] of [
        ["URL.createObjectURL(blob)", 1],
        ["upload.files?.item(0)", 1],
        ['new Blob([result.json], { type: "application/json" })', 1],
        ["blob: Blob", typed ? 1 : 0],
    ]) {
        if (reviewed.split(site).length - 1 !== (paper ? count : 0)) {
            return true;
        }
        reviewed = reviewed.replace(site, "");
    }
    return [
        new RegExp(
            String.raw`\b(?:FileReader|FileReaderSync|FileList|` +
                String.raw`DataTransfer|DataTransferItem|Blob|` +
                String.raw`getAsFile|webkitGetAsEntry|createObjectURL)\b`,
            "u",
        ),
        /\.\s*files\b/u,
        /\[\s*['"]files['"]\s*\]/u,
    ].some((pattern) => pattern.test(reviewed));
}

test("browser ordinary file input and drag-file reads require review",
    async () => {
    const html = await readFile(INDEX_HTML, "utf8");
    assert.equal(usesUnreviewedMarkupRequests(html), false);
    for (const candidate of [
        '<input type="file">',
        "<input type='FILE' accept='image/jpeg'>",
        '<input type=file multiple>',
        '<input webkitdirectory>',
    ]) {
        assert.equal(usesUnreviewedMarkupRequests(
            html.replace("</body>", `${candidate}</body>`)), true,
        candidate);
    }
    for (const { filename, source } of await browserModuleSources()) {
        assert.equal(hasUnreviewedBrowserFileIntake(source,
            filename === "paper-profile.ts" || filename === "paper-profile.js",
            filename.endsWith(".ts")), false,
            `${filename} gained an unreviewed local-file read`);
        for (const candidate of [
            "void FileReader;",
            "new FileReader();",
            "new FileReaderSync();",
            "void FileList;",
            "void DataTransfer;",
            "void DataTransferItem;",
            "void Blob;",
            "item.getAsFile();",
            "item.webkitGetAsEntry();",
            "URL.createObjectURL(blob);",
            "input.files;",
            'input["files"];',
            "dragEvent.dataTransfer.files;",
        ]) {
            assert.equal(hasUnreviewedBrowserFileIntake(
                `${source}\n${candidate}`,
                filename === "paper-profile.ts" || filename === "paper-profile.js",
                filename.endsWith(".ts")), true,
            `${filename}: ${candidate} escaped ordinary-file review`);
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
        String.raw`language|languages|platform|oscpu|vendor|appVersion|` +
        String.raw`productSub|maxTouchPoints|pdfViewerEnabled|onLine|` +
        String.raw`mediaCapabilities|connection|getBattery|getGamepads|` +
        String.raw`bluetooth|usb|` +
        String.raw`serial|hid|gpu|keyboard|permissions|requestMIDIAccess|` +
        String.raw`contacts|xr|wakeLock)`;
    const ambientRead = new RegExp(
        String.raw`\bnavigator\s*(?:(?:\?\.|\.)\s*|(?:\?\.)?\[\s*['"])` +
            hostProperty + String.raw`\b`,
        "u",
    );
    // Legacy media, contacts, local fonts, screen pixels and idle status
    // can reveal host data without touching navigator.mediaDevices.
    const directCapture = new RegExp(
        String.raw`\b(?:SpeechRecognition|webkitSpeechRecognition|` +
            String.raw`SpeechGrammarList|webkitSpeechGrammarList|` +
            String.raw`getUserMedia|webkitGetUserMedia|` +
            String.raw`mozGetUserMedia|msGetUserMedia|` +
            String.raw`requestMIDIAccess|MIDIAccess|MIDIPort|MIDIInput|` +
            String.raw`MIDIOutput|MIDIMessageEvent|NDEFReader|NDEFMessage|` +
            String.raw`NDEFRecord|NDEFReadingEvent|ContactsManager|` +
            String.raw`queryLocalFonts|FontData|EyeDropper|` +
            String.raw`getScreenDetails|IdleDetector|DeviceMotionEvent|` +
            String.raw`DeviceOrientationEvent|Accelerometer|` +
            String.raw`LinearAccelerationSensor|GravitySensor|Gyroscope|` +
            String.raw`Magnetometer|AmbientLightSensor|ProximitySensor|` +
            String.raw`AbsoluteOrientationSensor|RelativeOrientationSensor|` +
            String.raw`XR[A-Z][A-Za-z0-9_]*|PublicKeyCredential|` +
            String.raw`AuthenticatorResponse|AuthenticatorAssertionResponse|` +
            String.raw`AuthenticatorAttestationResponse|CredentialsContainer|` +
            String.raw`PasswordCredential|FederatedCredential|` +
            String.raw`IdentityCredential|IdentityProvider|OTPCredential|` +
            String.raw`DigitalCredential|KeyboardLayoutMap|getLayoutMap|` +
            String.raw`WakeLock|WakeLockSentinel|ScreenOrientation)\b`,
        "u",
    );
    const motionEvents =
        /\b(?:on)?(?:devicemotion|deviceorientation(?:absolute)?)\b/u;
    // A first-class Screen reference permits future physical display reads
    // even when the original expression never names width or color depth.
    // Keep the existing viewport-only window.innerWidth path admitted.
    const physicalScreen =
        /\b(?:screen|Screen|ScreenDetailed|ScreenDetails)\b/u;
    // Topics can disclose browsing interests even without session-network
    // calls or the navigator credential/device interfaces above.
    const browsingInterests = /\bbrowsingTopics\b/u;
    if (ambientRead.test(source)
        || physicalScreen.test(source)
        || directCapture.test(source)
        || browsingInterests.test(source)
        || motionEvents.test(source)
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
    // The clipboard writer is the entire admitted navigator capability.
    // Count aliases, destructured properties, and literal-indexed roots too;
    // a new host API must not bypass review by hiding its property name.
    const executable = source.replace(/^\s*\/\/[^\n]*$/gmu, "");
    const navigatorReferences = executable.match(/\bnavigator\b/gu) ?? [];
    return clipboardAccesses.length !== (main ? 1 : 0)
        || clipboardWrites.length !== (main ? 1 : 0)
        || navigatorReferences.length !== (main ? 1 : 0);
}

// The one Copy prompt writeText acquisition is the only reviewed clipboard
// text output path. First-class/indexed aliases and legacy execCommand copy
// would allow unrelated private draft output without a new fetch or popup.
function hasUnreviewedClipboardWrite(source, main) {
    const acquisition = "const writeText = clipboard?.writeText;";
    const acquisitions = source.split(acquisition).length - 1;
    const executable = source.replace(/^\s*\/\/[^\n]*$/gmu, "");
    const methodReferences = executable.match(/\bwriteText\b/gu) ?? [];
    const clipboardReferences = executable.match(/\bclipboard\b/gu) ?? [];
    return acquisitions !== (main ? 1 : 0)
        || methodReferences.length !== (main ? 4 : 0)
        || clipboardReferences.length !== (main ? 4 : 0)
        || /\b(?:execCommand|clipboardData|ClipboardItem)\b/u
            .test(executable);
}

test("browser clipboard-write paths require explicit source review",
    async () => {
    const reviewed = "const clipboard = navigator.clipboard; " +
        "const writeText = clipboard?.writeText; " +
        "if (typeof writeText === 'function') { writeText.bind(clipboard); }";
    assert.equal(hasUnreviewedClipboardWrite(reviewed, true), false);
    assert.equal(hasUnreviewedClipboardWrite("export const x = 1;", false),
        false);
    for (const { filename, source } of await browserModuleSources()) {
        const main = filename === "main.ts" || filename === "main.js";
        assert.equal(hasUnreviewedClipboardWrite(source, main), false,
            `${filename} gained an unreviewed clipboard-write path`);
        for (const candidate of [
            'const other = clipboard["writeText"];',
            "const other = clipboard?.writeText;",
            "const { writeText: other } = clipboard;",
            "const other = navigator.clipboard.writeText;",
            'clipboard?.["writeText"](privateDraft);',
            'document.execCommand("copy");',
            'window["clipboardData"].setData("Text", privateDraft);',
            "const { write } = clipboard;",
            'const writer = clipboard["wri" + "teText"];',
            "void ClipboardItem;",
        ]) {
            assert.equal(hasUnreviewedClipboardWrite(
                `${source}\n${candidate}`, main), true,
            `${filename}: ${candidate} escaped clipboard-output review`);
        }
    }
});

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
        "new SpeechRecognition()",
        "new window.webkitSpeechRecognition()",
        'const recognition = globalThis["SpeechRecognition"];',
        "new SpeechGrammarList()",
        "new webkitSpeechGrammarList()",
        "navigator.getUserMedia({ audio: true }, yes, no)",
        "navigator.webkitGetUserMedia({ video: true }, yes, no)",
        "navigator.mozGetUserMedia({ audio: true }, yes, no)",
        "navigator.msGetUserMedia({ audio: true }, yes, no)",
        'const capture = navigator["getUserMedia"];',
        "navigator.requestMIDIAccess({ sysex: true })",
        'const midi = navigator["requestMIDIAccess"];',
        "const request = navigator?.requestMIDIAccess;",
        "void requestMIDIAccess;",
        "void MIDIAccess;",
        "void MIDIInput;",
        "void MIDIOutput;",
        "void MIDIPort;",
        "void MIDIMessageEvent;",
        "new NDEFReader();",
        'const reader = globalThis["NDEFReader"];',
        "void NDEFMessage;",
        "void NDEFRecord;",
        "void NDEFReadingEvent;",
    ]) {
        assert.equal(usesUnreviewedBrowserHostInput(
            allowedWrite + candidate, true), true, candidate);
        assert.equal(usesUnreviewedBrowserHostInput(candidate, false),
            true, candidate);
    }
});

// A first-class navigator alias can subsequently access any new host API
// without a literal navigator.property expression. Restrict the initial
// navigator capability itself, retaining only the reviewed clipboard writer.
test("browser first-class navigator aliases require host review", async () => {
    const reviewed = "const clipboard = navigator.clipboard; " +
        "const writeText = clipboard?.writeText;";
    assert.equal(usesUnreviewedBrowserHostInput(reviewed, true), false);
    assert.equal(usesUnreviewedBrowserHostInput(
        "// navigator is not used\nexport const x = 1;", false), false);
    const probes = [
        "const device = navigator;",
        "const device = window.navigator;",
        "const device = globalThis.navigator;",
        'const device = window["navigator"];',
        'const device = globalThis?.["navigator"];',
        "const { language: locale } = navigator;",
        "const { geolocation: location } = window.navigator;",
        "const { platform } = globalThis.navigator;",
        'const { ["languages"]: locale } = navigator;',
        'const unknown = navigator["unreviewedFeature"];',
    ];
    for (const { filename, source } of await browserModuleSources()) {
        const main = filename === "main.ts" || filename === "main.js";
        assert.equal(usesUnreviewedBrowserHostInput(source, main), false,
            `${filename} gained unreviewed first-class host authority`);
        for (const candidate of probes) {
            assert.equal(usesUnreviewedBrowserHostInput(
                `${source}\n${candidate}`, main), true,
            `${filename}: ${candidate} bypassed navigator alias review`);
        }
    }
});

// Keyboard Map reveals the physical keyboard's localized labels without
// reading a draft or using a reviewed network request. Keyboard locking
// also captures host key input and needs an explicit source review.
test("browser keyboard layout APIs require host-input review", async () => {
    const probes = [
        "navigator.keyboard.getLayoutMap()",
        'const keyboard = navigator["keyboard"];',
        "const keyboard = globalThis.navigator?.keyboard;",
        "const keyboard = navigator?.['keyboard'];",
        "const map = keyboard.getLayoutMap;",
        'const map = keyboard["getLayoutMap"];',
        "void KeyboardLayoutMap;",
        "navigator.keyboard.lock(['KeyW']);",
    ];
    for (const { filename, source } of await browserModuleSources()) {
        const main = filename === "main.ts" || filename === "main.js";
        assert.equal(usesUnreviewedBrowserHostInput(source, main), false,
            `${filename} gained an unreviewed host keyboard read`);
        for (const candidate of probes) {
            assert.equal(usesUnreviewedBrowserHostInput(
                `${source}\n${candidate}`, main), true,
            `${filename}: ${candidate} escaped keyboard authority review`);
        }
    }
});

// Screen wake locks can keep the device awake, while screen orientation
// exposes display state and can lock presentation direction. Neither is
// required by the page editor's responsive window-size layout.
test("browser screen power and orientation need host review", async () => {
    const probes = [
        "navigator.wakeLock.request('screen')",
        'const power = navigator["wakeLock"];',
        "const power = navigator?.wakeLock;",
        "void WakeLock;",
        "void WakeLockSentinel;",
        "screen.orientation.lock('portrait');",
        'const rotation = window.screen["orientation"];',
        'const rotation = screen?.["orientation"];',
        "void ScreenOrientation;",
    ];
    for (const { filename, source } of await browserModuleSources()) {
        const main = filename === "main.ts" || filename === "main.js";
        assert.equal(usesUnreviewedBrowserHostInput(source, main), false,
            `${filename} gained unreviewed screen power or orientation`);
        for (const candidate of probes) {
            assert.equal(usesUnreviewedBrowserHostInput(
                `${source}\n${candidate}`, main), true,
            `${filename}: ${candidate} escaped screen authority review`);
        }
    }
});

// Physical display metrics identify monitor resolution, available screen
// bounds and color depth rather than the editor's own viewport. These host
// properties are not required by the responsive workspace.
test("browser physical screen metrics require host-input review", async () => {
    const probes = [
        "screen.width",
        "screen.availWidth",
        "screen.colorDepth",
        'const display = window["screen"];',
        "const display = globalThis?.screen;",
        "const display = screen;",
        "const { width } = window.screen;",
        "void Screen;",
        "void ScreenDetailed;",
        "void ScreenDetails;",
    ];
    assert.equal(usesUnreviewedBrowserHostInput(
        "const width = window.innerWidth;", false), false);
    for (const { filename, source } of await browserModuleSources()) {
        const main = filename === "main.ts" || filename === "main.js";
        assert.equal(usesUnreviewedBrowserHostInput(source, main), false,
            `${filename} gained an unreviewed display metric read`);
        for (const candidate of probes) {
            assert.equal(usesUnreviewedBrowserHostInput(
                `${source}\n${candidate}`, main), true,
            `${filename}: ${candidate} escaped display metric review`);
        }
    }
});

// Locale, OS and decoder-capability metadata can fingerprint the user's host
// without a permission prompt or an API request. The workspace currently
// derives character boundaries through Intl, not direct navigator metadata.
test("browser locale and platform metadata require host-input review",
    async () => {
    const probes = [
        "navigator.language",
        "navigator.languages",
        'const locale = navigator["language"];',
        "const locale = window.navigator?.languages;",
        "navigator.platform",
        "navigator.oscpu",
        "navigator.vendor",
        "navigator.appVersion",
        "navigator.productSub",
        "navigator.maxTouchPoints",
        "navigator.pdfViewerEnabled",
        "navigator.onLine",
        "navigator.mediaCapabilities.decodingInfo(config)",
        'const codecs = globalThis.navigator?.["mediaCapabilities"];',
    ];
    assert.equal(usesUnreviewedBrowserHostInput(
        "new Intl.Segmenter(undefined, { granularity: 'grapheme' });",
        false), false);
    for (const { filename, source } of await browserModuleSources()) {
        const main = filename === "main.ts" || filename === "main.js";
        assert.equal(usesUnreviewedBrowserHostInput(source, main), false,
            `${filename} gained an unreviewed host-metadata read`);
        for (const candidate of probes) {
            assert.equal(usesUnreviewedBrowserHostInput(
                `${source}\n${candidate}`, main), true,
            `${filename}: ${candidate} escaped host-metadata review`);
        }
    }
});

// The browser Topics API can reveal browsing interests without accessing
// the session draft or any reviewed fetch site. Require source review before
// reading or capturing the method, including bracket and destructuring forms.
test("browser Topics interest reads require host-input review", async () => {
    const probes = [
        "document.browsingTopics()",
        "window.document.browsingTopics()",
        'globalThis.document["browsingTopics"]()',
        "const read = document.browsingTopics;",
        "const { browsingTopics: read } = document;",
        'document?.["browsingTopics"]?.()',
    ];
    for (const { filename, source } of await browserModuleSources()) {
        const main = filename === "main.ts" || filename === "main.js";
        assert.equal(usesUnreviewedBrowserHostInput(source, main), false,
            `${filename} gained an unreviewed host-data read`);
        for (const candidate of probes) {
            assert.equal(usesUnreviewedBrowserHostInput(
                `${source}\n${candidate}`, main), true,
            `${filename}: ${candidate} escaped Topics API review`);
        }
    }
});

// The contact picker, local font enumeration, screen picker, window
// placement and idle detection expose user or host data beyond draft text.
// They need explicit admission even if a browser will prompt the user.
test("browser contact and desktop host-data references need review",
    async () => {
    const probes = [
        'navigator.contacts.select(["name"])',
        'const contacts = navigator?.["contacts"];',
        "const picker = navigator.contacts;",
        "void ContactsManager;",
        "window.queryLocalFonts()",
        'const fonts = globalThis["queryLocalFonts"];',
        "void FontData;",
        "new EyeDropper();",
        'const picker = window["EyeDropper"];',
        "window.getScreenDetails()",
        'const details = window["getScreenDetails"];',
        "new IdleDetector();",
        'const detector = globalThis["IdleDetector"];',
    ];
    for (const { filename, source } of await browserModuleSources()) {
        const main = filename === "main.ts" || filename === "main.js";
        assert.equal(usesUnreviewedBrowserHostInput(source, main), false,
            `${filename} gained an unreviewed host-data read`);
        for (const candidate of probes) {
            assert.equal(usesUnreviewedBrowserHostInput(
                `${source}\n${candidate}`, main), true,
            `${filename}: ${candidate} escaped desktop host-data review`);
        }
    }
});

// Browser motion/orientation events and ambient sensor constructors can
// reveal device movement or surroundings without any mediaDevices access.
// Requiring source review does not assert that permissions are unavailable.
test("browser ambient motion and sensor references need review", async () => {
    const probes = [
        'window.addEventListener("devicemotion", handler)',
        'window.addEventListener("deviceorientation", handler)',
        'window.addEventListener("deviceorientationabsolute", handler)',
        "window.ondevicemotion = handler;",
        'globalThis["ondeviceorientation"] = handler;',
        "new DeviceMotionEvent('devicemotion');",
        "void DeviceOrientationEvent;",
        "new Accelerometer();",
        "new LinearAccelerationSensor();",
        "new GravitySensor();",
        "new Gyroscope();",
        "new Magnetometer();",
        "new AmbientLightSensor();",
        "new ProximitySensor();",
        "new AbsoluteOrientationSensor();",
        "new RelativeOrientationSensor();",
    ];
    for (const { filename, source } of await browserModuleSources()) {
        const main = filename === "main.ts" || filename === "main.js";
        assert.equal(usesUnreviewedBrowserHostInput(source, main), false,
            `${filename} gained an unreviewed ambient sensor read`);
        for (const candidate of probes) {
            assert.equal(usesUnreviewedBrowserHostInput(
                `${source}\n${candidate}`, main), true,
            `${filename}: ${candidate} escaped ambient sensor review`);
        }
    }
});

// WebXR exposes device pose, input controllers, and potentially camera pixels.
// Its navigator.xr entrypoint and first-class XR handles do not require
// navigator.mediaDevices or a generic permissions reference in source.
test("browser XR device sensing requires explicit source review", async () => {
    const probes = [
        "navigator.xr.requestSession('immersive-ar')",
        "const device = navigator?.xr;",
        'const device = navigator["xr"];',
        'const device = globalThis.navigator?.["xr"];',
        "void XRSystem;",
        "void XRSession;",
        "void XRFrame;",
        "void XRReferenceSpace;",
        "void XRInputSource;",
        "void XRHitTestSource;",
        "void XRCamera;",
        "void XRWebGLLayer;",
    ];
    for (const { filename, source } of await browserModuleSources()) {
        const main = filename === "main.ts" || filename === "main.js";
        assert.equal(usesUnreviewedBrowserHostInput(source, main), false,
            `${filename} gained unreviewed device sensing authority`);
        for (const candidate of probes) {
            assert.equal(usesUnreviewedBrowserHostInput(
                `${source}\n${candidate}`, main), true,
            `${filename}: ${candidate} escaped WebXR review`);
        }
    }
});

// Credential Management and WebAuthn expose identity and authenticator
// properties through named objects as well as navigator.credentials.
// A new first-class reference therefore needs explicit source review.
test("browser credential objects require host-input review", async () => {
    const probes = [
        "PublicKeyCredential.isUserVerifyingPlatformAuthenticatorAvailable()",
        'const passkey = window["PublicKeyCredential"];',
        "void AuthenticatorResponse;",
        "void AuthenticatorAssertionResponse;",
        "void AuthenticatorAttestationResponse;",
        "void CredentialsContainer;",
        "new PasswordCredential({ id: 'user', password: 'pass' });",
        "new FederatedCredential({ id: 'user', provider: 'provider' });",
        "void IdentityCredential;",
        "void IdentityProvider;",
        "void OTPCredential;",
        "void DigitalCredential;",
    ];
    for (const { filename, source } of await browserModuleSources()) {
        const main = filename === "main.ts" || filename === "main.js";
        assert.equal(usesUnreviewedBrowserHostInput(source, main), false,
            `${filename} gained unreviewed credential authority`);
        for (const candidate of probes) {
            assert.equal(usesUnreviewedBrowserHostInput(
                `${source}\n${candidate}`, main), true,
            `${filename}: ${candidate} escaped credential-object review`);
        }
    }
});

test("browser source and emitted modules reject unreviewed host reads",
    async () => {
    for (const { filename, source } of await browserModuleSources()) {
        const main = filename === "main.ts" || filename === "main.js";
        assert.equal(usesUnreviewedBrowserHostInput(source, main), false,
            `${filename} gained clipboard or ambient device read authority`);
        assert.equal(hasUnreviewedBrowserFileAuthority(source), false,
            `${filename} gained unreviewed browser file authority`);
    }
});

// Navigation, dynamic element loading, and form submission can send a
// request without invoking the four reviewed fetch sites. Only fragment
// scrubbing and bfcache reload are approved in the main workspace module.
// This guards obvious source drift, not dynamically aliased objects or the
// browser's complete navigation and network execution paths.
function usesUnreviewedBrowserNavigation(source, main, paper = false) {
    // The complete known location/history authority is in the main module.
    // Permit only these exact expressions: a captured method or computed
    // alias can otherwise navigate without a direct .assign() call.
    let remaining = source.replace(/^\s*\/\/[^\n]*$/gmu, "");
    for (const [site, count] of [
        ["window.location.pathname", 1],
        ["window.location.search", 1],
        ["window.location.hash", 3],
        ["window.location.replace(localUrl)", 1],
        ["window.location.reload()", 1],
        ['window.history.replaceState(null, "", localUrl)', 2],
    ]) {
        if (remaining.split(site).length - 1 !== (main ? count : 0)) {
            return true;
        }
        remaining = remaining.replaceAll(site, "");
    }
    // One anchor click is confined to an explicit local Blob download.
    if (remaining.split("link.click()").length - 1 !== (paper ? 1 : 0)) {
        return true;
    }
    remaining = remaining.replace("link.click()", "");
    const locationMethod = String.raw`(?:assign|replace|reload)`;
    const locationWritable = String.raw`(?:href|search|hash|pathname|` +
        String.raw`host|hostname|protocol)`;
    const locationAccess = String.raw`\b(?:` +
        String.raw`(?:(?:window|document|globalThis)\s*\.\s*)?location|` +
        String.raw`(?:window|document|globalThis)\s*(?:\?\.)?` +
        String.raw`\[\s*['"]location['"]\s*\])\s*`;
    const locationNavigation = new RegExp(
        locationAccess +
            String.raw`(?:=(?!=)|\.\s*` + locationMethod +
            String.raw`\s*\(|\.\s*` + locationWritable +
            String.raw`\s*=(?!=)|\[\s*['"](?:href|search|hash|pathname|` +
            String.raw`host|assign|replace|reload)['"]\s*\]` +
            String.raw`\s*(?:\(|=(?!=)))`,
        "u",
    );
    const historyMethod = String.raw`(?:pushState|replaceState|go|` +
        String.raw`back|forward)`;
    const historyAccess = String.raw`\b(?:` +
        String.raw`(?:(?:window|globalThis)\s*\.\s*)?history|` +
        String.raw`(?:window|globalThis)\s*(?:\?\.)?` +
        String.raw`\[\s*['"]history['"]\s*\])\s*`;
    const historyNavigation = new RegExp(
        historyAccess +
            String.raw`(?:\.\s*` + historyMethod + String.raw`\s*\(|` +
            String.raw`\[\s*['"]` + historyMethod +
            String.raw`['"]\s*\]\s*\()`,
        "u",
    );
    const navigation = [
        locationNavigation,
        historyNavigation,
        // The newer Navigation API can leave the page or reload without
        // calling location/history. Review both direct and captured access.
        /\bnavigation\b/u,
        /\.\s*(?:submit|requestSubmit|click)\s*\(/u,
        /\[\s*['"](?:submit|requestSubmit|click)['"]\s*\]\s*\(/u,
        /\bnew\s+(?:Image|Audio)\s*\(/u,
        // Renaming a location/history object hides subsequent methods from
        // these surface checks; require review of the alias itself.
        new RegExp(
            String.raw`\b(?:const|let|var)\s+[\w$]+\s*=\s*` +
                String.raw`(?:window\s*\.\s*)?(?:location|history)` +
                String.raw`\b(?!\s*[.[])`,
            "u",
        ),
    ];
    // Standalone location/history are ambient browser globals. Reject the
    // first-class references too, not only named mutations: destructuring,
    // optional chaining and bound methods can hide a later navigation.
    return /\b(?:location|Location|history|History)\b/u.test(remaining)
        || navigation.some((pattern) => pattern.test(remaining));
}

const CAPTURED_NAVIGATION_PROBES = [
    "const navigate = location.assign.bind(location);",
    'const navigate = location["assign"].bind(location);',
    "const navigate = location?.replace;",
    "const { assign: navigate } = location;",
    "const { href: target } = location;",
    "const handle = Location;",
    "const go = history.pushState.bind(history);",
    'const go = history["replaceState"];',
    "const { pushState: go } = history;",
    "const handle = History;",
];

test("browser navigation inventory distinguishes reviewed local calls", () => {
    const reviewed = [
        'window.history.replaceState(null, "", localUrl);',
        'window.history.replaceState(null, "", localUrl);',
        "window.location.replace(localUrl);",
        "window.location.reload();",
        "const hash = window.location.hash;",
        "const otherHash = window.location.hash;",
        "const lastHash = window.location.hash;",
        "const local = window.location.pathname + window.location.search;",
    ].join("\n");
    assert.equal(usesUnreviewedBrowserNavigation(reviewed, true), false);
    assert.equal(usesUnreviewedBrowserNavigation(
        "// harmless navigation comment\nconst draft = 'text';", false),
        false);
    for (const candidate of [
        "window.location.assign(destination)",
        "location.assign(destination)",
        "document.location.replace(destination)",
        "window['location'].assign(destination)",
        "document['location']['href'] = destination",
        "globalThis.location.assign(destination)",
        "globalThis['location'].replace(destination)",
        "window.location.replace(destination)",
        "window.location.reload()",
        "window.location.href = destination",
        "window.location['href'] = destination",
        "window.location.hash = destination",
        "window.location = destination",
        "location = destination",
        "window.history.pushState(null, '', destination)",
        "window.history.replaceState(null, '', destination)",
        "history.go(-1)",
        "history['back']()",
        "window['history'].go(-1)",
        "globalThis.history.pushState(null, '', destination)",
        "form.submit()",
        "form.requestSubmit()",
        "form['submit']()",
        "anchor.click()",
        "anchor['click']()",
        "new Image()",
        "new Audio()",
        "const navigatorTarget = window.location;",
        ...CAPTURED_NAVIGATION_PROBES,
        'navigation.navigate("https://example.invalid/private");',
        'window.navigation.reload();',
        'globalThis["navigation"].navigate(target);',
        'const { navigation: nav } = window;',
        "const nav = navigation;",
    ]) {
        assert.equal(usesUnreviewedBrowserNavigation(
            reviewed + candidate, true), true, candidate);
        assert.equal(usesUnreviewedBrowserNavigation(candidate, false),
            true, candidate);
    }
});

test("browser source and emitted modules reject unreviewed navigation",
    async () => {
    for (const { filename, source } of await browserModuleSources()) {
        const main = filename === "main.ts" || filename === "main.js";
        assert.equal(usesUnreviewedBrowserNavigation(source, main,
            filename === "paper-profile.ts" || filename === "paper-profile.js"), false,
            `${filename} gained an unreviewed navigation or form request`);
        for (const candidate of CAPTURED_NAVIGATION_PROBES) {
            assert.equal(usesUnreviewedBrowserNavigation(
                `${source}\n${candidate}`, main,
                filename === "paper-profile.ts" || filename === "paper-profile.js"), true,
            `${filename}: ${candidate} escaped navigation review`);
        }
    }
});

// Runtime-created elements, injected markup and CSSOM rules can issue
// requests without appearing in the static HTML/CSS or four fetch sites.
// Keep current accessibility attributes and numeric layout properties
// explicitly reviewed; new DOM sinks require a source-owner decision.
function hasUnreviewedDynamicResourceAuthority(source, main, paper = false) {
    let remaining = source;
    for (const [site, count] of [
        ["function setAttributeIfChanged(", 1],
        ["element.setAttribute(name, value)", 1],
        ['divider.setAttribute("data-pointer-drag", "")', 1],
        ['rootStyle.setProperty("--editor-track", `${share}fr`)', 1],
        ['rootStyle.setProperty("--preview-track", `${previewShare}fr`)', 1],
        [
            'rootStyle.setProperty("--preview-zoom", ' +
                'String(previewZoom / 100))', 1,
        ],
        ['setAttributeIfChanged(divider, "aria-valuenow", String(share))', 1],
        ['setAttributeIfChanged(divider, "aria-valuetext", dividerValue)', 1],
        ['setAttributeIfChanged(divider, "aria-valuemin", "50")', 1],
        ['setAttributeIfChanged(divider, "aria-valuemax", "50")', 1],
        ['setAttributeIfChanged(divider, "aria-disabled", "true")', 1],
        ['setAttributeIfChanged(divider, "tabindex", "-1")', 1],
        ['setAttributeIfChanged(divider, "aria-valuemin", "35")', 1],
        ['setAttributeIfChanged(divider, "aria-valuemax", "65")', 1],
        ['setAttributeIfChanged(divider, "tabindex", "0")', 1],
    ]) {
        if (remaining.split(site).length - 1 !== (main ? count : 0)) {
            return true;
        }
        remaining = remaining.replaceAll(site, "");
    }
    // The download anchor is the only admitted paper-generated element.
    for (const site of [
        'document.createElement("a")',
        "link.href = url",
    ]) {
        if (remaining.split(site).length - 1 !== (paper ? 1 : 0)) {
            return true;
        }
        remaining = remaining.replace(site, "");
    }
    return [
        new RegExp(
            String.raw`\b(?:createElement|createElementNS|` +
                String.raw`createHTMLDocument|DOMParser|` +
                String.raw`createContextualFragment|insertAdjacentHTML|` +
                String.raw`innerHTML|outerHTML)\b`, "u",
        ),
        new RegExp(
            String.raw`\b(?:CSSStyleSheet|insertRule|addRule|` +
                String.raw`replaceSync|adoptedStyleSheets|cssText)\b`, "u",
        ),
        /\b(?:setAttribute|setAttributeIfChanged|setProperty)\b/u,
        /\bdocument\s*\.\s*(?:write|writeln)\s*\(/u,
        new RegExp(
            String.raw`\.\s*(?:src|srcset|href|poster|action|` +
                String.raw`formAction|background|backgroundImage)` +
                String.raw`\s*=(?!=)`, "u",
        ),
    ].some((pattern) => pattern.test(remaining));
}

test("browser dynamic markup and CSSOM request sinks require review",
    async () => {
    for (const { filename, source } of await browserModuleSources()) {
        const main = filename === "main.ts" || filename === "main.js";
        assert.equal(hasUnreviewedDynamicResourceAuthority(source, main,
            filename === "paper-profile.ts" || filename === "paper-profile.js"),
            false, `${filename} gained an unreviewed DOM/CSSOM sink`);
        for (const candidate of [
            'document.createElement("img")',
            'document.createElementNS("svg", "image")',
            'document.implementation.createHTMLDocument("foreign")',
            'element.innerHTML = draftText',
            'element.insertAdjacentHTML("beforeend", draftText)',
            'new DOMParser()',
            'sheet.insertRule("@import url(/unreviewed.css)")',
            'sheet.replaceSync("@import url(/unreviewed.css)")',
            'new CSSStyleSheet()',
            'image.setAttribute("src", destination)',
            'setAttributeIfChanged(divider, "src", destination)',
            'rootStyle.setProperty("background-image", destination)',
            'image.src = destination',
            'document.write(draftText)',
        ]) {
            assert.equal(hasUnreviewedDynamicResourceAuthority(
                `${source}\n${candidate}`, main,
                filename === "paper-profile.ts" || filename === "paper-profile.js"), true,
            `${filename}: ${candidate} escaped dynamic-resource review`);
        }
    }
});

// Logging, browser dialogs, notifications and OS sharing are separate
// private-data output surfaces from network requests and clipboard copying.
// Only the reviewed Copy prompt clipboard.writeText handoff is admitted.
function hasUnreviewedBrowserDisclosure(source) {
    const executable = source.replace(/^\s*\/\/[^\n]*$/gmu, "");
    const navigatorShare = new RegExp(
        String.raw`\bnavigator\s*(?:\?\.|\.)\s*(?:share|canShare)\b|` +
            String.raw`\bnavigator\s*(?:\?\.)?\[\s*` +
            String.raw`['"](?:share|canShare)['"]\s*\]`,
        "u",
    );
    return [
        new RegExp(
            String.raw`\b(?:console|debugger|reportError|` +
                String.raw`Notification|alert|confirm|print|speechSynthesis|` +
                String.raw`SpeechSynthesisUtterance|AudioContext|` +
                String.raw`webkitAudioContext|AudioWorkletNode|` +
                String.raw`MediaMetadata|MediaSession|mediaSession|` +
                String.raw`setAppBadge|clearAppBadge|Presentation|` +
                String.raw`PresentationRequest|PresentationConnection|` +
                String.raw`PresentationReceiver|presentation|PaymentRequest|` +
                String.raw`PaymentResponse|PaymentAddress|` +
                String.raw`PaymentMethodChangeEvent|documentPictureInPicture|` +
                String.raw`DocumentPictureInPicture|PictureInPictureWindow|` +
                String.raw`PictureInPictureEvent|requestPictureInPicture|` +
                String.raw`webkitSetPresentationMode)\b`, "u",
        ),
        /\bprompt\s*\(/u,
        /\b(?:window|globalThis)\s*(?:\?\.|\.)\s*prompt\b/u,
        /\b(?:window|globalThis)\s*(?:\?\.)?\[\s*['"]prompt['"]\s*\]/u,
        navigatorShare,
        /\bclipboard\s*(?:\?\.|\.)\s*write\b/u,
        /\bclipboard\s*(?:\?\.)?\[\s*['"]write['"]\s*\]/u,
    ].some((pattern) => pattern.test(executable));
}

test("browser private-output sinks require explicit review", async () => {
    for (const { filename, source } of await browserModuleSources()) {
        assert.equal(hasUnreviewedBrowserDisclosure(source), false,
            `${filename} gained an unreviewed private-output sink`);
        for (const candidate of [
            "void console;",
            'void globalThis["console"];',
            "debugger;",
            "void reportError;",
            "void Notification;",
            "alert('dialog');",
            "confirm('dialog');",
            "window['print']();",
            "prompt('dialog');",
            'globalThis["prompt"]("dialog");',
            "speechSynthesis.speak(new SpeechSynthesisUtterance(text));",
            'const speaker = globalThis["speechSynthesis"];',
            "new AudioContext();",
            "new webkitAudioContext();",
            "new AudioWorkletNode(context, 'processor');",
            "new MediaMetadata({ title: privateDraft });",
            "void window.MediaMetadata;",
            "void MediaSession;",
            "navigator.mediaSession.metadata = metadata;",
            'const session = navigator["mediaSession"];',
            "navigator.setAppBadge(privateCount);",
            "navigator.clearAppBadge();",
            'const badge = navigator["setAppBadge"];',
            'navigator.share({ text: "example" });',
            'void navigator["share"];',
            "clipboard.write([]);",
            'clipboard?.["write"]([]);',
        ]) {
            assert.equal(hasUnreviewedBrowserDisclosure(
                `${source}\n${candidate}`), true,
            `${filename}: ${candidate} escaped private-output review`);
        }
    }
});

// Picture-in-Picture can present content in a separate OS-managed window
// without the admitted Copy prompt flow. Its methods and first-class handles
// need explicit source review even when no video element exists in the shell.
test("browser picture-in-picture surfaces require review", async () => {
    const probes = [
        "window.documentPictureInPicture.requestWindow()",
        'const pip = globalThis["documentPictureInPicture"];',
        "const pip = window?.documentPictureInPicture;",
        "video.requestPictureInPicture()",
        'const start = video["requestPictureInPicture"];',
        "void DocumentPictureInPicture;",
        "void PictureInPictureWindow;",
        "void PictureInPictureEvent;",
        'video.webkitSetPresentationMode("picture-in-picture")',
    ];
    for (const { filename, source } of await browserModuleSources()) {
        assert.equal(hasUnreviewedBrowserDisclosure(source), false,
            `${filename} gained an unreviewed OS display output`);
        for (const candidate of probes) {
            assert.equal(hasUnreviewedBrowserDisclosure(
                `${source}\n${candidate}`), true,
            `${filename}: ${candidate} escaped picture-in-picture review`);
        }
    }
});

// Browser Presentation can send document-selected data to another display.
// Payment Request can invoke the host's payment UI outside the notebook.
// These are distinct from the reviewed Copy prompt output action.
test("browser remote presentation and payment APIs require review",
    async () => {
    const probes = [
        'new PresentationRequest(["https://example.invalid"]);',
        'navigator.presentation.defaultRequest = request;',
        'const p = navigator["presentation"];',
        "void Presentation;",
        "void PresentationConnection;",
        "void PresentationReceiver;",
        'const request = globalThis["PresentationRequest"];',
        "new PaymentRequest(methods, details);",
        'const request = window["PaymentRequest"];',
        "void PaymentResponse;",
        "void PaymentAddress;",
        "void PaymentMethodChangeEvent;",
    ];
    for (const { filename, source } of await browserModuleSources()) {
        assert.equal(hasUnreviewedBrowserDisclosure(source), false,
            `${filename} gained an unreviewed presentation or payment API`);
        for (const candidate of probes) {
            assert.equal(hasUnreviewedBrowserDisclosure(
                `${source}\n${candidate}`), true,
            `${filename}: ${candidate} escaped host-output review`);
        }
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

    // Execute the emitted callback in a bounded VM as well as checking its
    // textual order. A persisted page must not run ordinary restore effects.
    const effects = [];
    let onPageShow;
    runInNewContext(source.slice(start, end + "\n});".length), {
        window: {
            addEventListener(name, callback) {
                assert.equal(name, "pageshow");
                onPageShow = callback;
            },
            location: {
                reload() { effects.push("reload"); },
            },
            setTimeout(callback, delay) {
                assert.equal(delay, 0);
                assert.equal(typeof callback, "function");
                effects.push("schedule-reset");
            },
        },
        discardLocalNavigationFragment() {
            effects.push("discard-fragment");
            return true;
        },
        resetLocalViewportState() {
            effects.push("reset-viewport");
        },
    });
    assert.equal(typeof onPageShow, "function");
    onPageShow({ persisted: true });
    assert.deepEqual(effects, ["reload"]);
    effects.length = 0;
    onPageShow({ persisted: false });
    assert.deepEqual(effects, [
        "discard-fragment", "reset-viewport", "schedule-reset",
    ]);
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
