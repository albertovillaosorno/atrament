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
//   - Process-level regression evidence for fail-closed runtime startup and
//     live Linux socket confinement.
// - Must-Not:
//   - Launch a real browser, persist session state, or expose credentials.
// - Allows:
//   - Inputs: The built Atrament binary and deterministic fake openers.
//   - Outputs: Assertions over startup records, secret-free launch failure, and
//     live Linux Internet-socket confinement.
//   - Side effects: Repository-local build/cache and temporary fixture files.
// - Split-When:
//   - Startup lifecycle gains multiple independently executable process modes.
// - Merge-When:
//   - Runtime startup process evidence moves into another acceptance harness.
// - Summary:
//   - Proves browser-launch failure never publishes a ready Atrament session.
// - Description:
//   - Runs deterministic fake xdg-open helpers against fresh loopback process
//     startup.
// - Usage:
//   - Execute through the repository frontend test script.
// - Defaults:
//   - The fixture removes its repository-local temporary opener directory.
//   - Live socket inspection is skipped outside Linux.
//
import assert from "node:assert/strict";
import { spawn, spawnSync } from "node:child_process";
import { EventEmitter } from "node:events";
import fs from "node:fs";
import { PassThrough } from "node:stream";
import path from "node:path";
import test from "node:test";

const PINNED_CARGO = path.resolve(
    ".dependencies/rust/1.97.1/bin/cargo",
);
const TARGET_DIR = path.resolve(".cache/cargo-target");
const RUNTIME_BINARY = path.join(TARGET_DIR, "debug", "atrament");

function buildRuntime() {
    const build = spawnSync(
        PINNED_CARGO,
        [
            "build",
            "--quiet",
            "-p",
            "atrament_runtime_bootstrap",
            "--bin",
            "atrament",
        ],
        {
            env: { ...process.env, CARGO_TARGET_DIR: TARGET_DIR },
            stdio: "inherit",
        },
    );
    assert.equal(build.status, 0);
}

// The child process is not an unlimited or trusted source of readiness
// records. Bound its complete UTF-8 output and fail without quoting it.
const MAX_STARTUP_OUTPUT_BYTES = 65_536;
const STARTUP_STATES = ["starting", "listening", "ready"];
const STARTUP_FIELDS = [
    "origin", "process_version", "product", "protocol_version", "state",
];

function canonicalStartupOrigin(value) {
    if (typeof value !== "string") {
        return false;
    }
    const port = /^http:\/\/127\.0\.0\.1:([1-9][0-9]{0,4})$/u.exec(value);
    return port !== null && Number(port[1]) <= 65_535;
}

function validStartupRecord(record, phase, previousOrigin) {
    return record !== null
        && typeof record === "object"
        && !Array.isArray(record)
        && Object.keys(record).length === STARTUP_FIELDS.length
        && STARTUP_FIELDS.every((field) => Object.hasOwn(record, field))
        && record.product === "atrament"
        && record.process_version === "0.1.0"
        && record.protocol_version === "atrament.runtime/1"
        && record.state === STARTUP_STATES[phase]
        && (phase === 0 ? record.origin === null
            : canonicalStartupOrigin(record.origin))
        && (phase !== 2 || record.origin === previousOrigin);
}

