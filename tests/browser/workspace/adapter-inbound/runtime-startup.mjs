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

function waitForReady(child) {
    return new Promise((resolve, reject) => {
        let buffered = "";
        let receivedBytes = 0;
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
                if (record === null || typeof record !== "object"
                    || Array.isArray(record)) {
                    finish(new Error("invalid runtime readiness record"));
                    return;
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
    child.stdout.write('{"state":"sta');
    child.stdout.write('rting"}\n{"state":"rea');
    child.stdout.write('dy","origin":"http://127.0.0.1:1234"}\n');
    const ready = await pending;
    assert.deepEqual(ready, {
        state: "ready", origin: "http://127.0.0.1:1234",
    });
    assert.equal(child.stdout.listenerCount("data"), 0);
    assert.equal(child.listenerCount("exit"), 0);
    child.stdout.destroy();
});

test(
    "startup readiness decodes fragmented UTF-8 without substitution",
    async () => {
        const child = fixtureReadinessChild();
        const pending = waitForReady(child);
        const expected = { state: "ready", marker: "á👩‍🔬" };
        const bytes = Buffer.from(`${JSON.stringify(expected)}\n`);
        const split = bytes.indexOf(Buffer.from("á"));
        assert.ok(split > 0);
        child.stdout.write(bytes.subarray(0, split + 1));
        child.stdout.write(bytes.subarray(split + 1));
        assert.deepEqual(await pending, expected);
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

async function terminateChild(child) {
    if (child.exitCode !== null || child.signalCode !== null) {
        return;
    }
    const exited = new Promise((resolve) => child.once("exit", resolve));
    child.kill("SIGKILL");
    await exited;
}

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
