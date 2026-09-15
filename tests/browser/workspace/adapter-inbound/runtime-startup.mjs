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
import fs from "node:fs";
import path from "node:path";
import test from "node:test";

const TARGET_DIR = path.resolve(".cache/cargo-target");
const RUNTIME_BINARY = path.join(TARGET_DIR, "debug", "atrament");

function buildRuntime() {
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
            env: { ...process.env, CARGO_TARGET_DIR: TARGET_DIR },
            stdio: "inherit",
        },
    );
    assert.equal(build.status, 0);
}

function waitForReady(child) {
    return new Promise((resolve, reject) => {
        let buffered = "";
        const timeout = setTimeout(() => {
            cleanup();
            reject(new Error("runtime did not publish ready state"));
        }, 2_000);
        const cleanup = () => {
            clearTimeout(timeout);
            child.stdout.off("data", onData);
            child.off("exit", onExit);
        };
        const onExit = (code, signal) => {
            cleanup();
            reject(new Error(
                `runtime exited before ready: ${code}/${signal}`,
            ));
        };
        const onData = (chunk) => {
            buffered += chunk.toString("utf8");
            const lines = buffered.split("\n");
            buffered = lines.pop() ?? "";
            for (const line of lines) {
                if (line === "") {
                    continue;
                }
                const record = JSON.parse(line);
                if (record.state === "ready") {
                    cleanup();
                    resolve(record);
                    return;
                }
            }
        };
        child.stdout.on("data", onData);
        child.once("exit", onExit);
    });
}

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