function waitForReady(child) {
    return new Promise((resolve, reject) => {
        let buffered = "";
        let receivedBytes = 0;
        let nextPhase = 0;
        let listeningOrigin = null;
        let settled = false;
        const decoder = new TextDecoder("utf-8", { fatal: true });
        const cleanup = () => {
            clearTimeout(timeout);
            child.stdout.off("data", onData);
            child.stdout.off("error", onError);
            child.stdout.off("end", onEnd);
            child.stdout.off("close", onEnd);
            child.off("exit", onExit);
        };
        const finish = (error, record) => {
            if (settled) {
                return;
            }
            settled = true;
            cleanup();
            if (error !== null) {
                reject(error);
            } else {
                resolve(record);
            }
        };
        const onExit = (code, signal) => finish(new Error(
            `runtime exited before ready: ${code}/${signal}`,
        ));
        const onError = () => finish(new Error(
            "runtime startup output stream failed",
        ));
        const onEnd = () => finish(new Error(
            "runtime startup output ended before ready",
        ));
        const onData = (chunk) => {
            const bytes = Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk);
            if (bytes.length > MAX_STARTUP_OUTPUT_BYTES - receivedBytes) {
                finish(new Error("startup output exceeds byte budget"));
                return;
            }
            receivedBytes += bytes.length;
            try {
                buffered += decoder.decode(bytes, { stream: true });
            } catch {
                finish(new Error("invalid runtime readiness encoding"));
                return;
            }
            const lines = buffered.split("\n");
            buffered = lines.pop() ?? "";
            for (const line of lines) {
                if (line === "") {
                    continue;
                }
                let record;
                try {
                    record = JSON.parse(line);
                } catch {
                    finish(new Error("invalid runtime readiness record"));
                    return;
                }
                if (!validStartupRecord(record, nextPhase, listeningOrigin)) {
                    finish(new Error("invalid runtime readiness record"));
                    return;
                }
                nextPhase += 1;
                if (record.state === "listening") {
                    listeningOrigin = record.origin;
                }
                if (record.state === "ready") {
                    finish(null, record);
                    return;
                }
            }
        };
        const timeout = setTimeout(() => finish(new Error(
            "runtime did not publish ready state",
        )), 2_000);
        child.stdout.on("data", onData);
        child.stdout.once("error", onError);
        child.stdout.once("end", onEnd);
        child.stdout.once("close", onEnd);
        child.once("exit", onExit);
    });
}

function fixtureReadinessChild() {
    const child = new EventEmitter();
    child.stdout = new PassThrough();
    return child;
}

test("startup readiness bounds untrusted UTF-8 process output", async () => {
    const child = fixtureReadinessChild();
    const pending = waitForReady(child);
    child.stdout.write("á".repeat(32_769));
    await assert.rejects(pending, /startup output exceeds byte budget/);
    assert.equal(child.stdout.listenerCount("data"), 0);
    assert.equal(child.listenerCount("exit"), 0);
    child.stdout.destroy();
});

test(
    "startup readiness refuses malformed output without throwing",
    async () => {
        const child = fixtureReadinessChild();
        const pending = waitForReady(child);
        let thrown = null;
        try {
            child.stdout.write("private-marker-invalid-json\n");
        } catch (error) {
            thrown = error;
        }
        child.emit("exit", 17, null);
        assert.equal(thrown, null,
            "malformed records cannot escape data handlers");
        await assert.rejects(pending, /invalid runtime readiness record/);
        assert.equal(child.stdout.listenerCount("data"), 0);
        child.stdout.destroy();
    },
);

test(
    "startup readiness rejects partial output without reflecting it",
    async () => {
        const child = fixtureReadinessChild();
        const pending = waitForReady(child);
        child.stdout.end("private-marker");
        await assert.rejects(pending, (error) => {
            assert.match(error.message, /output ended before ready/);
            assert.equal(error.message.includes("private-marker"), false);
            return true;
        });
        assert.equal(child.stdout.listenerCount("data"), 0);
        assert.equal(child.listenerCount("exit"), 0);
    },
);

test("startup readiness accepts fragmented finite JSON output", async () => {
    const child = fixtureReadinessChild();
    const pending = waitForReady(child);
    const startup = [
        startupRecord("starting"),
        startupRecord("listening", "http://127.0.0.1:1234"),
        startupRecord("ready", "http://127.0.0.1:1234"),
    ];
    const bytes = startup.map((record) => JSON.stringify(record))
        .join("\n") + "\n";
    const split = bytes.indexOf('"ready"') + 3;
    child.stdout.write(bytes.slice(0, split));
    child.stdout.write(bytes.slice(split));
    const ready = await pending;
    assert.deepEqual(ready, startup[2]);
    assert.equal(child.stdout.listenerCount("data"), 0);
    assert.equal(child.listenerCount("exit"), 0);
    child.stdout.destroy();
});

