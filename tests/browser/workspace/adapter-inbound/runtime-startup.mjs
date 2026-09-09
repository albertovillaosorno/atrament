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
//   - Process-level regression evidence for fail-closed runtime startup.
// - Must-Not:
//   - Launch a real browser, persist session state, or expose credentials.
// - Allows:
//   - Inputs: The built Atrament binary and one deterministic failing opener.
//   - Outputs: Assertions over startup records and secret-free launch failure.
//   - Side effects: Repository-local build/cache and temporary fixture files.
// - Split-When:
//   - Startup lifecycle gains multiple independently executable process modes.
// - Merge-When:
//   - Runtime startup process evidence moves into another acceptance harness.
// - Summary:
//   - Proves browser-launch failure never publishes a ready Atrament session.
// - Description:
//   - Runs a fake failing xdg-open against one fresh loopback process startup.
// - Usage:
//   - Execute through the repository frontend test script.
// - Defaults:
//   - The fixture removes its repository-local temporary opener directory.
//
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
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
