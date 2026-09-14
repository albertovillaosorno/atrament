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
//   - Real-browser evidence for workspace presentation and layout, localhost
//     frame isolation, and browser-session disposal.
// - Must-Not:
//   - Read live session-private state, persist browser data, or weaken runtime
//     admission to make automation easier.
// - Allows:
//   - Inputs: The generated workspace, one freshly started Atrament runtime,
//     and deterministic loopback fixtures.
//   - Outputs: Assertions over Firefox frame policy and session disposal.
//   - Side effects: Starts disposable runtime/browser processes and loopback
//     HTTP servers, then removes their temporary browser profiles.
// - Split-When:
//   - More browser-enforced response policies need independent fixtures.
// - Merge-When:
//   - Browser security integration moves to one shared automation harness.
// - Summary:
//   - Proves workspace presentation, frame isolation, and page-session
//     disposal.
// - Description:
//   - Uses Firefox's built-in WebDriver BiDi endpoint through a dependency-free
//     RFC 6455 client. It checks responsive/scriptless layout, inert hostile
//     text, framing, fragment-scrubbed refresh, and close-time cancellation.
// - Usage:
//   - Run through `pnpm test:browser-security` from the repository root.
// - Defaults:
//   - Skips only when Firefox is unavailable on the host.
//
import assert from "node:assert/strict";
import crypto from "node:crypto";
import { spawn, spawnSync } from "node:child_process";
import { once } from "node:events";
import fs from "node:fs";
import http from "node:http";
import net from "node:net";
import path from "node:path";
import test from "node:test";

const FIREFOX_AVAILABLE = process.platform === "linux"
    && spawnSync(
        "firefox",
        ["--version"],
        { stdio: "ignore" },
    ).status === 0;