test(
    "startup readiness rejects unexpected UTF-8 fields and invalid bytes",
    async () => {
        const child = fixtureReadinessChild();
        const pending = waitForReady(child);
        const unexpected = { ...startupRecord("starting"), marker: "á👩‍🔬" };
        const bytes = Buffer.from(`${JSON.stringify(unexpected)}\n`);
        const split = bytes.indexOf(Buffer.from("á"));
        assert.ok(split > 0);
        child.stdout.write(bytes.subarray(0, split + 1));
        child.stdout.write(bytes.subarray(split + 1));
        await assert.rejects(pending, /invalid runtime readiness record/);
        child.stdout.destroy();

        const invalid = fixtureReadinessChild();
        const rejected = waitForReady(invalid);
        invalid.stdout.write(Buffer.concat([
            Buffer.from('{"state":"ready","marker":"'),
            Buffer.from([0xff]), Buffer.from('"}\n'),
        ]));
        await assert.rejects(rejected, /invalid runtime readiness encoding/);
        invalid.stdout.destroy();
    },
);

function startupRecord(state, origin = null) {
    return {
        product: "atrament",
        process_version: "0.1.0",
        protocol_version: "atrament.runtime/1",
        origin,
        state,
    };
}

test("startup readiness requires ordered secret-free loopback records",
    async () => {
        const starting = startupRecord("starting");
        const listening = startupRecord("listening",
            "http://127.0.0.1:1234");
        const ready = startupRecord("ready", listening.origin);
        const rejectedCases = [
            [ready],
            [starting, ready],
            [starting, listening, { ...ready, origin: "http://example.com" }],
            [starting, listening, {
                ...ready, origin: "http://127.0.0.1:1235",
            }],
            [starting, { ...listening, origin: "http://127.0.0.1:65536" },
                ready],
            [starting, listening, { ...ready, product: "another-product" }],
            [starting, listening, { ...ready, process_version: "" }],
            [starting, listening, { ...ready, protocol_version: "unknown" }],
            [starting, listening, {
                ...ready, session_secret: "secret-marker",
            }],
            [starting, { ...listening, secret: "secret-marker" }, ready],
            [starting, { ...listening, state: "starting" }, ready],
            [starting, listening, listening, ready],
        ];
        for (const records of rejectedCases) {
            const child = fixtureReadinessChild();
            const pending = waitForReady(child);
            child.stdout.end(records.map((record) =>
                JSON.stringify(record)).join("\n") + "\n");
            await assert.rejects(pending, /invalid runtime readiness record/,
                JSON.stringify(records));
            assert.equal(child.stdout.listenerCount("data"), 0);
            assert.equal(child.listenerCount("exit"), 0);
        }
    },
);

function processSocketInodes(pid) {
    const descriptors = fs.readdirSync(`/proc/${pid}/fd`);
    const inodes = [];
    for (const descriptor of descriptors) {
        let target;
        try {
            target = fs.readlinkSync(`/proc/${pid}/fd/${descriptor}`);
        } catch (error) {
            if (error.code === "ENOENT") {
                continue;
            }
            throw error;
        }
        const match = /^socket:\[(\d+)\]$/u.exec(target);
        if (match !== null) {
            inodes.push(match[1]);
        }
    }
    return inodes.sort();
}

function processInternetSocketRows(pid, inodes) {
    const wanted = new Set(inodes);
    const rows = [];
    for (const protocol of ["tcp", "tcp6", "udp", "udp6"]) {
        const tablePath = `/proc/${pid}/net/${protocol}`;
        if (!fs.existsSync(tablePath)) {
            continue;
        }
        const table = fs.readFileSync(tablePath, "utf8");
        for (const line of table.trim().split("\n").slice(1)) {
            const fields = line.trim().split(/\s+/u);
            if (fields.length < 10 || !wanted.has(fields[9])) {
                continue;
            }
            rows.push({
                inode: fields[9],
                local: fields[1],
                protocol,
                remote: fields[2],
                state: fields[3],
            });
        }
    }
    return rows;
}

