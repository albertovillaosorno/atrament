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
//   - Real-browser evidence that the localhost workspace cannot be framed by
//     another loopback origin.
// - Must-Not:
//   - Read session-private state, persist browser data, or weaken runtime
//     admission to make automation easier.
// - Allows:
//   - Inputs: One freshly started Atrament runtime and hostile loopback page.
//   - Outputs: Assertions over direct and framed Firefox document identity.
//   - Side effects: Starts disposable runtime/browser processes and one hostile
//     loopback HTTP server, then removes their temporary browser profile.
// - Split-When:
//   - More browser-enforced response policies need independent fixtures.
// - Merge-When:
//   - Browser security integration moves to one shared automation harness.
// - Summary:
//   - Proves `frame-ancestors 'none'` is enforced by Firefox, not just emitted.
// - Description:
//   - Uses Firefox's built-in WebDriver BiDi endpoint through a dependency-free
//     RFC 6455 client. A direct load must commit the Atrament document, while
//     a hostile iframe of the same URL must commit Firefox's error document.
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

function waitForLine(stream, select, timeoutMs = 15_000) {
    return new Promise((resolve, reject) => {
        let text = "";
        const timer = setTimeout(() => {
            reject(new Error(`timed out waiting for process output: ${text}`));
        }, timeoutMs);
        stream.setEncoding("utf8");
        stream.on("data", (chunk) => {
            text += chunk;
            for (const line of text.split("\n")) {
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

    command(method, params = {}) {
        const id = this.nextId;
        this.nextId += 1;
        return new Promise((resolve, reject) => {
            const timer = setTimeout(() => {
                if (this.pending.delete(id)) {
                    reject(new Error(`${method} timed out`));
                }
            }, 10_000);
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

test(
    "Firefox blocks Atrament inside a hostile loopback frame",
    { skip: !FIREFOX_AVAILABLE, timeout: 30_000 },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/frame-ancestors-");
        const children = [];
        let hostile;
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
            const origin = await waitForLine(runtime.stdout, (line) => {
                try {
                    const record = JSON.parse(line);
                    return record.state === "ready" ? record.origin : null;
                } catch {
                    return null;
                }
            });

            hostile = http.createServer((_request, response) => {
                response.writeHead(200, {
                    "Cache-Control": "no-store",
                    "Content-Type": "text/html; charset=utf-8",
                });
                response.end([
                    "<!doctype html><title>Hostile frame</title>",
                    `<iframe src="${origin}/"></iframe>`,
                ].join(""));
            });
            hostile.listen(0, "127.0.0.1");
            await once(hostile, "listening");
            const hostileAddress = hostile.address();
            assert.notEqual(typeof hostileAddress, "string");
            assert.notEqual(hostileAddress, null);
            const hostileOrigin =
                `http://127.0.0.1:${hostileAddress.port}`;

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
            bidi = new BidiClient(`${remote}/session`);
            await bidi.connect();
            await bidi.command("session.new", {
                capabilities: { alwaysMatch: {} },
            });

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
                    hasWorkspace: true,
                    title: "Atrament",
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
            assert.equal(framed.hasWorkspace, false);
            assert.notEqual(framed.title, "Atrament");
        } finally {
            if (bidi?.socket != null) {
                bidi.socket.end();
            }
            if (hostile !== undefined) {
                hostile.close();
                await once(hostile, "close");
            }
            for (const child of children.reverse()) {
                await stopChild(child);
            }
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);