const TARGET_DIR = path.resolve(".cache/cargo-target");
const RUNTIME_BINARY = path.join(TARGET_DIR, "debug", "atrament");
const WORKSPACE_DIR = path.resolve(
    "src/browser/workspace/adapter-inbound",
);
const REFRESH_SECRET = "a".repeat(64);
const REFRESH_DRAFT = {
    candidate: "refresh-private candidate",
    source: "refresh-private source",
    task: "refresh-private task",
};
const COMPATIBLE_HANDSHAKE = JSON.stringify({
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

function waitForLine(stream, select, timeoutMs = 15_000) {
    return new Promise((resolve, reject) => {
        let text = "";
        const timer = setTimeout(() => {
            reject(new Error(`timed out waiting for process output: ${text}`));
        }, timeoutMs);
        stream.setEncoding("utf8");
        stream.on("data", (chunk) => {
            text += chunk;
            const lines = text.split("\n");
            text = lines.pop() ?? "";
            for (const line of lines) {
                const value = select(line);
                if (value !== null) {
                    clearTimeout(timer);
                    resolve(value);
                    return;
                }
            }
        });
    });
}

async function stopChild(child) {
    if (child.exitCode !== null || child.signalCode !== null) {
        return;
    }
    child.kill("SIGTERM");
    const exited = await Promise.race([
        once(child, "exit").then(() => true),
        new Promise((resolve) => setTimeout(() => resolve(false), 2_000)),
    ]);
    if (!exited) {
        child.kill("SIGKILL");
        await once(child, "exit");
    }
}

class BidiClient {
    constructor(url) {
        this.buffer = Buffer.alloc(0);
        this.events = [];
        this.nextId = 1;
        this.pending = new Map();
        this.socket = null;
        this.url = new URL(url);
    }

    async connect() {
        this.socket = net.createConnection({
            host: this.url.hostname,
            port: Number(this.url.port),
        });
        await once(this.socket, "connect");
        const key = crypto.randomBytes(16).toString("base64");
        this.socket.write([
            `GET ${this.url.pathname} HTTP/1.1`,
            `Host: ${this.url.host}`,
            "Upgrade: websocket",
            "Connection: Upgrade",
            `Sec-WebSocket-Key: ${key}`,
            "Sec-WebSocket-Version: 13",
            "",
            "",
        ].join("\r\n"));
        let handshake = Buffer.alloc(0);
        while (handshake.indexOf("\r\n\r\n") < 0) {
            const [chunk] = await once(this.socket, "data");
            handshake = Buffer.concat([handshake, chunk]);
        }
        const boundary = handshake.indexOf("\r\n\r\n");
        const head = handshake.subarray(0, boundary).toString("latin1");
        assert.ok(head.startsWith("HTTP/1.1 101"), head);
        this.buffer = handshake.subarray(boundary + 4);
        this.socket.on("data", (chunk) => {
            this.buffer = Buffer.concat([this.buffer, chunk]);
            this.drain();
        });
        this.drain();
    }

    command(method, params = {}, timeoutMs = 10_000) {
        const id = this.nextId;
        this.nextId += 1;
        return new Promise((resolve, reject) => {
            const timer = setTimeout(() => {
                if (this.pending.delete(id)) {
                    reject(new Error(`${method} timed out`));
                }
            }, timeoutMs);
            this.pending.set(id, {
                reject,
                resolve: (result) => {
                    clearTimeout(timer);
                    resolve(result);
                },
            });
            this.socket.write(this.textFrame(JSON.stringify({
                id,
                method,
                params,
            })));
        });
    }

    drain() {
        while (this.buffer.length >= 2) {
            const opcode = this.buffer[0] & 0x0f;
            let length = this.buffer[1] & 0x7f;
            let offset = 2;
            if (length === 126) {
                if (this.buffer.length < 4) {
                    return;
                }
                length = this.buffer.readUInt16BE(2);
                offset = 4;
            } else if (length === 127) {
                if (this.buffer.length < 10) {
                    return;
                }
                length = Number(this.buffer.readBigUInt64BE(2));
                offset = 10;
            }
            if (this.buffer.length < offset + length) {
                return;
            }
            const payload = this.buffer.subarray(offset, offset + length);
            this.buffer = this.buffer.subarray(offset + length);
            if (opcode !== 0x1) {
                continue;
            }
            const message = JSON.parse(payload.toString("utf8"));
            if (message.type === "success" || message.type === "error") {
                const pending = this.pending.get(message.id);
                if (pending === undefined) {
                    continue;
                }
                this.pending.delete(message.id);
                if (message.type === "success") {
                    pending.resolve(message.result);
                } else {
                    pending.reject(new Error(JSON.stringify(message)));
                }
            } else {
                this.events.push(message);
            }
        }
    }

    textFrame(text) {
        const payload = Buffer.from(text);
        const mask = crypto.randomBytes(4);
        let header;
        if (payload.length < 126) {
            header = Buffer.from([0x81, 0x80 | payload.length]);
        } else {
            assert.ok(payload.length < 65_536);
            header = Buffer.from([
                0x81,
                0xfe,
                payload.length >> 8,
                payload.length & 0xff,
            ]);
        }
        const masked = Buffer.alloc(payload.length);
        for (let index = 0; index < payload.length; index += 1) {
            masked[index] = payload[index] ^ mask[index % 4];
        }
        return Buffer.concat([header, mask, masked]);
    }
}

async function evaluateDocument(client, context) {
    const response = await client.command("script.evaluate", {
        awaitPromise: false,
        expression: `JSON.stringify({
            hasControl: document.querySelector(".control-frame") !== null,
            hasWorkspace: document.querySelector(".workspace-shell") !== null,
            title: document.title
        })`,
        resultOwnership: "none",
        target: { context },
    });
    assert.equal(response.type, "success");
    assert.equal(response.result.type, "string");
    return JSON.parse(response.result.value);
}

async function waitForChildContext(client, root) {
    for (let attempt = 0; attempt < 100; attempt += 1) {
        const tree = await client.command("browsingContext.getTree", {
            maxDepth: 2,
            root,
        });
        const child = tree.contexts[0]?.children?.[0]?.context;
        if (child !== undefined) {
            return child;
        }
        await new Promise((resolve) => setTimeout(resolve, 25));
    }
    throw new Error("hostile iframe never created a browsing context");
}

async function waitForFramedDocument(client, context) {
    await new Promise((resolve) => setTimeout(resolve, 1_000));
    let documentState = null;
    for (let attempt = 0; attempt < 20; attempt += 1) {
        documentState = await evaluateDocument(client, context);
        if (documentState.hasWorkspace || documentState.title !== "") {
            return documentState;
        }
        await new Promise((resolve) => setTimeout(resolve, 250));
    }
    throw new Error(
        `framed navigation did not settle: ${JSON.stringify(documentState)}`,
    );
}

async function navigateFromDocument(client, context, url) {
    await client.command("script.evaluate", {
        awaitPromise: false,
        expression: `location.assign(${JSON.stringify(url)})`,
        resultOwnership: "none",
        target: { context },
    });
}

async function observedWithin(observation, label) {
    return Promise.race([
        observation,
        new Promise((_resolve, reject) => setTimeout(() => {
            reject(new Error(`${label} request was not observed`));
        }, 5_000)),
    ]);
}

async function startBidiFirefox(profile, children) {
    const firefox = spawn(
        "firefox",
        [
            "--headless",
            "--no-remote",
            "--profile",
            profile,
            "--remote-debugging-port",
            "0",
            "about:blank",
        ],
        { stdio: ["ignore", "ignore", "pipe"] },
    );
    children.push(firefox);
    const remote = await waitForLine(firefox.stderr, (line) => {
        const prefix = "WebDriver BiDi listening on ";
        return line.startsWith(prefix)
            ? line.slice(prefix.length)
            : null;
    });
    const bidi = new BidiClient(`${remote}/session`);
    await bidi.connect();
    await bidi.command(
        "session.new",
        { capabilities: { alwaysMatch: {} } },
        15_000,
    );
    return bidi;
}

function serveFile(response, file, contentType) {
    response.writeHead(200, {
        "Cache-Control": "no-store",
        "Content-Type": contentType,
    });
    response.end(fs.readFileSync(file));
}

function createStaticWorkspaceServer(requests, serveScripts = false) {
    const generated = new Set([
        "main.js",
        "session-diagnostic.js",
        "session-draft.js",
        "session-fragment.js",
        "session-handshake.js",
    ]);
    return http.createServer((request, response) => {
        const pathname = new URL(
            request.url ?? "/",
            "http://127.0.0.1",
        ).pathname;
        requests.push(pathname);
        if (pathname === "/" || pathname === "/index.html") {
            serveFile(
                response,
                path.join(WORKSPACE_DIR, "index.html"),
                "text/html; charset=utf-8",
            );
            return;
        }
        if (pathname === "/workspace.css") {
            serveFile(
                response,
                path.join(WORKSPACE_DIR, "workspace.css"),
                "text/css; charset=utf-8",
            );
            return;
        }
        if (serveScripts && pathname.startsWith("/generated/")) {
            const file = pathname.slice("/generated/".length);
            if (generated.has(file)) {
                serveFile(
                    response,
                    path.join(WORKSPACE_DIR, "generated", file),
                    "text/javascript; charset=utf-8",
                );
                return;
            }
        }
        response.writeHead(404);
        response.end();
    });
}

function createSessionFixtureServer(apiRequests, onDraftMutation = null) {
    const generated = new Set([
        "main.js",
        "session-diagnostic.js",
        "session-draft.js",
        "session-fragment.js",
        "session-handshake.js",
    ]);
    return http.createServer((request, response) => {
        const pathname = new URL(
            request.url ?? "/",
            "http://127.0.0.1",
        ).pathname;
        if (pathname === "/" || pathname === "/index.html") {
            serveFile(
                response,
                path.join(WORKSPACE_DIR, "index.html"),
                "text/html; charset=utf-8",
            );
            return;
        }
        if (pathname === "/workspace.css") {
            serveFile(
                response,
                path.join(WORKSPACE_DIR, "workspace.css"),
                "text/css; charset=utf-8",
            );
            return;
        }
        if (pathname.startsWith("/generated/")) {
            const file = pathname.slice("/generated/".length);
            if (generated.has(file)) {
                serveFile(
                    response,
                    path.join(WORKSPACE_DIR, "generated", file),
                    "text/javascript; charset=utf-8",
                );
                return;
            }
        }
        if (pathname === "/api/handshake") {
            apiRequests.push(`${request.method} ${pathname}`);
            if (
                request.method !== "POST"
                || request.headers.authorization
                    !== `Bearer ${REFRESH_SECRET}`
            ) {
                response.writeHead(401);
                response.end();
                return;
            }
            response.writeHead(200, {
                "Cache-Control": "no-store",
                "Content-Type": "application/json; charset=utf-8",
            });
            response.end(COMPATIBLE_HANDSHAKE);
            return;
        }
        if (pathname.startsWith("/api/session/")) {
            apiRequests.push(`${request.method} ${pathname}`);
            const field = pathname.slice("/api/session/".length);
            if (
                request.headers.authorization !== `Bearer ${REFRESH_SECRET}`
                || !(field in REFRESH_DRAFT)
            ) {
                response.writeHead(401);
                response.end();
                return;
            }
            if (request.method === "POST" && onDraftMutation !== null) {
                onDraftMutation(request, response, field);
                return;
            }
            if (request.method !== "GET") {
                response.writeHead(405);
                response.end();
                return;
            }
            response.writeHead(200, {
                "Cache-Control": "no-store",
                "Content-Type": "text/plain; charset=utf-8",
            });
            response.end(REFRESH_DRAFT[field]);
            return;
        }
        response.writeHead(404);
        response.end();
    });
}

async function evaluateSessionDocument(client, context) {
    const response = await client.command("script.evaluate", {
        awaitPromise: false,
        expression: `JSON.stringify({
            hash: window.location.hash,
            status: document.querySelector("#session-status")?.textContent,
            task: {
                disabled: document.querySelector("#task-input")?.disabled,
                value: document.querySelector("#task-input")?.value
            },
            source: {
                disabled: document.querySelector("#source-input")?.disabled,
                value: document.querySelector("#source-input")?.value
            },
            candidate: {
                disabled: document.querySelector("#candidate-input")?.disabled,
                value: document.querySelector("#candidate-input")?.value
            }
        })`,
        resultOwnership: "none",
        target: { context },
    });
    assert.equal(response.type, "success");
    assert.equal(response.result.type, "string");
    return JSON.parse(response.result.value);
}

async function waitForSessionStatus(client, context, expected) {
    let state = null;
    for (let attempt = 0; attempt < 100; attempt += 1) {
        state = await evaluateSessionDocument(client, context);
        if (state.status === expected) {
            return state;
        }
        await new Promise((resolve) => setTimeout(resolve, 25));
    }
    throw new Error(
        `session status did not reach ${expected}: ${JSON.stringify(state)}`,
    );
}

async function performTrustedKey(client, context, value) {
    await client.command("input.performActions", {
        actions: [
            {
                actions: [
                    { type: "keyDown", value },
                    { type: "keyUp", value },
                ],
                id: "keyboard",
                type: "key",
            },
        ],
        context,
    });
}

async function performTrustedKeyChord(
    client,
    context,
    modifier,
    value,
) {
    await client.command("input.performActions", {
        actions: [
            {
                actions: [
                    { type: "keyDown", value: modifier },
                    { type: "keyDown", value },
                    { type: "keyUp", value },
                    { type: "keyUp", value: modifier },
                ],
                id: "keyboard",
                type: "key",
            },
        ],
        context,
    });
}

async function performTrustedPointer(client, context, actions) {
    await client.command("input.performActions", {
        actions: [
            {
                actions,
                id: "mouse",
                parameters: { pointerType: "mouse" },
                type: "pointer",
            },
        ],
        context,
    });
}

async function activeElementIdentity(client, context) {
    const response = await client.command("script.evaluate", {
        awaitPromise: false,
        expression: `JSON.stringify({
            href: document.activeElement?.getAttribute("href") ?? null,
            id: document.activeElement?.id ?? "",
            tag: document.activeElement?.tagName ?? ""
        })`,
        resultOwnership: "none",
        target: { context },
    });
    assert.equal(response.type, "success");
    assert.equal(response.result.type, "string");
    return JSON.parse(response.result.value);
}

async function setFirefoxViewport(client, context, width, height) {
    await client.command("browsingContext.setViewport", {
        context,
        devicePixelRatio: 1,
        viewport: { height, width },
    });
    for (let attempt = 0; attempt < 40; attempt += 1) {
        const response = await client.command("script.evaluate", {
            awaitPromise: false,
            expression: [
                "JSON.stringify([window.innerWidth,",
                "window.innerHeight])",
            ].join(" "),
            resultOwnership: "none",
            target: { context },
        });
        assert.equal(response.type, "success");
        assert.equal(response.result.type, "string");
        const observed = JSON.parse(response.result.value);
        if (observed[0] === width && observed[1] === height) {
            return;
        }
        await new Promise((resolve) => setTimeout(resolve, 25));
    }
    throw new Error(`Firefox viewport did not settle at ${width}x${height}`);
}

async function configureWorkspaceLayout(client, context, key, zoom) {
    const response = await client.command("script.evaluate", {
        awaitPromise: false,
        expression: `(() => {
            const divider = document.querySelector("#workspace-divider");
            divider.dispatchEvent(new KeyboardEvent("keydown", {
                bubbles: true,
                key: ${JSON.stringify(key)}
            }));
            const reset = document.querySelector("#zoom-reset");
            const zoomOut = document.querySelector("#zoom-out");
            const zoomIn = document.querySelector("#zoom-in");
            reset.click();
            const control = ${zoom} < 100 ? zoomOut : zoomIn;
            const steps = Math.abs(${zoom} - 100) / 10;
            for (let step = 0; step < steps; step += 1) {
                control.click();
            }
            return true;
        })()`,
        resultOwnership: "none",
        target: { context },
    });
    assert.equal(response.type, "success");
}

async function evaluateWorkspaceLayout(client, context) {
    const response = await client.command("script.evaluate", {
        awaitPromise: false,
        expression: `JSON.stringify((() => {
            const rect = (selector) => {
                const element = document.querySelector(selector);
                const box = element.getBoundingClientRect();
                return {
                    bottom: box.bottom,
                    height: box.height,
                    left: box.left,
                    right: box.right,
                    top: box.top,
                    width: box.width
                };
            };
            const divider = document.querySelector("#workspace-divider");
            const sourcePanel = document.querySelector("#source-panel");
            const stage = document.querySelector("#page-stage");
            const paper = document.querySelector(".paper-preview");
            const sourceBox = sourcePanel.getBoundingClientRect();
            const taskBox = rect("#task-input");
            sourcePanel.scrollTop = sourcePanel.scrollHeight;
            const copyBox = document
                .querySelector(".copy-toolbar")
                .getBoundingClientRect();
            const copyStickyVisible =
                copyBox.top >= sourceBox.top - 1
                && copyBox.bottom <= sourceBox.bottom + 1;
            const sourceScrolled = sourcePanel.scrollTop > 0;
            sourcePanel.scrollTop = 0;
            const stageBox = stage.getBoundingClientRect();
            stage.scrollLeft = 0;
            stage.scrollTop = 0;
            const paperAtStart = paper.getBoundingClientRect();
            const startReachable = {
                left: paperAtStart.left >= stageBox.left - 1,
                top: paperAtStart.top >= stageBox.top - 1
            };
            stage.scrollLeft = stage.scrollWidth;
            stage.scrollTop = stage.scrollHeight;
            const paperAtEnd = paper.getBoundingClientRect();
            const endReachable = {
                bottom: paperAtEnd.bottom <= stageBox.bottom + 1,
                right: paperAtEnd.right <= stageBox.right + 1
            };
            stage.scrollLeft = 0;
            stage.scrollTop = 0;
            return {
                copyStickyVisible,
                divider: {
                    disabled: divider.getAttribute("aria-disabled"),
                    maximum: divider.getAttribute("aria-valuemax"),
                    minimum: divider.getAttribute("aria-valuemin"),
                    now: divider.getAttribute("aria-valuenow"),
                    tabindex: divider.getAttribute("tabindex")
                },
                document: {
                    scrollHeight: document.documentElement.scrollHeight,
                    scrollWidth: document.documentElement.scrollWidth
                },
                endReachable,
                paper: rect(".paper-preview"),
                preview: rect("#preview-panel"),
                source: rect("#source-panel"),
                sourceScrolled,
                stage: rect("#page-stage"),
                stageSize: {
                    clientHeight: stage.clientHeight,
                    clientWidth: stage.clientWidth,
                    scrollHeight: stage.scrollHeight,
                    scrollWidth: stage.scrollWidth
                },
                startReachable,
                task: taskBox,
                viewport: {
                    height: window.innerHeight,
                    width: window.innerWidth
                },
                workspace: rect(".workspace-grid"),
                zoom: document
                    .querySelector("#preview-scale")
                    .textContent.trim()
            };
        })())`,
        resultOwnership: "none",
        target: { context },
    });
    assert.equal(response.type, "success");
    assert.equal(response.result.type, "string");
    return JSON.parse(response.result.value);
}

function assertVisibleInViewport(rectangle, viewportHeight, label) {
    assert.ok(rectangle.width > 0, `${label} has positive width`);
    assert.ok(rectangle.height > 0, `${label} has positive height`);
    assert.ok(rectangle.bottom > 0, `${label} reaches below viewport top`);
    assert.ok(rectangle.top < viewportHeight, `${label} reaches viewport`);
}

test(
    "Firefox without JavaScript keeps both static workspace surfaces visible",
    { skip: !FIREFOX_AVAILABLE, timeout: 30_000 },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/workspace-static-");
        fs.writeFileSync(
            path.join(profile, "user.js"),
            'user_pref("javascript.enabled", false);\n',
        );
        const requests = [];
        const server = createStaticWorkspaceServer(requests);
        const children = [];
        let bidi;
        try {
            server.listen(0, "127.0.0.1");
            await once(server, "listening");
            const address = server.address();
            assert.notEqual(typeof address, "string");
            assert.notEqual(address, null);
            const origin = `http://127.0.0.1:${address.port}`;

            bidi = await startBidiFirefox(profile, children);
            const tab = await bidi.command("browsingContext.create", {
                type: "tab",
            });
            await bidi.command("browsingContext.navigate", {
                context: tab.context,
                url: origin,
                wait: "complete",
            });

            for (const width of [320, 481]) {
                await setFirefoxViewport(
                    bidi,
                    tab.context,
                    width,
                    225,
                );
                const response = await bidi.command("script.evaluate", {
                    awaitPromise: false,
                    expression: `JSON.stringify((() => {
                        const rect = (selector) => {
                            const element = document.querySelector(selector);
                            const box = element.getBoundingClientRect();
                            return {
                                bottom: box.bottom,
                                height: box.height,
                                top: box.top,
                                width: box.width
                            };
                        };
                        const divider = document.querySelector(
                            "#workspace-divider"
                        );
                        return {
                            divider: {
                                disabled: divider.getAttribute("aria-disabled"),
                                maximum: divider.getAttribute("aria-valuemax"),
                                minimum: divider.getAttribute("aria-valuemin"),
                                now: divider.getAttribute("aria-valuenow"),
                                tabindex: divider.getAttribute("tabindex")
                            },
                            document: {
                                height: document.documentElement.scrollHeight,
                                width: document.documentElement.scrollWidth
                            },
                            preview: rect("#preview-panel"),
                            source: rect("#source-panel"),
                            stage: rect("#page-stage"),
                            task: rect("#task-input"),
                            viewport: {
                                height: window.innerHeight,
                                width: window.innerWidth
                            },
                            warning: rect(".script-warning")
                        };
                    })())`,
                    resultOwnership: "none",
                    target: { context: tab.context },
                });
                assert.equal(response.type, "success");
                assert.equal(response.result.type, "string");
                const state = JSON.parse(response.result.value);
                const label = `${width}x225 static shell`;
                assert.deepEqual(
                    state.viewport,
                    { height: 225, width },
                    `${label}: exact viewport`,
                );
                assert.ok(
                    state.document.width <= width,
                    `${label}: no horizontal document overflow`,
                );
                assert.ok(
                    state.document.height <= 225,
                    `${label}: no vertical document overflow`,
                );
                assert.deepEqual(
                    state.divider,
                    {
                        disabled: "true",
                        maximum: "50",
                        minimum: "50",
                        now: "50",
                        tabindex: "-1",
                    },
                    `${label}: divider remains inert 50/50`,
                );
                for (const [name, rectangle] of [
                    ["source panel", state.source],
                    ["preview panel", state.preview],
                    ["task field", state.task],
                    ["page stage", state.stage],
                    ["script warning", state.warning],
                ]) {
                    assertVisibleInViewport(
                        rectangle,
                        225,
                        `${label}: ${name}`,
                    );
                }
            }
            assert.deepEqual(requests, ["/", "/workspace.css"]);
        } finally {
            if (bidi?.socket != null) {
                bidi.socket.end();
            }
            if (server.listening) {
                server.close();
                await once(server, "close");
            }
            for (const child of children.reverse()) {
                await stopChild(child);
            }
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox keeps hostile prompt and response text inert",
    { skip: !FIREFOX_AVAILABLE, timeout: 30_000 },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/workspace-hostile-text-");
        const requests = [];
        const server = createStaticWorkspaceServer(requests, true);
        const children = [];
        let bidi;
        try {
            server.listen(0, "127.0.0.1");
            await once(server, "listening");
            const address = server.address();
            assert.notEqual(typeof address, "string");
            assert.notEqual(address, null);
            const origin = `http://127.0.0.1:${address.port}`;

            bidi = await startBidiFirefox(profile, children);
            const tab = await bidi.command("browsingContext.create", {
                type: "tab",
            });
            await bidi.command("browsingContext.navigate", {
                context: tab.context,
                url: origin,
                wait: "complete",
            });

            const payload = [
                '<img id="evil-image" src="/evil">',
                '<script>globalThis.__hostileExecuted = true;</script>',
                'fetch("/evil")',
                '{"command":"export","path":"/tmp/private"}',
                "start hardware now · á n\\u{301} 👩‍🔬",
            ].join("\n");
            const response = await bidi.command("script.evaluate", {
                awaitPromise: true,
                expression: `(async () => {
                    const payload = ${JSON.stringify(payload)};
                    globalThis.__hostileExecuted = false;
                    globalThis.__clipboardWrites = [];
                    Object.defineProperty(navigator, "clipboard", {
                        configurable: true,
                        value: {
                            writeText(text) {
                                globalThis.__clipboardWrites.push(text);
                                return Promise.resolve();
                            }
                        }
                    });
                    const prompt = document.querySelector("#prompt-output");
                    const candidate = document.querySelector(
                        "#candidate-input"
                    );
                    prompt.value = payload;
                    prompt.dispatchEvent(new Event("input", { bubbles: true }));
                    candidate.value = payload;
                    candidate.dispatchEvent(
                        new Event("input", { bubbles: true })
                    );
                    document.querySelector("#copy-prompt").click();
                    await new Promise((resolve) => setTimeout(resolve, 0));
                    return JSON.stringify({
                        candidateChildren: candidate.childElementCount,
                        candidateValue: candidate.value,
                        clipboardWrites: globalThis.__clipboardWrites,
                        evilImage:
                            document.querySelector("#evil-image") !== null,
                        executed: globalThis.__hostileExecuted,
                        promptChildren: prompt.childElementCount,
                        promptValue: prompt.value,
                        status: document.querySelector("#copy-status")
                            .textContent.trim()
                    });
                })()`,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.equal(response.type, "success");
            assert.equal(response.result.type, "string");
            const state = JSON.parse(response.result.value);
            assert.equal(state.promptValue, payload);
            assert.equal(state.candidateValue, payload);
            assert.deepEqual(state.clipboardWrites, [payload]);
            assert.equal(state.promptChildren, 0);
            assert.equal(state.candidateChildren, 0);
            assert.equal(state.evilImage, false);
            assert.equal(state.executed, false);
            assert.equal(state.status, "Prompt copied.");
            assert.equal(requests.includes("/evil"), false);
            assert.equal(
                requests.some((request) => request.startsWith("/api/")),
                false,
            );
        } finally {
            if (bidi?.socket != null) {
                bidi.socket.end();
            }
            if (server.listening) {
                server.close();
                await once(server, "close");
            }
            for (const child of children.reverse()) {
                await stopChild(child);
            }
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox preserves large Unicode prompt data and canonical line endings",
    { skip: !FIREFOX_AVAILABLE, timeout: 30_000 },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/workspace-prompt-text-");
        const requests = [];
        const server = createStaticWorkspaceServer(requests, true);
        const children = [];
        let bidi;
        try {
            server.listen(0, "127.0.0.1");
            await once(server, "listening");
            const address = server.address();
            assert.notEqual(typeof address, "string");
            assert.notEqual(address, null);
            const origin = `http://127.0.0.1:${address.port}`;

            bidi = await startBidiFirefox(profile, children);
            const tab = await bidi.command("browsingContext.create", {
                type: "tab",
            });
            await bidi.command("browsingContext.navigate", {
                context: tab.context,
                url: origin,
                wait: "complete",
            });

            const response = await bidi.command("script.evaluate", {
                awaitPromise: true,
                expression: `(async () => {
                    const prompt = document.querySelector("#prompt-output");
                    const candidate = document.querySelector(
                        "#candidate-input"
                    );
                    globalThis.__clipboardWrites = [];
                    Object.defineProperty(navigator, "clipboard", {
                        configurable: true,
                        value: {
                            writeText(text) {
                                globalThis.__clipboardWrites.push(text);
                                return Promise.resolve();
                            }
                        }
                    });
                    const encoder = new TextEncoder();
                    const targetBytes = 1_406_010;
                    const marker = [
                        "semantic-command Apply revision=opaque",
                        "á a\\u{301} 👩‍🔬 ¿listo?",
                        "{\\\"intent\\\":\\\"copy-only\\\"}"
                    ].join(" · ") + "\\n";
                    const markerBytes = encoder.encode(marker).length;
                    const repeats = Math.floor(targetBytes / markerBytes);
                    let large = marker.repeat(repeats);
                    large += "x".repeat(
                        targetBytes - encoder.encode(large).length
                    );
                    prompt.value = large;
                    prompt.dispatchEvent(
                        new Event("input", { bubbles: true })
                    );
                    candidate.value = large;
                    candidate.dispatchEvent(
                        new Event("input", { bubbles: true })
                    );
                    document.querySelector("#copy-prompt").click();
                    await new Promise((resolve) => setTimeout(resolve, 0));
                    const largeState = {
                        bytes: encoder.encode(large).length,
                        candidateExact: candidate.value === large,
                        clipboardExact:
                            globalThis.__clipboardWrites.length === 1
                            && globalThis.__clipboardWrites[0] === large,
                        promptExact: prompt.value === large,
                        responseChildren: candidate.childElementCount,
                        promptChildren: prompt.childElementCount,
                        status: document.querySelector("#copy-status")
                            .textContent.trim()
                    };

                    globalThis.__clipboardWrites = [];
                    const raw = [
                        "first\\r\\nsecond\\rthird",
                        "NFC=á",
                        "NFD=a\\u{301}",
                        "ZWJ=👩‍🔬"
                    ].join("\\n");
                    const canonical = [
                        "first",
                        "second",
                        "third",
                        "NFC=á",
                        "NFD=a\\u{301}",
                        "ZWJ=👩‍🔬"
                    ].join("\\n");
                    prompt.value = raw;
                    prompt.dispatchEvent(
                        new Event("input", { bubbles: true })
                    );
                    candidate.value = raw;
                    candidate.dispatchEvent(
                        new Event("input", { bubbles: true })
                    );
                    document.querySelector("#copy-prompt").click();
                    await new Promise((resolve) => setTimeout(resolve, 0));
                    const canonicalState = {
                        candidateExact: candidate.value === canonical,
                        clipboardExact:
                            globalThis.__clipboardWrites.length === 1
                            && globalThis.__clipboardWrites[0] === canonical,
                        codePoints: Array.from(prompt.value).map((value) => {
                            return value.codePointAt(0);
                        }),
                        promptExact: prompt.value === canonical,
                        rawChanged: prompt.value !== raw,
                        status: document.querySelector("#copy-status")
                            .textContent.trim()
                    };
                    return JSON.stringify({
                        canonicalState,
                        largeState
                    });
                })()`,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.equal(response.type, "success");
            assert.equal(response.result.type, "string");
            const state = JSON.parse(response.result.value);
            assert.deepEqual(state.largeState, {
                bytes: 1_406_010,
                candidateExact: true,
                clipboardExact: true,
                promptExact: true,
                responseChildren: 0,
                promptChildren: 0,
                status: "Prompt copied.",
            });
            const canonical = [
                "first",
                "second",
                "third",
                "NFC=á",
                "NFD=a\u{301}",
                "ZWJ=👩‍🔬",
            ].join("\n");
            assert.deepEqual(state.canonicalState, {
                candidateExact: true,
                clipboardExact: true,
                codePoints: Array.from(canonical).map((value) => {
                    return value.codePointAt(0);
                }),
                promptExact: true,
                rawChanged: true,
                status: "Prompt copied.",
            });
            assert.equal(
                requests.some((request) => request.startsWith("/api/")),
                false,
            );
        } finally {
            if (bidi?.socket != null) {
                bidi.socket.end();
            }
            if (server.listening) {
                server.close();
                await once(server, "close");
            }
            for (const child of children.reverse()) {
                await stopChild(child);
            }
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox Copy prompt ignores stale writes and reports current failures",
    { skip: !FIREFOX_AVAILABLE, timeout: 30_000 },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/workspace-copy-lifecycle-");
        const requests = [];
        const server = createStaticWorkspaceServer(requests, true);
        const children = [];
        let bidi;
        try {
            server.listen(0, "127.0.0.1");
            await once(server, "listening");
            const address = server.address();
            assert.notEqual(typeof address, "string");
            assert.notEqual(address, null);
            const origin = `http://127.0.0.1:${address.port}`;

            bidi = await startBidiFirefox(profile, children);
            const tab = await bidi.command("browsingContext.create", {
                type: "tab",
            });
            await bidi.command("browsingContext.navigate", {
                context: tab.context,
                url: origin,
                wait: "complete",
            });

            const response = await bidi.command("script.evaluate", {
                awaitPromise: true,
                expression: `(async () => {
                    const prompt = document.querySelector("#prompt-output");
                    const copy = document.querySelector("#copy-prompt");
                    const status = document.querySelector("#copy-status");
                    const writes = [];
                    const pending = [];
                    Object.defineProperty(navigator, "clipboard", {
                        configurable: true,
                        value: {
                            writeText(text) {
                                writes.push(text);
                                return new Promise((resolve, reject) => {
                                    pending.push({ reject, resolve });
                                });
                            }
                        }
                    });
                    prompt.value = "first prompt";
                    prompt.dispatchEvent(
                        new Event("input", { bubbles: true })
                    );
                    copy.click();
                    const firstPending = status.textContent.trim();
                    prompt.value = "second prompt";
                    prompt.dispatchEvent(
                        new Event("input", { bubbles: true })
                    );
                    pending[0].resolve();
                    await new Promise((resolve) => setTimeout(resolve, 0));
                    const staleCompletion = status.textContent.trim();

                    copy.click();
                    const secondPending = status.textContent.trim();
                    pending[1].reject(new Error("clipboard denied"));
                    await new Promise((resolve) => setTimeout(resolve, 0));
                    const currentFailure = status.textContent.trim();

                    Object.defineProperty(navigator, "clipboard", {
                        configurable: true,
                        value: undefined
                    });
                    prompt.value = "third prompt";
                    prompt.dispatchEvent(
                        new Event("input", { bubbles: true })
                    );
                    copy.click();
                    const unavailable = status.textContent.trim();
                    return JSON.stringify({
                        currentFailure,
                        firstPending,
                        secondPending,
                        staleCompletion,
                        unavailable,
                        writes
                    });
                })()`,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.equal(response.type, "success");
            assert.equal(response.result.type, "string");
            assert.deepEqual(JSON.parse(response.result.value), {
                currentFailure: "Clipboard write failed.",
                firstPending: "Copying prompt…",
                secondPending: "Copying prompt…",
                staleCompletion: "",
                unavailable: "Clipboard access is unavailable.",
                writes: ["first prompt", "second prompt"],
            });
            assert.equal(
                requests.some((request) => request.startsWith("/api/")),
                false,
            );
        } finally {
            if (bidi?.socket != null) {
                bidi.socket.end();
            }
            if (server.listening) {
                server.close();
                await once(server, "close");
            }
            for (const child of children.reverse()) {
                await stopChild(child);
            }
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox serializes repeated prompt copy requests without UI churn",
    { skip: !FIREFOX_AVAILABLE, timeout: 30_000 },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/workspace-copy-stress-");
        const requests = [];
        const server = createStaticWorkspaceServer(requests, true);
        const children = [];
        let bidi;
        try {
            server.listen(0, "127.0.0.1");
            await once(server, "listening");
            const address = server.address();
            assert.notEqual(typeof address, "string");
            assert.notEqual(address, null);
            const origin = `http://127.0.0.1:${address.port}`;

            bidi = await startBidiFirefox(profile, children);
            const tab = await bidi.command("browsingContext.create", {
                type: "tab",
            });
            await bidi.command("browsingContext.navigate", {
                context: tab.context,
                url: origin,
                wait: "complete",
            });

            const response = await bidi.command("script.evaluate", {
                awaitPromise: true,
                expression: `(async () => {
                    const prompt = document.querySelector("#prompt-output");
                    const copy = document.querySelector("#copy-prompt");
                    const status = document.querySelector("#copy-status");
                    const writes = [];
                    const pending = [];
                    Object.defineProperty(navigator, "clipboard", {
                        configurable: true,
                        value: {
                            writeText(text) {
                                writes.push(text);
                                return new Promise((resolve) => {
                                    pending.push(resolve);
                                });
                            }
                        }
                    });
                    prompt.value = "repeat-safe prompt";
                    prompt.dispatchEvent(
                        new Event("input", { bubbles: true })
                    );
                    let statusMutations = 0;
                    let disabledMutations = 0;
                    const statusObserver = new MutationObserver((records) => {
                        statusMutations += records.length;
                    });
                    statusObserver.observe(status, {
                        characterData: true,
                        childList: true,
                        subtree: true
                    });
                    const buttonObserver = new MutationObserver((records) => {
                        disabledMutations += records.filter((record) => {
                            return record.attributeName === "disabled";
                        }).length;
                    });
                    buttonObserver.observe(copy, { attributes: true });
                    for (let index = 0; index < 1_000; index += 1) {
                        prompt.dispatchEvent(
                            new Event("input", { bubbles: true })
                        );
                    }
                    for (let index = 0; index < 1_000; index += 1) {
                        copy.click();
                    }
                    await new Promise((resolve) => setTimeout(resolve, 0));
                    const pendingState = {
                        disabledMutations,
                        pendingWrites: pending.length,
                        status: status.textContent.trim(),
                        statusMutations,
                        writes: writes.length
                    };
                    pending[0]();
                    await new Promise((resolve) => setTimeout(resolve, 0));
                    statusObserver.disconnect();
                    buttonObserver.disconnect();
                    return JSON.stringify({
                        finalStatus: status.textContent.trim(),
                        pendingState,
                        writes
                    });
                })()`,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.equal(response.type, "success");
            assert.equal(response.result.type, "string");
            assert.deepEqual(JSON.parse(response.result.value), {
                finalStatus: "Prompt copied.",
                pendingState: {
                    disabledMutations: 0,
                    pendingWrites: 1,
                    status: "Copying prompt…",
                    statusMutations: 1,
                    writes: 1,
                },
                writes: ["repeat-safe prompt"],
            });
            assert.equal(
                requests.some((request) => request.startsWith("/api/")),
                false,
            );
        } finally {
            if (bidi?.socket != null) {
                bidi.socket.end();
            }
            if (server.listening) {
                server.close();
                await once(server, "close");
            }
            for (const child of children.reverse()) {
                await stopChild(child);
            }
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox trusted Tab order follows compact divider availability",
    { skip: !FIREFOX_AVAILABLE, timeout: 30_000 },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/workspace-tab-order-");
        const requests = [];
        const server = createStaticWorkspaceServer(requests, true);
        const children = [];
        let bidi;
        try {
            server.listen(0, "127.0.0.1");
            await once(server, "listening");
            const address = server.address();
            assert.notEqual(typeof address, "string");
            assert.notEqual(address, null);
            const origin = `http://127.0.0.1:${address.port}`;

            bidi = await startBidiFirefox(profile, children);
            for (const [width, expected] of [
                [320, [
                    { href: "#llm-editor-title", id: "", tag: "A" },
                    { href: "#human-editor-title", id: "", tag: "A" },
                    { href: null, id: "source-panel", tag: "SECTION" },
                    { href: null, id: "preview-panel", tag: "SECTION" },
                    { href: null, id: "zoom-out", tag: "BUTTON" },
                    { href: null, id: "zoom-in", tag: "BUTTON" },
                    { href: null, id: "page-stage", tag: "DIV" },
                ]],
                [481, [
                    { href: "#llm-editor-title", id: "", tag: "A" },
                    { href: "#human-editor-title", id: "", tag: "A" },
                    { href: null, id: "source-panel", tag: "SECTION" },
                    {
                        href: null,
                        id: "workspace-divider",
                        tag: "DIV",
                    },
                    { href: null, id: "preview-panel", tag: "SECTION" },
                    { href: null, id: "zoom-out", tag: "BUTTON" },
                    { href: null, id: "zoom-in", tag: "BUTTON" },
                    { href: null, id: "page-stage", tag: "DIV" },
                ]],
            ]) {
                const tab = await bidi.command("browsingContext.create", {
                    type: "tab",
                });
                await bidi.command("browsingContext.navigate", {
                    context: tab.context,
                    url: origin,
                    wait: "complete",
                });
                await setFirefoxViewport(bidi, tab.context, width, 480);
                const observed = [];
                for (let index = 0; index < expected.length; index += 1) {
                    await performTrustedKey(bidi, tab.context, "\uE004");
                    observed.push(
                        await activeElementIdentity(bidi, tab.context),
                    );
                }
                assert.deepEqual(
                    observed,
                    expected,
                    `${width}px trusted Tab order`,
                );
                await bidi.command("browsingContext.close", {
                    context: tab.context,
                });
            }
            assert.equal(
                requests.some((request) => request.startsWith("/api/")),
                false,
            );
        } finally {
            if (bidi?.socket != null) {
                bidi.socket.end();
            }
            if (server.listening) {
                server.close();
                await once(server, "close");
            }
            for (const child of children.reverse()) {
                await stopChild(child);
            }
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox trusted keyboard activates native skip links without workspace JS",
    { skip: !FIREFOX_AVAILABLE, timeout: 30_000 },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/workspace-skip-links-");
        const requests = [];
        const server = createStaticWorkspaceServer(requests);
        const children = [];
        let bidi;
        try {
            server.listen(0, "127.0.0.1");
            await once(server, "listening");
            const address = server.address();
            assert.notEqual(typeof address, "string");
            assert.notEqual(address, null);
            const origin = `http://127.0.0.1:${address.port}`;

            bidi = await startBidiFirefox(profile, children);
            for (const [tabs, expected] of [
                [1, {
                    hash: "#llm-editor-title",
                    heading: "llm-editor-title",
                    panel: "source-panel",
                }],
                [2, {
                    hash: "#human-editor-title",
                    heading: "human-editor-title",
                    panel: "preview-panel",
                }],
            ]) {
                const tab = await bidi.command("browsingContext.create", {
                    type: "tab",
                });
                await bidi.command("browsingContext.navigate", {
                    context: tab.context,
                    url: origin,
                    wait: "complete",
                });
                await setFirefoxViewport(bidi, tab.context, 320, 225);
                for (let index = 0; index < tabs; index += 1) {
                    await performTrustedKey(bidi, tab.context, "\uE004");
                }
                await performTrustedKey(bidi, tab.context, "\uE007");
                const response = await bidi.command("script.evaluate", {
                    awaitPromise: false,
                    expression: `JSON.stringify((() => {
                        const heading = document.activeElement;
                        const panel = document.querySelector(
                            ${JSON.stringify(`#${expected.panel}`)}
                        );
                        const box = heading.getBoundingClientRect();
                        const panelBox = panel.getBoundingClientRect();
                        return {
                            active: heading.id,
                            hash: window.location.hash,
                            headingVisible:
                                box.bottom > panelBox.top
                                && box.top < panelBox.bottom,
                            panelScrollLeft: panel.scrollLeft,
                            panelScrollTop: panel.scrollTop
                        };
                    })())`,
                    resultOwnership: "none",
                    target: { context: tab.context },
                });
                assert.equal(response.type, "success");
                assert.equal(response.result.type, "string");
                assert.deepEqual(JSON.parse(response.result.value), {
                    active: expected.heading,
                    hash: expected.hash,
                    headingVisible: true,
                    panelScrollLeft: 0,
                    panelScrollTop: 0,
                });
                await bidi.command("browsingContext.close", {
                    context: tab.context,
                });
            }
            assert.equal(
                requests.some((request) => request.startsWith("/api/")),
                false,
            );
        } finally {
            if (bidi?.socket != null) {
                bidi.socket.end();
            }
            if (server.listening) {
                server.close();
                await once(server, "close");
            }
            for (const child of children.reverse()) {
                await stopChild(child);
            }
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox trusted divider keys and zoom boundaries preserve keyboard focus",
    { skip: !FIREFOX_AVAILABLE, timeout: 30_000 },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/workspace-keyboard-controls-");
        const requests = [];
        const server = createStaticWorkspaceServer(requests, true);
        const children = [];
        let bidi;
        try {
            server.listen(0, "127.0.0.1");
            await once(server, "listening");
            const address = server.address();
            assert.notEqual(typeof address, "string");
            assert.notEqual(address, null);
            const origin = `http://127.0.0.1:${address.port}`;

            bidi = await startBidiFirefox(profile, children);
            const tab = await bidi.command("browsingContext.create", {
                type: "tab",
            });
            await bidi.command("browsingContext.navigate", {
                context: tab.context,
                url: origin,
                wait: "complete",
            });
            await setFirefoxViewport(bidi, tab.context, 481, 480);
            for (let index = 0; index < 4; index += 1) {
                await performTrustedKey(bidi, tab.context, "\uE004");
            }
            assert.deepEqual(
                await activeElementIdentity(bidi, tab.context),
                { href: null, id: "workspace-divider", tag: "DIV" },
            );
            await bidi.command("script.evaluate", {
                awaitPromise: false,
                expression: `(() => {
                    globalThis.__dividerKeyEvents = [];
                    document.addEventListener("keydown", (event) => {
                        const navigationKeys = new Set([
                            "ArrowLeft",
                            "ArrowRight",
                            "Home",
                            "End"
                        ]);
                        if (
                            event.target?.id !== "workspace-divider"
                            || !navigationKeys.has(event.key)
                        ) {
                            return;
                        }
                        queueMicrotask(() => {
                            globalThis.__dividerKeyEvents.push({
                                ctrl: event.ctrlKey,
                                defaultPrevented: event.defaultPrevented,
                                key: event.key,
                                shift: event.shiftKey
                            });
                        });
                    });
                })()`,
                resultOwnership: "none",
                target: { context: tab.context },
            });

            await performTrustedKeyChord(
                bidi,
                tab.context,
                "\uE009",
                "\uE014",
            );
            await performTrustedKey(bidi, tab.context, "\uE014");
            await performTrustedKey(bidi, tab.context, "\uE011");
            await performTrustedKey(bidi, tab.context, "\uE010");
            await performTrustedKeyChord(
                bidi,
                tab.context,
                "\uE008",
                "\uE011",
            );
            await new Promise((resolve) => setTimeout(resolve, 0));
            const dividerResponse = await bidi.command("script.evaluate", {
                awaitPromise: false,
                expression: `JSON.stringify({
                    events: globalThis.__dividerKeyEvents,
                    now: document.querySelector("#workspace-divider")
                        .getAttribute("aria-valuenow")
                })`,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.equal(dividerResponse.type, "success");
            assert.equal(dividerResponse.result.type, "string");
            assert.deepEqual(JSON.parse(dividerResponse.result.value), {
                events: [
                    {
                        ctrl: true,
                        defaultPrevented: false,
                        key: "ArrowRight",
                        shift: false,
                    },
                    {
                        ctrl: false,
                        defaultPrevented: true,
                        key: "ArrowRight",
                        shift: false,
                    },
                    {
                        ctrl: false,
                        defaultPrevented: true,
                        key: "Home",
                        shift: false,
                    },
                    {
                        ctrl: false,
                        defaultPrevented: true,
                        key: "End",
                        shift: false,
                    },
                    {
                        ctrl: false,
                        defaultPrevented: false,
                        key: "Home",
                        shift: true,
                    },
                ],
                now: "65",
            });

            await performTrustedKey(bidi, tab.context, "\uE004");
            await performTrustedKey(bidi, tab.context, "\uE004");
            assert.deepEqual(
                await activeElementIdentity(bidi, tab.context),
                { href: null, id: "zoom-out", tag: "BUTTON" },
            );
            for (let index = 0; index < 4; index += 1) {
                await performTrustedKey(bidi, tab.context, "\uE007");
            }
            assert.deepEqual(
                await activeElementIdentity(bidi, tab.context),
                { href: null, id: "zoom-reset", tag: "BUTTON" },
            );
            const lowerBoundary = await bidi.command("script.evaluate", {
                awaitPromise: false,
                expression: `JSON.stringify({
                    scale: document.querySelector("#preview-scale")
                        .textContent.trim(),
                    zoomOutDisabled:
                        document.querySelector("#zoom-out").disabled
                })`,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.equal(lowerBoundary.type, "success");
            assert.equal(lowerBoundary.result.type, "string");
            assert.deepEqual(JSON.parse(lowerBoundary.result.value), {
                scale: "Preview · 60%",
                zoomOutDisabled: true,
            });
            await performTrustedKey(bidi, tab.context, "\uE007");
            assert.deepEqual(
                await activeElementIdentity(bidi, tab.context),
                { href: null, id: "zoom-out", tag: "BUTTON" },
            );
            const resetState = await bidi.command("script.evaluate", {
                awaitPromise: false,
                expression: `JSON.stringify({
                    resetDisabled:
                        document.querySelector("#zoom-reset").disabled,
                    scale: document.querySelector("#preview-scale")
                        .textContent.trim()
                })`,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.equal(resetState.type, "success");
            assert.equal(resetState.result.type, "string");
            assert.deepEqual(JSON.parse(resetState.result.value), {
                resetDisabled: true,
                scale: "Preview · 100%",
            });
            assert.equal(
                requests.some((request) => request.startsWith("/api/")),
                false,
            );
        } finally {
            if (bidi?.socket != null) {
                bidi.socket.end();
            }
            if (server.listening) {
                server.close();
                await once(server, "close");
            }
            for (const child of children.reverse()) {
                await stopChild(child);
            }
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox text-spacing override keeps short workspace controls reachable",
    { skip: !FIREFOX_AVAILABLE, timeout: 30_000 },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/workspace-text-spacing-");
        const requests = [];
        const server = createStaticWorkspaceServer(requests, true);
        const children = [];
        let bidi;
        try {
            server.listen(0, "127.0.0.1");
            await once(server, "listening");
            const address = server.address();
            assert.notEqual(typeof address, "string");
            assert.notEqual(address, null);
            const origin = `http://127.0.0.1:${address.port}`;

            bidi = await startBidiFirefox(profile, children);
            for (const width of [320, 480, 481]) {
                const tab = await bidi.command("browsingContext.create", {
                    type: "tab",
                });
                await bidi.command("browsingContext.navigate", {
                    context: tab.context,
                    url: origin,
                    wait: "complete",
                });
                await setFirefoxViewport(bidi, tab.context, width, 225);
                const response = await bidi.command("script.evaluate", {
                    awaitPromise: false,
                    expression: `JSON.stringify((() => {
                        for (const element of document.querySelectorAll("*")) {
                            element.style.setProperty(
                                "line-height",
                                "1.5",
                                "important"
                            );
                            element.style.setProperty(
                                "letter-spacing",
                                "0.12em",
                                "important"
                            );
                            element.style.setProperty(
                                "word-spacing",
                                "0.16em",
                                "important"
                            );
                        }
                        for (
                            const paragraph of document.querySelectorAll("p")
                        ) {
                            paragraph.style.setProperty(
                                "margin-bottom",
                                "2em",
                                "important"
                            );
                        }
                        const inside = (element, owner) => {
                            const box = element.getBoundingClientRect();
                            const ownerBox = owner.getBoundingClientRect();
                            return box.width > 0
                                && box.height > 0
                                && box.right > ownerBox.left
                                && box.left < ownerBox.right
                                && box.bottom > ownerBox.top
                                && box.top < ownerBox.bottom;
                        };
                        const source = document.querySelector("#source-panel");
                        const preview = document.querySelector(
                            "#preview-panel"
                        );
                        const task = document.querySelector("#task-input");
                        const copy = document.querySelector("#copy-prompt");
                        const stage = document.querySelector("#page-stage");
                        const diagnostics = document.querySelector(
                            ".diagnostics"
                        );
                        task.scrollIntoView({ block: "nearest" });
                        const taskReachable = inside(task, source);
                        const copyReachable = inside(copy, source);
                        stage.scrollIntoView({ block: "nearest" });
                        const stageReachable = inside(stage, preview);
                        diagnostics.scrollIntoView({ block: "nearest" });
                        const diagnosticsReachable = inside(
                            diagnostics,
                            preview
                        );
                        return {
                            copyReachable,
                            diagnosticsReachable,
                            document: {
                                height: document.documentElement.scrollHeight,
                                width: document.documentElement.scrollWidth,
                                x: window.scrollX,
                                y: window.scrollY
                            },
                            previewScrollTop: preview.scrollTop,
                            sourceScrollTop: source.scrollTop,
                            stageReachable,
                            taskReachable,
                            viewport: {
                                height: window.innerHeight,
                                width: window.innerWidth
                            }
                        };
                    })())`,
                    resultOwnership: "none",
                    target: { context: tab.context },
                });
                assert.equal(response.type, "success");
                assert.equal(response.result.type, "string");
                const state = JSON.parse(response.result.value);
                assert.deepEqual(state.viewport, { height: 225, width });
                assert.equal(state.taskReachable, true);
                assert.equal(state.copyReachable, true);
                assert.equal(state.stageReachable, true);
                assert.equal(state.diagnosticsReachable, true);
                assert.ok(state.sourceScrollTop >= 0);
                assert.ok(state.previewScrollTop > 0);
                assert.deepEqual(
                    state.document,
                    { height: 225, width, x: 0, y: 0 },
                );
                await bidi.command("browsingContext.close", {
                    context: tab.context,
                });
            }
            assert.equal(
                requests.some((request) => request.startsWith("/api/")),
                false,
            );
        } finally {
            if (bidi?.socket != null) {
                bidi.socket.end();
            }
            if (server.listening) {
                server.close();
                await once(server, "close");
            }
            for (const child of children.reverse()) {
                await stopChild(child);
            }
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox 200 percent text keeps compact skip links viewport bounded",
    { skip: !FIREFOX_AVAILABLE, timeout: 30_000 },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/workspace-text-zoom-");
        const requests = [];
        const server = createStaticWorkspaceServer(requests);
        const children = [];
        let bidi;
        try {
            server.listen(0, "127.0.0.1");
            await once(server, "listening");
            const address = server.address();
            assert.notEqual(typeof address, "string");
            assert.notEqual(address, null);
            const origin = `http://127.0.0.1:${address.port}`;

            bidi = await startBidiFirefox(profile, children);
            const observed = [];
            for (const width of [320, 480]) {
                const tab = await bidi.command("browsingContext.create", {
                    type: "tab",
                });
                await bidi.command("browsingContext.navigate", {
                    context: tab.context,
                    url: origin,
                    wait: "complete",
                });
                await setFirefoxViewport(bidi, tab.context, width, 480);
                await bidi.command("script.evaluate", {
                    awaitPromise: false,
                    expression: `(() => {
                        document.documentElement.style.fontSize = "200%";
                    })()`,
                    resultOwnership: "none",
                    target: { context: tab.context },
                });
                for (let index = 0; index < 2; index += 1) {
                    await performTrustedKey(bidi, tab.context, "\uE004");
                    const response = await bidi.command("script.evaluate", {
                        awaitPromise: false,
                        expression: `JSON.stringify((() => {
                            const active = document.activeElement;
                            const box = active.getBoundingClientRect();
                            return {
                                bottom: box.bottom,
                                height: box.height,
                                href: active.getAttribute("href"),
                                left: box.left,
                                right: box.right,
                                scrollWidth:
                                    document.documentElement.scrollWidth,
                                top: box.top,
                                width: window.innerWidth
                            };
                        })())`,
                        resultOwnership: "none",
                        target: { context: tab.context },
                    });
                    assert.equal(response.type, "success");
                    assert.equal(response.result.type, "string");
                    observed.push(JSON.parse(response.result.value));
                }
                await bidi.command("browsingContext.close", {
                    context: tab.context,
                });
            }
            const expected = [
                { height: 104, href: "#llm-editor-title", width: 320 },
                { height: 60, href: "#human-editor-title", width: 320 },
                { height: 60, href: "#llm-editor-title", width: 480 },
                { height: 60, href: "#human-editor-title", width: 480 },
            ];
            assert.equal(observed.length, expected.length);
            for (let index = 0; index < observed.length; index += 1) {
                const state = observed[index];
                assert.equal(state.href, expected[index].href);
                assert.equal(state.height, expected[index].height);
                assert.equal(state.width, expected[index].width);
                assert.equal(state.left, 8);
                assert.equal(state.top, 8);
                assert.ok(state.right <= state.width + 1);
                assert.ok(state.bottom <= 480);
                assert.ok(state.scrollWidth <= state.width);
            }
        } finally {
            if (bidi?.socket != null) {
                bidi.socket.end();
            }
            if (server.listening) {
                server.close();
                await once(server, "close");
            }
            for (const child of children.reverse()) {
                await stopChild(child);
            }
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox compaction moves focused divider to the visible source heading",
    { skip: !FIREFOX_AVAILABLE, timeout: 30_000 },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/workspace-compact-focus-");
        const requests = [];
        const server = createStaticWorkspaceServer(requests, true);
        const children = [];
        let bidi;
        try {
            server.listen(0, "127.0.0.1");
            await once(server, "listening");
            const address = server.address();
            assert.notEqual(typeof address, "string");
            assert.notEqual(address, null);
            const origin = `http://127.0.0.1:${address.port}`;

            bidi = await startBidiFirefox(profile, children);
            const tab = await bidi.command("browsingContext.create", {
                type: "tab",
            });
            await bidi.command("browsingContext.navigate", {
                context: tab.context,
                url: origin,
                wait: "complete",
            });
            await setFirefoxViewport(bidi, tab.context, 481, 225);
            for (let index = 0; index < 4; index += 1) {
                await performTrustedKey(bidi, tab.context, "\uE004");
            }
            assert.deepEqual(
                await activeElementIdentity(bidi, tab.context),
                { href: null, id: "workspace-divider", tag: "DIV" },
            );
            const scrolled = await bidi.command("script.evaluate", {
                awaitPromise: false,
                expression: `JSON.stringify((() => {
                    const panel = document.querySelector("#source-panel");
                    const header = document.querySelector(".editor-heading");
                    panel.scrollTop = panel.scrollHeight;
                    const panelBox = panel.getBoundingClientRect();
                    const headerBox = header.getBoundingClientRect();
                    return {
                        headerAbovePanel:
                            headerBox.bottom <= panelBox.top + 1,
                        scrollTop: panel.scrollTop
                    };
                })())`,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.equal(scrolled.type, "success");
            assert.equal(scrolled.result.type, "string");
            const scrolledState = JSON.parse(scrolled.result.value);
            assert.ok(scrolledState.scrollTop > 0);
            assert.equal(scrolledState.headerAbovePanel, true);

            await setFirefoxViewport(bidi, tab.context, 480, 225);
            const compacted = await bidi.command("script.evaluate", {
                awaitPromise: false,
                expression: `JSON.stringify((() => {
                    const active = document.activeElement;
                    const panel = document.querySelector("#source-panel");
                    const header = document.querySelector(".editor-heading");
                    const divider = document.querySelector(
                        "#workspace-divider"
                    );
                    const panelBox = panel.getBoundingClientRect();
                    const headerBox = header.getBoundingClientRect();
                    return {
                        active: active.id,
                        dividerDisabled:
                            divider.getAttribute("aria-disabled"),
                        dividerTabindex:
                            divider.getAttribute("tabindex"),
                        headerFullyVisible:
                            headerBox.top >= panelBox.top - 1
                            && headerBox.bottom <= panelBox.bottom + 1,
                        scrollTop: panel.scrollTop
                    };
                })())`,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.equal(compacted.type, "success");
            assert.equal(compacted.result.type, "string");
            assert.deepEqual(JSON.parse(compacted.result.value), {
                active: "llm-editor-title",
                dividerDisabled: "true",
                dividerTabindex: "-1",
                headerFullyVisible: true,
                scrollTop: 0,
            });
            assert.equal(
                requests.some((request) => request.startsWith("/api/")),
                false,
            );
        } finally {
            if (bidi?.socket != null) {
                bidi.socket.end();
            }
            if (server.listening) {
                server.close();
                await once(server, "close");
            }
            for (const child of children.reverse()) {
                await stopChild(child);
            }
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox pointer-capture fallback preserves focus until completed click",
    { skip: !FIREFOX_AVAILABLE, timeout: 30_000 },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/workspace-pointer-fallback-");
        const requests = [];
        const server = createStaticWorkspaceServer(requests, true);
        const children = [];
        let bidi;
        try {
            server.listen(0, "127.0.0.1");
            await once(server, "listening");
            const address = server.address();
            assert.notEqual(typeof address, "string");
            assert.notEqual(address, null);
            const origin = `http://127.0.0.1:${address.port}`;

            bidi = await startBidiFirefox(profile, children);
            for (const mode of ["throw", "silent"]) {
                const tab = await bidi.command("browsingContext.create", {
                    type: "tab",
                });
                await bidi.command("browsingContext.navigate", {
                    context: tab.context,
                    url: origin,
                    wait: "complete",
                });
                await setFirefoxViewport(bidi, tab.context, 640, 480);
                const setup = await bidi.command("script.evaluate", {
                    awaitPromise: false,
                    expression: `JSON.stringify((() => {
                        const divider = document.querySelector(
                            "#workspace-divider"
                        );
                        const source = document.querySelector("#source-panel");
                        const workspace = document.querySelector(
                            ".workspace-grid"
                        );
                        source.focus();
                        if (${JSON.stringify(mode)} === "throw") {
                            divider.setPointerCapture = () => {
                                throw new DOMException("capture denied");
                            };
                        } else {
                            divider.setPointerCapture = () => {};
                            divider.hasPointerCapture = () => false;
                        }
                        globalThis.__fallbackPointerDown = null;
                        divider.addEventListener("pointerdown", (event) => {
                            queueMicrotask(() => {
                                globalThis.__fallbackPointerDown = {
                                    defaultPrevented: event.defaultPrevented,
                                    pointerId: event.pointerId
                                };
                            });
                        });
                        const dividerBox = divider.getBoundingClientRect();
                        const workspaceBox = workspace.getBoundingClientRect();
                        const panelWidth =
                            workspaceBox.width - dividerBox.width;
                        const x = Math.round(
                            dividerBox.left + dividerBox.width / 2 + 7
                        );
                        const sourceWidth =
                            x
                            - workspaceBox.left
                            - dividerBox.width / 2;
                        const expectedShare = Math.round(
                            (sourceWidth / panelWidth) * 1_000
                        ) / 10;
                        return {
                            expectedShare,
                            x,
                            y: Math.round(
                                dividerBox.top + dividerBox.height / 2
                            )
                        };
                    })())`,
                    resultOwnership: "none",
                    target: { context: tab.context },
                });
                assert.equal(setup.type, "success");
                assert.equal(setup.result.type, "string");
                const geometry = JSON.parse(setup.result.value);

                await performTrustedPointer(bidi, tab.context, [
                    {
                        duration: 0,
                        origin: "viewport",
                        type: "pointerMove",
                        x: geometry.x,
                        y: geometry.y,
                    },
                    { button: 0, type: "pointerDown" },
                ]);
                const down = await bidi.command("script.evaluate", {
                    awaitPromise: true,
                    expression: `(async () => {
                        await new Promise((resolve) => setTimeout(resolve, 0));
                        const divider = document.querySelector(
                            "#workspace-divider"
                        );
                        const evidence = globalThis.__fallbackPointerDown;
                        return JSON.stringify({
                            active: document.activeElement.id,
                            dataPointerDrag:
                                divider.hasAttribute("data-pointer-drag"),
                            defaultPrevented: evidence.defaultPrevented,
                            hasCapture:
                                divider.hasPointerCapture(evidence.pointerId),
                            now: divider.getAttribute("aria-valuenow"),
                            touchAction:
                                getComputedStyle(divider).touchAction
                        });
                    })()`,
                    resultOwnership: "none",
                    target: { context: tab.context },
                });
                assert.equal(down.type, "success");
                assert.equal(down.result.type, "string");
                assert.deepEqual(JSON.parse(down.result.value), {
                    active: "source-panel",
                    dataPointerDrag: false,
                    defaultPrevented: false,
                    hasCapture: false,
                    now: "46",
                    touchAction: "auto",
                });

                await performTrustedPointer(bidi, tab.context, [
                    { button: 0, type: "pointerUp" },
                ]);
                const completed = await bidi.command("script.evaluate", {
                    awaitPromise: false,
                    expression: `document.querySelector("#workspace-divider")
                        .getAttribute("aria-valuenow")`,
                    resultOwnership: "none",
                    target: { context: tab.context },
                });
                assert.equal(completed.type, "success");
                assert.equal(completed.result.type, "string");
                assert.equal(
                    completed.result.value,
                    String(geometry.expectedShare),
                    `${mode} fallback click share`,
                );
                assert.notEqual(completed.result.value, "46");
                await bidi.command("browsingContext.close", {
                    context: tab.context,
                });
            }
            assert.equal(
                requests.some((request) => request.startsWith("/api/")),
                false,
            );
        } finally {
            if (bidi?.socket != null) {
                bidi.socket.end();
            }
            if (server.listening) {
                server.close();
                await once(server, "close");
            }
            for (const child of children.reverse()) {
                await stopChild(child);
            }
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox resize interrupts pointer capture without poisoning the next drag",
    { skip: !FIREFOX_AVAILABLE, timeout: 30_000 },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/workspace-pointer-resize-");
        const requests = [];
        const server = createStaticWorkspaceServer(requests, true);
        const children = [];
        let bidi;
        try {
            server.listen(0, "127.0.0.1");
            await once(server, "listening");
            const address = server.address();
            assert.notEqual(typeof address, "string");
            assert.notEqual(address, null);
            const origin = `http://127.0.0.1:${address.port}`;

            bidi = await startBidiFirefox(profile, children);
            const tab = await bidi.command("browsingContext.create", {
                type: "tab",
            });
            await bidi.command("browsingContext.navigate", {
                context: tab.context,
                url: origin,
                wait: "complete",
            });
            await setFirefoxViewport(bidi, tab.context, 640, 480);
            const initialGeometry = await bidi.command("script.evaluate", {
                awaitPromise: false,
                expression: `JSON.stringify((() => {
                    const divider = document.querySelector(
                        "#workspace-divider"
                    );
                    const box = divider.getBoundingClientRect();
                    globalThis.__dividerPointerEvidence = {
                        down: null,
                        lost: 0
                    };
                    divider.addEventListener("pointerdown", (event) => {
                        queueMicrotask(() => {
                            globalThis.__dividerPointerEvidence.down = {
                                captured:
                                    divider.hasPointerCapture(event.pointerId),
                                pointerId: event.pointerId
                            };
                        });
                    });
                    divider.addEventListener("lostpointercapture", () => {
                        globalThis.__dividerPointerEvidence.lost += 1;
                    });
                    return {
                        x: box.left + box.width / 2,
                        y: box.top + box.height / 2
                    };
                })())`,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.equal(initialGeometry.type, "success");
            assert.equal(initialGeometry.result.type, "string");
            const start = JSON.parse(initialGeometry.result.value);
            await performTrustedPointer(bidi, tab.context, [
                {
                    duration: 0,
                    origin: "viewport",
                    type: "pointerMove",
                    x: Math.round(start.x),
                    y: Math.round(start.y),
                },
                { button: 0, type: "pointerDown" },
            ]);
            await new Promise((resolve) => setTimeout(resolve, 0));
            const captured = await bidi.command("script.evaluate", {
                awaitPromise: false,
                expression: `JSON.stringify({
                    evidence: globalThis.__dividerPointerEvidence,
                    now: document.querySelector("#workspace-divider")
                        .getAttribute("aria-valuenow")
                })`,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.equal(captured.type, "success");
            assert.equal(captured.result.type, "string");
            const capturedState = JSON.parse(captured.result.value);
            assert.equal(capturedState.evidence.down.captured, true);
            assert.equal(capturedState.evidence.lost, 0);
            assert.equal(capturedState.now, "46");

            await setFirefoxViewport(bidi, tab.context, 800, 480);
            const released = await bidi.command("script.evaluate", {
                awaitPromise: false,
                expression: `JSON.stringify((() => {
                    const divider = document.querySelector(
                        "#workspace-divider"
                    );
                    const evidence = globalThis.__dividerPointerEvidence;
                    return {
                        evidence,
                        hasCapture: divider.hasPointerCapture(
                            evidence.down.pointerId
                        ),
                        now: divider.getAttribute("aria-valuenow")
                    };
                })())`,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.equal(released.type, "success");
            assert.equal(released.result.type, "string");
            const releasedState = JSON.parse(released.result.value);
            assert.deepEqual(releasedState.evidence.down, {
                captured: true,
                pointerId: capturedState.evidence.down.pointerId,
            });
            assert.equal(releasedState.hasCapture, false);
            assert.equal(releasedState.now, "46");

            await performTrustedPointer(bidi, tab.context, [
                {
                    duration: 50,
                    origin: "viewport",
                    type: "pointerMove",
                    x: 760,
                    y: Math.round(start.y),
                },
                { button: 0, type: "pointerUp" },
            ]);
            const afterLateMove = await bidi.command("script.evaluate", {
                awaitPromise: false,
                expression: `document.querySelector("#workspace-divider")
                    .getAttribute("aria-valuenow")`,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.equal(afterLateMove.type, "success");
            assert.equal(afterLateMove.result.type, "string");
            assert.equal(afterLateMove.result.value, "46");

            const nextGeometry = await bidi.command("script.evaluate", {
                awaitPromise: false,
                expression: `JSON.stringify((() => {
                    const divider = document.querySelector(
                        "#workspace-divider"
                    );
                    const workspace = document.querySelector(
                        ".workspace-grid"
                    );
                    const dividerBox = divider.getBoundingClientRect();
                    const workspaceBox = workspace.getBoundingClientRect();
                    const panelWidth =
                        workspaceBox.width - dividerBox.width;
                    const dividerCenter =
                        dividerBox.left + dividerBox.width / 2;
                    const startX = Math.round(dividerCenter + 3);
                    const grabOffset = startX - dividerCenter;
                    const targetX = Math.round(
                        workspaceBox.left
                        + panelWidth * 0.6
                        + dividerBox.width / 2
                        + grabOffset
                    );
                    const adjustedTarget = targetX - grabOffset;
                    const sourceWidth =
                        adjustedTarget
                        - workspaceBox.left
                        - dividerBox.width / 2;
                    const expectedShare = Math.round(
                        (sourceWidth / panelWidth) * 1_000
                    ) / 10;
                    return {
                        expectedShare,
                        startX,
                        targetX,
                        y: dividerBox.top + dividerBox.height / 2
                    };
                })())`,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.equal(nextGeometry.type, "success");
            assert.equal(nextGeometry.result.type, "string");
            const next = JSON.parse(nextGeometry.result.value);
            await performTrustedPointer(bidi, tab.context, [
                {
                    duration: 0,
                    origin: "viewport",
                    type: "pointerMove",
                    x: next.startX,
                    y: Math.round(next.y),
                },
                { button: 0, type: "pointerDown" },
                {
                    duration: 80,
                    origin: "viewport",
                    type: "pointerMove",
                    x: next.targetX,
                    y: Math.round(next.y),
                },
                { button: 0, type: "pointerUp" },
            ]);
            const finalShare = await bidi.command("script.evaluate", {
                awaitPromise: false,
                expression: `document.querySelector("#workspace-divider")
                    .getAttribute("aria-valuenow")`,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.equal(finalShare.type, "success");
            assert.equal(finalShare.result.type, "string");
            assert.equal(
                finalShare.result.value,
                String(next.expectedShare),
            );
            assert.notEqual(finalShare.result.value, "46");
            assert.equal(
                requests.some((request) => request.startsWith("/api/")),
                false,
            );
        } finally {
            if (bidi?.socket != null) {
                bidi.socket.end();
            }
            if (server.listening) {
                server.close();
                await once(server, "close");
            }
            for (const child of children.reverse()) {
                await stopChild(child);
            }
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox divider ARIA changes only when compact state changes",
    { skip: !FIREFOX_AVAILABLE, timeout: 30_000 },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/workspace-resize-aria-");
        const requests = [];
        const server = createStaticWorkspaceServer(requests, true);
        const children = [];
        let bidi;
        try {
            server.listen(0, "127.0.0.1");
            await once(server, "listening");
            const address = server.address();
            assert.notEqual(typeof address, "string");
            assert.notEqual(address, null);
            const origin = `http://127.0.0.1:${address.port}`;

            bidi = await startBidiFirefox(profile, children);
            const tab = await bidi.command("browsingContext.create", {
                type: "tab",
            });
            await bidi.command("browsingContext.navigate", {
                context: tab.context,
                url: origin,
                wait: "complete",
            });
            await setFirefoxViewport(bidi, tab.context, 640, 480);
            await bidi.command("script.evaluate", {
                awaitPromise: false,
                expression: `(() => {
                    globalThis.__dividerAttributeMutations = {};
                    const divider = document.querySelector(
                        "#workspace-divider"
                    );
                    globalThis.__dividerAttributeObserver =
                        new MutationObserver((records) => {
                            for (const record of records) {
                                const name = record.attributeName;
                                globalThis.__dividerAttributeMutations[name] =
                                    (globalThis.__dividerAttributeMutations[
                                        name
                                    ] ?? 0) + 1;
                            }
                        });
                    globalThis.__dividerAttributeObserver.observe(divider, {
                        attributeFilter: [
                            "aria-disabled",
                            "aria-valuemax",
                            "aria-valuemin",
                            "aria-valuenow",
                            "aria-valuetext",
                            "tabindex"
                        ],
                        attributes: true
                    });
                })()`,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            const mutationCounts = async () => {
                await new Promise((resolve) => setTimeout(resolve, 0));
                const response = await bidi.command("script.evaluate", {
                    awaitPromise: false,
                    expression: `JSON.stringify(
                        globalThis.__dividerAttributeMutations
                    )`,
                    resultOwnership: "none",
                    target: { context: tab.context },
                });
                assert.equal(response.type, "success");
                assert.equal(response.result.type, "string");
                return JSON.parse(response.result.value);
            };

            for (const width of [641, 1024, 481]) {
                await setFirefoxViewport(bidi, tab.context, width, 480);
            }
            assert.deepEqual(await mutationCounts(), {});

            await setFirefoxViewport(bidi, tab.context, 480, 480);
            const oneTransition = {
                "aria-disabled": 1,
                "aria-valuemax": 1,
                "aria-valuemin": 1,
                "aria-valuenow": 1,
                "aria-valuetext": 1,
                tabindex: 1,
            };
            assert.deepEqual(await mutationCounts(), oneTransition);

            await setFirefoxViewport(bidi, tab.context, 479, 480);
            assert.deepEqual(await mutationCounts(), oneTransition);

            await setFirefoxViewport(bidi, tab.context, 481, 480);
            assert.deepEqual(await mutationCounts(), {
                "aria-disabled": 2,
                "aria-valuemax": 2,
                "aria-valuemin": 2,
                "aria-valuenow": 2,
                "aria-valuetext": 2,
                tabindex: 2,
            });
            await bidi.command("script.evaluate", {
                awaitPromise: false,
                expression:
                    `globalThis.__dividerAttributeObserver.disconnect()`,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.equal(
                requests.some((request) => request.startsWith("/api/")),
                false,
            );
        } finally {
            if (bidi?.socket != null) {
                bidi.socket.end();
            }
            if (server.listening) {
                server.close();
                await once(server, "close");
            }
            for (const child of children.reverse()) {
                await stopChild(child);
            }
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox compact breakpoint matches viewport CSS and divider state",
    { skip: !FIREFOX_AVAILABLE, timeout: 30_000 },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/workspace-breakpoint-");
        const requests = [];
        const server = createStaticWorkspaceServer(requests, true);
        const children = [];
        let bidi;
        try {
            server.listen(0, "127.0.0.1");
            await once(server, "listening");
            const address = server.address();
            assert.notEqual(typeof address, "string");
            assert.notEqual(address, null);
            const origin = `http://127.0.0.1:${address.port}`;

            bidi = await startBidiFirefox(profile, children);
            const tab = await bidi.command("browsingContext.create", {
                type: "tab",
            });
            await bidi.command("browsingContext.navigate", {
                context: tab.context,
                url: origin,
                wait: "complete",
            });
            const observed = [];
            for (const width of [320, 479, 480, 481, 482, 640, 1024]) {
                await setFirefoxViewport(bidi, tab.context, width, 480);
                const response = await bidi.command("script.evaluate", {
                    awaitPromise: false,
                    expression: `JSON.stringify((() => {
                        const divider = document.querySelector(
                            "#workspace-divider"
                        );
                        return {
                            compactCss: window.matchMedia(
                                "(max-width: 480px)"
                            ).matches,
                            disabled:
                                divider.getAttribute("aria-disabled"),
                            maximum: divider.getAttribute("aria-valuemax"),
                            minimum: divider.getAttribute("aria-valuemin"),
                            now: divider.getAttribute("aria-valuenow"),
                            tabindex: divider.getAttribute("tabindex"),
                            width: window.innerWidth
                        };
                    })())`,
                    resultOwnership: "none",
                    target: { context: tab.context },
                });
                assert.equal(response.type, "success");
                assert.equal(response.result.type, "string");
                observed.push(JSON.parse(response.result.value));
            }
            assert.deepEqual(
                observed,
                [320, 479, 480, 481, 482, 640, 1024].map((width) => {
                    const compact = width <= 480;
                    return {
                        compactCss: compact,
                        disabled: compact ? "true" : null,
                        maximum: compact ? "50" : "65",
                        minimum: compact ? "50" : "35",
                        now: compact ? "50" : "46",
                        tabindex: compact ? "-1" : "0",
                        width,
                    };
                }),
            );
            assert.equal(
                requests.some((request) => request.startsWith("/api/")),
                false,
            );
        } finally {
            if (bidi?.socket != null) {
                bidi.socket.end();
            }
            if (server.listening) {
                server.close();
                await once(server, "close");
            }
            for (const child of children.reverse()) {
                await stopChild(child);
            }
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox viewport matrix keeps both workspace surfaces reachable",
    { skip: !FIREFOX_AVAILABLE, timeout: 30_000 },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/workspace-layout-");
        const apiRequests = [];
        const server = createSessionFixtureServer(apiRequests);
        const children = [];
        let bidi;
        try {
            server.listen(0, "127.0.0.1");
            await once(server, "listening");
            const address = server.address();
            assert.notEqual(typeof address, "string");
            assert.notEqual(address, null);
            const origin = `http://127.0.0.1:${address.port}`;

            bidi = await startBidiFirefox(profile, children);
            const tab = await bidi.command("browsingContext.create", {
                type: "tab",
            });
            await bidi.command("browsingContext.navigate", {
                context: tab.context,
                url: origin,
                wait: "complete",
            });

            const widths = [320, 480, 481, 1024];
            const heights = [225, 360, 480, 576];
            const variants = [
                { key: "Home", share: 35, zoom: 60 },
                { key: "End", share: 65, zoom: 160 },
            ];
            let cases = 0;
            for (const width of widths) {
                for (const height of heights) {
                    for (const variant of variants) {
                        await setFirefoxViewport(
                            bidi,
                            tab.context,
                            width,
                            height,
                        );
                        await configureWorkspaceLayout(
                            bidi,
                            tab.context,
                            variant.key,
                            variant.zoom,
                        );
                        const state = await evaluateWorkspaceLayout(
                            bidi,
                            tab.context,
                        );
                        const label = [
                            `${width}x${height}`,
                            variant.key,
                            `${variant.zoom}%`,
                        ].join(" ");
                        assert.deepEqual(
                            state.viewport,
                            { height, width },
                            `${label}: exact viewport`,
                        );
                        assert.ok(
                            state.document.scrollWidth <= width,
                            `${label}: no document horizontal overflow`,
                        );
                        assert.ok(
                            state.document.scrollHeight <= height,
                            `${label}: no document vertical overflow`,
                        );
                        assert.ok(
                            state.workspace.left >= -1
                                && state.workspace.right <= width + 1,
                            `${label}: workspace remains horizontally visible`,
                        );
                        assert.ok(
                            state.workspace.top >= -1
                                && state.workspace.bottom <= height + 1,
                            `${label}: workspace remains viewport bounded`,
                        );
                        assertVisibleInViewport(
                            state.source,
                            height,
                            `${label}: source panel`,
                        );
                        assert.equal(
                            state.sourceScrolled,
                            true,
                            `${label}: source panel actually scrolls`,
                        );
                        assert.equal(
                            state.copyStickyVisible,
                            true,
                            `${label}: Copy prompt stays visible after scroll`,
                        );
                        assertVisibleInViewport(
                            state.preview,
                            height,
                            `${label}: preview panel`,
                        );
                        assertVisibleInViewport(
                            state.task,
                            height,
                            `${label}: task field`,
                        );
                        assertVisibleInViewport(
                            state.stage,
                            height,
                            `${label}: page stage`,
                        );
                        assert.ok(
                            state.stageSize.clientWidth > 0
                                && state.stageSize.clientHeight > 0,
                            `${label}: page stage has a viewport`,
                        );
                        assert.deepEqual(
                            state.startReachable,
                            { left: true, top: true },
                            `${label}: page start edges reachable`,
                        );
                        assert.deepEqual(
                            state.endReachable,
                            { bottom: true, right: true },
                            `${label}: page end edges reachable`,
                        );
                        assert.equal(
                            state.zoom,
                            `Preview · ${variant.zoom}%`,
                            `${label}: preview zoom`,
                        );
                        if (width <= 480) {
                            assert.deepEqual(
                                state.divider,
                                {
                                    disabled: "true",
                                    maximum: "50",
                                    minimum: "50",
                                    now: "50",
                                    tabindex: "-1",
                                },
                                `${label}: compact divider is fixed`,
                            );
                        } else {
                            assert.deepEqual(
                                state.divider,
                                {
                                    disabled: null,
                                    maximum: "65",
                                    minimum: "35",
                                    now: String(variant.share),
                                    tabindex: "0",
                                },
                                `${label}: wide divider reaches requested edge`,
                            );
                        }
                        cases += 1;
                    }
                }
            }
            assert.equal(cases, 32);
            assert.deepEqual(apiRequests, []);
        } finally {
            if (bidi?.socket != null) {
                bidi.socket.end();
            }
            if (server.listening) {
                server.close();
                await once(server, "close");
            }
            for (const child of children.reverse()) {
                await stopChild(child);
            }
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox blocks Atrament inside a hostile loopback frame",
    { skip: !FIREFOX_AVAILABLE, timeout: 30_000 },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/frame-ancestors-");
        const children = [];
        let hostile;
        let permissive;
        let bidi;
        try {
            const build = spawnSync(
                "cargo",
                [
                    "build",
                    "--quiet",
                    "-p",
                    "atrament_runtime_bootstrap",
                    "--bin",
                    "atrament",
                ],
                {
                    env: {
                        ...process.env,
                        CARGO_TARGET_DIR: TARGET_DIR,
                    },
                    stdio: "inherit",
                },
            );
            assert.equal(build.status, 0);

            const openerDirectory = path.join(profile, "bin");
            fs.mkdirSync(openerDirectory);
            const opener = path.join(openerDirectory, "xdg-open");
            fs.writeFileSync(opener, "#!/bin/sh\nexit 0\n", { mode: 0o755 });
            const runtimeEnvironment = {
                ...process.env,
                DISPLAY: ":atrament-test",
                PATH: `${openerDirectory}:${process.env.PATH ?? ""}`,
            };
            delete runtimeEnvironment.WAYLAND_DISPLAY;
            const runtime = spawn(RUNTIME_BINARY, [], {
                env: runtimeEnvironment,
                stdio: ["ignore", "pipe", "pipe"],
            });
            children.push(runtime);
            const startupRecords = [];
            const origin = await waitForLine(runtime.stdout, (line) => {
                try {
                    const record = JSON.parse(line);
                    startupRecords.push(record);
                    return record.state === "ready" ? record.origin : null;
                } catch {
                    return null;
                }
            });
            assert.deepEqual(
                startupRecords.map((record) => record.state),
                ["starting", "listening", "ready"],
            );
            assert.equal(startupRecords[0].origin, null);
            assert.equal(startupRecords[1].origin, origin);
            assert.equal(startupRecords[2].origin, origin);
            for (const record of startupRecords) {
                assert.equal(record.product, "atrament");
                assert.equal(typeof record.process_version, "string");
                assert.equal(typeof record.protocol_version, "string");
                assert.equal("secret" in record, false);
                assert.equal("session_secret" in record, false);
            }

            let resolveAtramentReferrer;
            const atramentReferrer = new Promise((resolve) => {
                resolveAtramentReferrer = resolve;
            });
            let resolveControlReferrer;
            const controlReferrer = new Promise((resolve) => {
                resolveControlReferrer = resolve;
            });

            permissive = http.createServer((_request, response) => {
                response.writeHead(200, {
                    "Cache-Control": "no-store",
                    "Content-Type": "text/html; charset=utf-8",
                });
                response.end([
                    "<!doctype html><title>Permissive frame</title>",
                    '<main class="control-frame">Allowed control</main>',
                ].join(""));
            });
            permissive.listen(0, "127.0.0.1");
            await once(permissive, "listening");
            const permissiveAddress = permissive.address();
            assert.notEqual(typeof permissiveAddress, "string");
            assert.notEqual(permissiveAddress, null);
            const permissiveOrigin =
                `http://127.0.0.1:${permissiveAddress.port}`;

            hostile = http.createServer((request, response) => {
                if (request.url === "/atrament-referrer") {
                    resolveAtramentReferrer(request.headers.referer ?? null);
                    response.end("<!doctype html><title>Recorder</title>");
                    return;
                }
                if (request.url === "/control-referrer") {
                    resolveControlReferrer(request.headers.referer ?? null);
                    response.end("<!doctype html><title>Recorder</title>");
                    return;
                }
                const target = request.url === "/control"
                    ? permissiveOrigin
                    : origin;
                response.writeHead(200, {
                    "Cache-Control": "no-store",
                    "Content-Type": "text/html; charset=utf-8",
                });
                response.end([
                    "<!doctype html><title>Hostile frame</title>",
                    `<iframe src="${target}/"></iframe>`,
                ].join(""));
            });
            hostile.listen(0, "127.0.0.1");
            await once(hostile, "listening");
            const hostileAddress = hostile.address();
            assert.notEqual(typeof hostileAddress, "string");
            assert.notEqual(hostileAddress, null);
            const hostileOrigin =
                `http://127.0.0.1:${hostileAddress.port}`;

            bidi = await startBidiFirefox(profile, children);

            const direct = await bidi.command("browsingContext.create", {
                type: "tab",
            });
            await bidi.command("browsingContext.navigate", {
                context: direct.context,
                url: origin,
                wait: "complete",
            });
            assert.deepEqual(
                await evaluateDocument(bidi, direct.context),
                {
                    hasControl: false,
                    hasWorkspace: true,
                    title: "Atrament",
                },
            );

            await navigateFromDocument(
                bidi,
                direct.context,
                `${hostileOrigin}/atrament-referrer`,
            );
            assert.equal(
                await observedWithin(
                    atramentReferrer,
                    "Atrament referrer-policy",
                ),
                null,
            );

            const referrerControl = await bidi.command(
                "browsingContext.create",
                { type: "tab" },
            );
            await bidi.command("browsingContext.navigate", {
                context: referrerControl.context,
                url: permissiveOrigin,
                wait: "complete",
            });
            await navigateFromDocument(
                bidi,
                referrerControl.context,
                `${hostileOrigin}/control-referrer`,
            );
            const ordinaryReferrer = await observedWithin(
                controlReferrer,
                "permissive control referrer",
            );
            assert.equal(ordinaryReferrer, `${permissiveOrigin}/`);

            const control = await bidi.command("browsingContext.create", {
                type: "tab",
            });
            await bidi.command("browsingContext.navigate", {
                context: control.context,
                url: `${hostileOrigin}/control`,
                wait: "complete",
            });
            const controlChild = await waitForChildContext(
                bidi,
                control.context,
            );
            assert.deepEqual(
                await waitForFramedDocument(bidi, controlChild),
                {
                    hasControl: true,
                    hasWorkspace: false,
                    title: "Permissive frame",
                },
            );

            const attacker = await bidi.command("browsingContext.create", {
                type: "tab",
            });
            await bidi.command("browsingContext.navigate", {
                context: attacker.context,
                url: hostileOrigin,
                wait: "complete",
            });
            const child = await waitForChildContext(bidi, attacker.context);
            const framed = await waitForFramedDocument(bidi, child);
            assert.equal(framed.hasControl, false);
            assert.equal(framed.hasWorkspace, false);
            assert.notEqual(framed.title, "Atrament");
        } finally {
            if (bidi?.socket != null) {
                bidi.socket.end();
            }
            for (const server of [hostile, permissive]) {
                if (server !== undefined) {
                    server.close();
                    await once(server, "close");
                }
            }
            for (const child of children.reverse()) {
                await stopChild(child);
            }
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox refresh cannot reuse the scrubbed session credential",
    { skip: !FIREFOX_AVAILABLE, timeout: 30_000 },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/session-refresh-");
        const apiRequests = [];
        const server = createSessionFixtureServer(apiRequests);
        const children = [];
        let bidi;
        try {
            server.listen(0, "127.0.0.1");
            await once(server, "listening");
            const address = server.address();
            assert.notEqual(typeof address, "string");
            assert.notEqual(address, null);
            const origin = `http://127.0.0.1:${address.port}`;

            bidi = await startBidiFirefox(profile, children);
            const tab = await bidi.command("browsingContext.create", {
                type: "tab",
            });
            await bidi.command("browsingContext.navigate", {
                context: tab.context,
                url: `${origin}/#session=${REFRESH_SECRET}`,
                wait: "complete",
            });
            const ready = await waitForSessionStatus(
                bidi,
                tab.context,
                "Session ready",
            );
            assert.equal(ready.hash, "");
            assert.deepEqual(ready.task, {
                disabled: false,
                value: REFRESH_DRAFT.task,
            });
            assert.deepEqual(ready.source, {
                disabled: false,
                value: REFRESH_DRAFT.source,
            });
            assert.deepEqual(ready.candidate, {
                disabled: false,
                value: REFRESH_DRAFT.candidate,
            });
            assert.deepEqual(apiRequests, [
                "POST /api/handshake",
                "GET /api/session/task",
                "GET /api/session/source",
                "GET /api/session/candidate",
            ]);

            await bidi.command("browsingContext.reload", {
                context: tab.context,
                wait: "complete",
            });
            const refreshed = await waitForSessionStatus(
                bidi,
                tab.context,
                "Frontend ready · credential unavailable",
            );
            assert.equal(refreshed.hash, "");
            for (const field of ["task", "source", "candidate"]) {
                assert.deepEqual(refreshed[field], {
                    disabled: true,
                    value: "",
                });
            }
            await new Promise((resolve) => setTimeout(resolve, 100));
            assert.deepEqual(apiRequests, [
                "POST /api/handshake",
                "GET /api/session/task",
                "GET /api/session/source",
                "GET /api/session/candidate",
            ]);
        } finally {
            if (bidi?.socket != null) {
                bidi.socket.end();
            }
            if (server.listening) {
                server.close();
                await once(server, "close");
            }
            for (const child of children.reverse()) {
                await stopChild(child);
            }
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox close aborts an in-flight private draft replacement",
    { skip: !FIREFOX_AVAILABLE, timeout: 30_000 },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/session-close-");
        const apiRequests = [];
        let resolveMutationStarted;
        const mutationStarted = new Promise((resolve) => {
            resolveMutationStarted = resolve;
        });
        let resolveMutationCancelled;
        const mutationCancelled = new Promise((resolve) => {
            resolveMutationCancelled = resolve;
        });
        let pendingResponse = null;
        const server = createSessionFixtureServer(
            apiRequests,
            (request, response, field) => {
                pendingResponse = response;
                resolveMutationStarted(field);
                let settled = false;
                const settle = (reason) => {
                    if (!settled) {
                        settled = true;
                        resolveMutationCancelled(reason);
                    }
                };
                request.on("aborted", () => settle("request-aborted"));
                response.on("close", () => {
                    if (!response.writableEnded) {
                        settle("response-closed");
                    }
                });
            },
        );
        const children = [];
        let bidi;
        try {
            server.listen(0, "127.0.0.1");
            await once(server, "listening");
            const address = server.address();
            assert.notEqual(typeof address, "string");
            assert.notEqual(address, null);
            const origin = `http://127.0.0.1:${address.port}`;

            bidi = await startBidiFirefox(profile, children);
            const tab = await bidi.command("browsingContext.create", {
                type: "tab",
            });
            await bidi.command("browsingContext.navigate", {
                context: tab.context,
                url: `${origin}/#session=${REFRESH_SECRET}`,
                wait: "complete",
            });
            const ready = await waitForSessionStatus(
                bidi,
                tab.context,
                "Session ready",
            );
            assert.equal(ready.task.disabled, false);

            await bidi.command("script.evaluate", {
                awaitPromise: false,
                expression: `(() => {
                    const input = document.querySelector("#task-input");
                    input.value = "close-private replacement";
                    input.dispatchEvent(new Event("input", { bubbles: true }));
                })()`,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.equal(
                await observedWithin(
                    mutationStarted,
                    "draft mutation start",
                ),
                "task",
            );
            assert.equal(
                apiRequests.at(-1),
                "POST /api/session/task",
            );

            await bidi.command("browsingContext.close", {
                context: tab.context,
            });
            assert.match(
                await observedWithin(
                    mutationCancelled,
                    "draft mutation cancellation",
                ),
                /^(request-aborted|response-closed)$/u,
            );
            const tree = await bidi.command("browsingContext.getTree", {
                maxDepth: 0,
            });
            assert.equal(
                tree.contexts.some((context) => {
                    return context.context === tab.context;
                }),
                false,
            );
            assert.deepEqual(apiRequests, [
                "POST /api/handshake",
                "GET /api/session/task",
                "GET /api/session/source",
                "GET /api/session/candidate",
                "POST /api/session/task",
            ]);
        } finally {
            if (pendingResponse !== null && !pendingResponse.destroyed) {
                pendingResponse.destroy();
            }
            if (bidi?.socket != null) {
                bidi.socket.end();
            }
            if (server.listening) {
                server.close();
                await once(server, "close");
            }
            for (const child of children.reverse()) {
                await stopChild(child);
            }
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);