function terminateChild(child, forcedMs = 10_000) {
    if (child.exitCode !== null || child.signalCode !== null) {
        return Promise.resolve();
    }
    return new Promise((resolve, reject) => {
        let settled = false;
        const finish = (error = null) => {
            if (settled) {
                return;
            }
            settled = true;
            clearTimeout(timer);
            child.off("exit", onExit);
            child.off("error", onError);
            if (error === null) {
                resolve();
            } else {
                reject(error);
            }
        };
        const onExit = () => finish();
        const onError = (error) => finish(error);
        const timer = setTimeout(() => finish(new Error(
            "disposable runtime child did not exit after SIGKILL",
        )), forcedMs);
        child.once("exit", onExit);
        child.once("error", onError);
        try {
            if (!child.kill("SIGKILL") && !settled) {
                finish(new Error("disposable runtime child SIGKILL failed"));
            }
        } catch (error) {
            finish(error);
        }
    });
}

function fixtureTerminationChild(kill) {
    const child = new EventEmitter();
    child.exitCode = null;
    child.signalCode = null;
    child.kill = kill;
    return child;
}

// An unresponsive fake process must not hang the test runner indefinitely.
async function shutdownWithin(child, forcedMs = 20) {
    let timer;
    try {
        return await Promise.race([
            terminateChild(child, forcedMs),
            new Promise((_resolve, reject) => {
                timer = setTimeout(() => reject(new Error(
                    "fixture deadline elapsed before child shutdown",
                )), 120);
            }),
        ]);
    } finally {
        clearTimeout(timer);
    }
}

test("runtime startup child shutdown has a bounded SIGKILL reap", async () => {
    const signals = [];
    const child = fixtureTerminationChild((signal) => {
        signals.push(signal);
        return true;
    });
    await assert.rejects(shutdownWithin(child),
        /disposable runtime child did not exit after SIGKILL/);
    assert.deepEqual(signals, ["SIGKILL"]);
    assert.equal(child.listenerCount("exit"), 0);
    assert.equal(child.listenerCount("error"), 0);
});

test("runtime startup child shutdown cleans observers if kill throws",
    async () => {
        const failure = new Error("synthetic SIGKILL failure");
        const child = fixtureTerminationChild(() => { throw failure; });
        await assert.rejects(terminateChild(child),
            (error) => error === failure);
        assert.equal(child.listenerCount("exit"), 0);
        assert.equal(child.listenerCount("error"), 0);
    });

test("runtime startup child shutdown rejects an unsuccessful kill",
    async () => {
        const child = fixtureTerminationChild(() => false);
        await assert.rejects(shutdownWithin(child),
            /disposable runtime child SIGKILL failed/);
        assert.equal(child.listenerCount("exit"), 0);
    });

test("runtime startup child shutdown observes a delayed SIGKILL exit",
    async () => {
        let child;
        child = fixtureTerminationChild((signal) => {
            assert.equal(signal, "SIGKILL");
            setTimeout(() => {
                child.signalCode = signal;
                child.emit("exit", null, signal);
            }, 30);
            return true;
        });
        await terminateChild(child, 100);
        assert.equal(child.listenerCount("exit"), 0);
        assert.equal(child.listenerCount("error"), 0);
    });

test("failed browser launch never publishes runtime ready", () => {
    buildRuntime();
    fs.mkdirSync(".temp", { recursive: true });
    const fixture = fs.mkdtempSync(".temp/startup-failure-");
    try {
        const openerDirectory = path.join(fixture, "bin");
        fs.mkdirSync(openerDirectory);
        const opener = path.join(openerDirectory, "xdg-open");
        fs.writeFileSync(opener, "#!/bin/sh\nexit 17\n", { mode: 0o755 });
        const environment = {
            ...process.env,
            DISPLAY: ":atrament-test",
            PATH: `${openerDirectory}:${process.env.PATH ?? ""}`,
        };
        delete environment.WAYLAND_DISPLAY;
        const result = spawnSync(RUNTIME_BINARY, [], {
            encoding: "utf8",
            env: environment,
            timeout: 5_000,
        });
        assert.notEqual(result.status, 0);
        assert.equal(result.signal, null);
        assert.equal(result.error, undefined);
        const records = result.stdout
            .trim()
            .split("\n")
            .filter((line) => line !== "")
            .map((line) => JSON.parse(line));
        assert.deepEqual(
            records.map((record) => record.state),
            ["starting", "listening"],
        );
        assert.equal(records[0].origin, null);
        assert.match(records[1].origin, /^http:\/\/127\.0\.0\.1:\d+$/u);
        for (const record of records) {
            assert.equal(record.product, "atrament");
            assert.equal(typeof record.process_version, "string");
            assert.equal(typeof record.protocol_version, "string");
            assert.equal("secret" in record, false);
            assert.equal("session_secret" in record, false);
        }
        assert.match(result.stderr, /browser launch failed/u);
        assert.match(result.stderr, /restart Atrament/u);
        assert.equal(result.stderr.includes("#session="), false);
        assert.equal(result.stderr.includes(records[1].origin), false);
    } finally {
        fs.rmSync(fixture, { recursive: true, force: true });
    }
});

test(
    "empty Linux display marker fails before opener can publish ready",
    { skip: process.platform !== "linux" },
    () => {
        buildRuntime();
        fs.mkdirSync(".temp", { recursive: true });
        const fixture = fs.mkdtempSync(".temp/startup-empty-display-");
        try {
            const openerDirectory = path.join(fixture, "bin");
            fs.mkdirSync(openerDirectory);
            const opener = path.join(openerDirectory, "xdg-open");
            fs.writeFileSync(opener, "#!/bin/sh\nexit 0\n", { mode: 0o755 });
            const environment = {
                ...process.env,
                DISPLAY: "",
                PATH: `${openerDirectory}:${process.env.PATH ?? ""}`,
            };
            delete environment.WAYLAND_DISPLAY;
            const result = spawnSync(RUNTIME_BINARY, [], {
                encoding: "utf8",
                env: environment,
                timeout: 2_000,
            });
            assert.notEqual(result.status, 0);
            assert.equal(result.signal, null);
            assert.equal(result.error, undefined);
            const records = result.stdout
                .trim()
                .split("\n")
                .filter((line) => line !== "")
                .map((line) => JSON.parse(line));
            assert.deepEqual(
                records.map((record) => record.state),
                ["starting", "listening"],
            );
            assert.match(result.stderr, /no graphical session/u);
            assert.equal(result.stderr.includes("#session="), false);
            assert.equal(result.stderr.includes(records[1].origin), false);
        } finally {
            fs.rmSync(fixture, { recursive: true, force: true });
        }
    },
);

test(
    "live Linux runtime owns only its published loopback listener",
    { skip: process.platform !== "linux" },
    async () => {
        buildRuntime();
        fs.mkdirSync(".temp", { recursive: true });
        const fixture = fs.mkdtempSync(".temp/startup-network-");
        let child = null;
        try {
            const openerDirectory = path.join(fixture, "bin");
            fs.mkdirSync(openerDirectory);
            const opener = path.join(openerDirectory, "xdg-open");
            fs.writeFileSync(opener, "#!/bin/sh\nexit 0\n", { mode: 0o755 });
            const environment = {
                ...process.env,
                DISPLAY: ":atrament-test",
                PATH: `${openerDirectory}:${process.env.PATH ?? ""}`,
            };
            delete environment.WAYLAND_DISPLAY;
            child = spawn(RUNTIME_BINARY, [], {
                env: environment,
                stdio: ["ignore", "pipe", "pipe"],
            });
            const ready = await waitForReady(child);
            const socketInodes = processSocketInodes(child.pid);
            const rows = processInternetSocketRows(
                child.pid,
                socketInodes,
            );
            assert.equal(rows.length, 1);
            assert.equal(socketInodes.includes(rows[0].inode), true);
            const origin = new URL(ready.origin);
            assert.equal(origin.hostname, "127.0.0.1");
            const expectedPort = Number.parseInt(origin.port, 10)
                .toString(16)
                .toUpperCase()
                .padStart(4, "0");
            const { inode: _inode, ...surface } = rows[0];
            assert.deepEqual(surface, {
                local: `0100007F:${expectedPort}`,
                protocol: "tcp",
                remote: "00000000:0000",
                state: "0A",
            });
        } finally {
            if (child !== null) {
                await terminateChild(child);
            }
            fs.rmSync(fixture, { recursive: true, force: true });
        }
    },
);
