// Copyright:
//   - Copyright © 2026 Alberto Villa Osorno.
// SPDX-License-Identifier:
//   - LicenseRef-Proprietary
// Confidential:
//   - true
// License-File:
//   - LICENSE
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
//     text, framing, draft coalescing, network egress, fragment-scrubbed
//     refresh, and close-time cancellation.
// - Usage:
//   - Run through `pnpm test:frontend` from the repository root.
// - Defaults:
//   - Skips only when Firefox is unavailable on the host.
//
import assert from "node:assert/strict";
import crypto from "node:crypto";
import { spawn, spawnSync } from "node:child_process";
import { EventEmitter, once } from "node:events";
import fs from "node:fs";
import http from "node:http";
import net from "node:net";
import path from "node:path";
import { PassThrough } from "node:stream";
import test from "node:test";
import { inflateSync } from "node:zlib";
const { geometricStarter, encodeGlyphLibrary,
    fillMissingBilingualBaseline, italicizeGlyphStrokes, scaleGlyphHeight,
    scaleGlyphWidth, scaleGlyphInk,
    translateGlyphStrokes } = await import(new URL(
    "../../../../src/browser/workspace/adapter-inbound/generated/"
        + "glyph-strokes.js", import.meta.url,
).href);
import { runInNewContext } from "node:vm";

const FIREFOX_AVAILABLE = process.platform === "linux"
    && spawnSync(
        "firefox",
        ["--version"],
        { stdio: "ignore" },
    ).status === 0;
const PINNED_CARGO = path.resolve(
    ".dependencies/rust/1.97.1/bin/cargo",
);
const TARGET_DIR = path.resolve(".cache/cargo-target");
const RUNTIME_BINARY = path.join(TARGET_DIR, "debug", "atrament");
const WORKSPACE_DIR = path.resolve(
    "src/browser/workspace/adapter-inbound",
);
// A Firefox automation result may exceed a megabyte, but not an unbounded
// length advertised by a malformed remote WebSocket peer.
// Headless Firefox can take longer to expose its debugging endpoint under
// workstation memory pressure. Both deadlines remain finite and test-owned.
const FIREFOX_BIDI_READY_TIMEOUT_MS = 25_000;
const FIREFOX_CASE_TIMEOUT_MS = 50_000;
const MAX_BIDI_FRAME_BYTES = 8 * 1024 * 1024;
const MAX_BIDI_FRAGMENTS = 1_024;
const MAX_BIDI_EVENTS = 16;
// WebDriver error descriptions, stacks and optional data may contain text
// entered in the private page. Only published protocol error categories may
// leave the automation peer as test error messages.
const SAFE_BIDI_ERROR_CODES = new Set([
    "invalid argument",
    "invalid selector",
    "invalid session id",
    "javascript error",
    "move target out of bounds",
    "no such alert",
    "no such element",
    "no such frame",
    "no such handle",
    "no such history entry",
    "no such intercept",
    "no such node",
    "no such request",
    "no such script",
    "no such shadow root",
    "no such user context",
    "no such window",
    "session not created",
    "timeout",
    "unable to capture screen",
    "unable to set cookie",
    "unknown command",
    "unknown error",
    "unsupported operation",
]);
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

// Readiness logs are untrusted process output. Never retain unbounded lines
// or reflect their contents through a test failure's diagnostics.
const MAX_READINESS_OUTPUT_BYTES = 65_536;

function waitForLine(stream, select, timeoutMs = 15_000, child = null) {
    return new Promise((resolve, reject) => {
        let buffered = "";
        let observedBytes = 0;
        const decoder = new TextDecoder("utf-8", { fatal: true });
        const finish = (error, value) => {
            clearTimeout(timer);
            stream.off("data", onData);
            stream.off("error", onError);
            stream.off("end", onEnd);
            stream.off("close", onEnd);
            child?.off("error", onChildError);
            if (error === null) {
                resolve(value);
            } else {
                reject(error);
            }
        };
        // Readiness errors can embed private process output. Keep diagnostic
        // messages fixed instead of propagating untrusted exception text.
        const onError = () => finish(new Error(
            "process readiness stream failed",
        ));
        // A child may fail before its stdout/stderr stream closes. Observe
        // the process event without propagating its private spawn metadata.
        const onChildError = () => finish(new Error(
            "readiness child process failed",
        ));
        const onEnd = () => finish(new Error(
            "process output ended before readiness",
        ));
        const onData = (bytes) => {
            // A string chunk may already contain replacement characters.
            // Only raw process bytes can establish unmodified readiness.
            if (!Buffer.isBuffer(bytes)) {
                finish(new Error("invalid readiness output encoding"));
                return;
            }
            if (bytes.length > MAX_READINESS_OUTPUT_BYTES - observedBytes) {
                finish(new Error("readiness output exceeds byte budget"));
                return;
            }
            observedBytes += bytes.length;
            try {
                buffered += decoder.decode(bytes, { stream: true });
            } catch {
                finish(new Error("invalid readiness output encoding"));
                return;
            }
            const lines = buffered.split("\n");
            buffered = lines.pop() ?? "";
            for (const line of lines) {
                let selected;
                try {
                    selected = select(line);
                } catch {
                    finish(new Error("invalid readiness record"));
                    return;
                }
                if (selected === null || selected === undefined) {
                    continue;
                }
                // Readiness call sites select endpoint strings, not truthy
                // values or arbitrary objects from untrusted process output.
                if (typeof selected !== "string" || selected.length === 0) {
                    finish(new Error("invalid readiness selector result"));
                    return;
                }
                finish(null, selected);
                return;
            }
        };
        const timer = setTimeout(() => finish(new Error(
            "timed out waiting for process output",
        )), timeoutMs);
        stream.on("data", onData);
        stream.once("error", onError);
        stream.once("end", onEnd);
        stream.once("close", onEnd);
        child?.once("error", onChildError);
    });
}

// Retain the process groups of disposable Firefox fixtures while this test
// worker runs. A worker that exits before reaching a test's finally block
// must not leave its detached browsers and helper processes behind.
const liveFirefoxFixtureGroups = new Set();
process.once("exit", () => {
    for (const pid of liveFirefoxFixtureGroups) {
        try { process.kill(-pid, "SIGKILL"); } catch { /* best effort */ }
    }
});

// A Node worker killed with SIGKILL cannot execute an exit listener. Keep a
// separate, test-owned watchdog outside the worker's process group. A closed
// parent pipe reaps exactly the Firefox fixture's detached group. It does not
// inspect or signal unrelated browser instances or the graphical session.
function watchFirefoxFixtureGroup(child) {
    if (process.platform !== "linux" || child.fixtureProcessGroup !== true
        || !Number.isSafeInteger(child.pid) || child.pid <= 1) return null;
    const supervisor = [
        "const pgid = Number(process.argv[1]);",
        "if (!Number.isSafeInteger(pgid) || pgid <= 1) process.exit(2);",
        "let stopped = false;",
        "const reap = () => {",
        "  if (stopped) return; stopped = true;",
        "  try { process.kill(-pgid, 'SIGKILL'); } catch {}",
        "  process.exit(0);",
        "};",
        "process.stdin.once('end', reap);",
        "process.stdin.once('error', reap);",
        "process.stdin.resume();",
    ].join("\n");
    const watcher = spawn(process.execPath, ["-e", supervisor,
        String(child.pid)], {
        detached: true,
        stdio: ["pipe", "ignore", "ignore"],
    });
    // Keep a failed watchdog from crashing the test worker; the original
    // synchronous exit handler remains a best-effort fallback.
    watcher.once("error", () => {});
    watcher.stdin.on("error", () => {});
    watcher.unref();
    child.once("exit", () => watcher.stdin.end());
    return watcher;
}

function trackFirefoxFixtureGroup(child) {
    if (process.platform !== "linux" || child.fixtureProcessGroup !== true
        || !Number.isSafeInteger(child.pid) || child.pid <= 1) return;
    liveFirefoxFixtureGroups.add(child.pid);
    watchFirefoxFixtureGroup(child);
    // The launcher can exit or crash without its helpers exiting. Reap them
    // immediately instead of waiting for stopChild's normal teardown path.
    child.once("exit", () => {
        try { signalOwnedFixtureGroup(child, "SIGKILL"); }
        catch { /* a subsequent teardown can retry */ }
        finally { liveFirefoxFixtureGroups.delete(child.pid); }
    });
}

test("Linux Firefox watchdog reaps its fixture after worker pipe loss",
    { skip: process.platform !== "linux", timeout: 5_000 },
    async () => {
        const fixture = spawn(process.execPath,
            ["-e", "setInterval(() => {}, 1000)"], {
                detached: true,
                stdio: "ignore",
            });
        fixture.fixtureProcessGroup = true;
        const exited = once(fixture, "exit");
        try {
            const watcher = watchFirefoxFixtureGroup(fixture);
            assert.ok(watcher, "Linux requires a dedicated fixture watchdog");
            // Simulate sudden loss of the test worker's open pipe. The
            // watchdog runs in a separate group and must reap the fixture.
            watcher.stdin.end();
            await Promise.race([
                exited,
                new Promise((_resolve, reject) => setTimeout(() => reject(
                    new Error("Firefox fixture survived worker pipe closure")),
                1_000)),
            ]);
        } finally {
            try { process.kill(-fixture.pid, "SIGKILL"); } catch {}
        }
    },
);

test("Linux Firefox watchdog refuses unowned process groups", () => {
    assert.equal(watchFirefoxFixtureGroup({
        pid: process.pid,
        fixtureProcessGroup: false,
    }), null);
    assert.equal(watchFirefoxFixtureGroup({
        pid: 0,
        fixtureProcessGroup: true,
    }), null);
});

// The watcher must survive a worker SIGKILL, not merely an orderly pipe end.
// Spawn only disposable Node fixtures so the test never touches user Firefox.
test("Linux Firefox watchdog reaps fixture after abrupt worker SIGKILL",
    { skip: process.platform !== "linux", timeout: 8_000 },
    async () => {
        const workerSource = [
            "const { spawn } = require('node:child_process');",
            `const watchFirefoxFixtureGroup = ${watchFirefoxFixtureGroup};`,
            "const fixture = spawn(process.execPath,",
            "  ['-e', 'setInterval(() => {}, 1000)'],",
            "  { detached: true, stdio: 'ignore' });",
            "fixture.fixtureProcessGroup = true;",
            "watchFirefoxFixtureGroup(fixture);",
            "process.stdout.write(String(fixture.pid) + '\\n');",
            "setInterval(() => {}, 1000);",
        ].join("\n");
        const worker = spawn(process.execPath, ["-e", workerSource], {
            stdio: ["ignore", "pipe", "ignore"],
        });
        let fixturePid;
        try {
            fixturePid = Number(await waitForLine(worker.stdout,
                (line) => /^\d+$/u.test(line) ? line : null,
                2_000, worker));
            assert.ok(Number.isSafeInteger(fixturePid) && fixturePid > 1);
            assert.equal(worker.kill("SIGKILL"), true);
            await once(worker, "exit");
            const deadline = Date.now() + 2_000;
            let running = true;
            while (Date.now() < deadline) {
                try {
                    const stat = fs.readFileSync(
                        `/proc/${fixturePid}/stat`, "utf8");
                    const state = stat[stat.lastIndexOf(") ") + 2];
                    running = !["Z", "X", "x"].includes(state);
                } catch (error) {
                    if (error.code !== "ENOENT") throw error;
                    running = false;
                }
                if (!running) break;
                await new Promise((resolve) => setTimeout(resolve, 25));
            }
            assert.equal(running, false,
                "worker SIGKILL must trigger its independent group watchdog");
        } finally {
            try { worker.kill("SIGKILL"); } catch {}
            if (Number.isSafeInteger(fixturePid) && fixturePid > 1) {
                try { process.kill(-fixturePid, "SIGKILL"); } catch {}
            }
        }
    },
);

// Detached fixture supervision must not keep a finished worker alive. A
// child-process handle is unref'd here so only the watchdog pipe remains.
test("Linux Firefox watchdog permits normal worker exit without a child exit",
    { skip: process.platform !== "linux", timeout: 5_000 },
    async () => {
        const source = [
            "const { spawn } = require('node:child_process');",
            `const watchFirefoxFixtureGroup = ${watchFirefoxFixtureGroup};`,
            "const fixture = spawn(process.execPath,",
            "  ['-e', 'setInterval(() => {}, 1000)'],",
            "  { detached: true, stdio: 'ignore' });",
            "fixture.fixtureProcessGroup = true;",
            "fixture.unref();",
            "watchFirefoxFixtureGroup(fixture);",
            "process.stdout.write(String(fixture.pid) + '\\n');",
        ].join("\n");
        const worker = spawn(process.execPath, ["-e", source], {
            stdio: ["ignore", "pipe", "ignore"],
        });
        let fixturePid;
        let timer;
        try {
            fixturePid = Number(await waitForLine(worker.stdout,
                (line) => /^\d+$/u.test(line) ? line : null,
                2_000, worker));
            assert.ok(Number.isSafeInteger(fixturePid) && fixturePid > 1);
            await Promise.race([
                once(worker, "exit"),
                new Promise((_resolve, reject) => {
                    timer = setTimeout(() => reject(new Error(
                        "watchdog kept its exited worker alive")), 1_000);
                }),
            ]);
        } finally {
            clearTimeout(timer);
            try { worker.kill("SIGKILL"); } catch {}
            if (Number.isSafeInteger(fixturePid) && fixturePid > 1) {
                try { process.kill(-fixturePid, "SIGKILL"); } catch {}
            }
        }
    },
);

// Linux Firefox runs in a dedicated process group. Its helpers can survive
// the leader, including an abrupt browser crash; reap the owned group even if
// the leader already emitted exit. Never signal an ordinary shared group.
function signalOwnedFixtureGroup(child, signal) {
    if (process.platform !== "linux" || child.fixtureProcessGroup !== true
        || !Number.isSafeInteger(child.pid) || child.pid <= 1) {
        return false;
    }
    try {
        process.kill(-child.pid, signal);
        return true;
    } catch (error) {
        if (error?.code === "ESRCH") return false;
        // OS error messages may contain process-private details.
        throw new Error(`disposable child ${signal} signal failed`);
    }
}

// Cleanup remains bounded even if a child does not emit exit after SIGKILL.
// Every path removes the test-owned process observers and both deadlines.
function stopChild(child, gracefulMs = 2_000, forcedMs = 10_000) {
    if (child.exitCode !== null || child.signalCode !== null) {
        try {
            signalOwnedFixtureGroup(child, "SIGKILL");
            return Promise.resolve();
        } catch (error) {
            return Promise.reject(error);
        }
    }
    return new Promise((resolve, reject) => {
        let gracefulTimer;
        let forcedTimer;
        let settled = false;
        const finish = (error = null) => {
            if (settled) {
                return;
            }
            settled = true;
            clearTimeout(gracefulTimer);
            clearTimeout(forcedTimer);
            child.off("exit", onExit);
            child.off("error", onError);
            if (error === null) {
                resolve();
            } else {
                reject(error);
            }
        };
        const onExit = () => {
            try {
                // A leader exit is not proof that Firefox helpers exited.
                signalOwnedFixtureGroup(child, "SIGKILL");
                finish();
            } catch (error) {
                finish(error);
            }
        };
        // Child process errors may expose process arguments or private data.
        const onError = () => finish(new Error(
            "disposable child process failed",
        ));
        const signalChild = (signal) => {
            try {
                if (!signalOwnedFixtureGroup(child, signal)
                    && !child.kill(signal)) {
                    if (child.exitCode !== null || child.signalCode !== null) {
                        finish();
                    } else {
                        finish(new Error(
                            `disposable child ${signal} signal failed`,
                        ));
                    }
                }
            } catch {
                finish(new Error(`disposable child ${signal} signal failed`));
            }
        };
        child.once("exit", onExit);
        child.once("error", onError);
        gracefulTimer = setTimeout(() => {
            if (child.exitCode !== null || child.signalCode !== null) {
                // A missed exit event must still reap the owned group.
                onExit();
                return;
            }
            forcedTimer = setTimeout(() => finish(new Error(
                "disposable child did not exit after SIGKILL",
            )), forcedMs);
            signalChild("SIGKILL");
        }, gracefulMs);
        signalChild("SIGTERM");
    });
}

// A Firefox launcher is test-owned, but its browser helpers may outlive the
// leader. A successful leader exit must not silently leave those helpers live.
test("Linux tracked Firefox group reaps a surviving child on leader exit",
    { skip: process.platform !== "linux", timeout: 5_000 },
    async () => {
        const helperCode = "setInterval(() => {}, 1000)";
        const leaderCode = [
            "const { spawn } = require('node:child_process');",
            "const helper = spawn(process.execPath,",
            `    ['-e', ${JSON.stringify(helperCode)}], { stdio: 'ignore' });`,
            "process.stdout.write(String(helper.pid) + '\\n',",
            "    () => process.exit(0));",
        ].join("\n");
        const leader = spawn(process.execPath, ["-e", leaderCode], {
            detached: true,
            stdio: ["ignore", "pipe", "ignore"],
        });
        leader.fixtureProcessGroup = true;
        trackFirefoxFixtureGroup(leader);
        assert.equal(liveFirefoxFixtureGroups.has(leader.pid), true);
        const exited = once(leader, "exit");
        try {
            const helperPid = Number(await waitForLine(
                leader.stdout,
                (line) => /^\d+$/u.test(line) ? line : null,
                2_000,
                leader,
            ));
            assert.ok(Number.isSafeInteger(helperPid) && helperPid > 0);
            await exited;
            assert.equal(liveFirefoxFixtureGroups.has(leader.pid), false);
            // The exit observer must reap helpers even without stopChild.
            const stopped = async () => {
                const deadline = Date.now() + 1_000;
                do {
                    let running = false;
                    try {
                        const stat = fs.readFileSync(
                            `/proc/${helperPid}/stat`, "utf8",
                        );
                        const state = stat[stat.lastIndexOf(") ") + 2];
                        running = !["Z", "X", "x"].includes(state);
                    } catch (error) {
                        if (error.code !== "ENOENT") throw error;
                    }
                    if (!running) return true;
                    await new Promise((resolve) => setTimeout(resolve, 25));
                } while (Date.now() < deadline);
                return false;
            };
            assert.equal(await stopped(), true,
                "fixture cleanup must kill helpers after leader exit");
            await stopChild(leader, 30, 30);
        } finally {
            // The red-first failure must not leave its owned group running.
            try { process.kill(-leader.pid, "SIGKILL"); } catch {}
            try { leader.kill("SIGKILL"); } catch {}
        }
    },
);

test("disposable child shutdown clears listeners on graceful exit",
    async () => {
        const child = new EventEmitter();
        child.exitCode = null;
        child.signalCode = null;
        const signals = [];
        child.kill = (signal) => {
            signals.push(signal);
            queueMicrotask(() => {
                child.signalCode = signal;
                child.emit("exit", null, signal);
            });
            return true;
        };
        await stopChild(child, 10, 10);
        assert.deepEqual(signals, ["SIGTERM"]);
        assert.equal(child.listenerCount("exit"), 0);
        assert.equal(child.listenerCount("error"), 0);
    },
);

test("disposable child shutdown escalates and observes SIGKILL exit",
    async () => {
        const child = new EventEmitter();
        child.exitCode = null;
        child.signalCode = null;
        const signals = [];
        child.kill = (signal) => {
            signals.push(signal);
            if (signal === "SIGKILL") {
                child.signalCode = signal;
                child.emit("exit", null, signal);
            }
            return true;
        };
        await stopChild(child, 10, 10);
        assert.deepEqual(signals, ["SIGTERM", "SIGKILL"]);
        assert.equal(child.listenerCount("exit"), 0);
        assert.equal(child.listenerCount("error"), 0);
    },
);

test("missed child-exit notification still reaps its disposable group",
    { skip: process.platform !== "linux", timeout: 5_000 },
    async () => {
        const sleeper = spawn(process.execPath, ["-e", [
            "process.on('SIGTERM', () => {});",
            "process.stdout.write('ready\\n');",
            "setInterval(() => {}, 1000);",
        ].join("\n")], {
            detached: true,
            stdio: ["ignore", "pipe", "ignore"],
        });
        const sleeperExited = once(sleeper, "exit");
        const reported = new EventEmitter();
        reported.pid = sleeper.pid;
        reported.fixtureProcessGroup = true;
        reported.exitCode = null;
        reported.signalCode = null;
        reported.kill = () => true;
        try {
            // A readiness handshake proves SIGTERM cannot end this sleeper.
            assert.equal(await waitForLine(sleeper.stdout,
                (line) => line === "ready" ? line : null, 2_000,
                sleeper), "ready");
            // Observe the real exit before stopChild can send SIGKILL.
            const completion = stopChild(reported, 50, 100);
            setTimeout(() => { reported.exitCode = 0; }, 10);
            await completion;
            let exitDeadline;
            try {
                await Promise.race([
                    sleeperExited,
                    new Promise((_resolve, reject) => {
                        exitDeadline = setTimeout(() => reject(new Error(
                            "disposable process group survived missed exit",
                        )), 1_000);
                    }),
                ]);
            } finally {
                clearTimeout(exitDeadline);
            }
        } finally {
            try { process.kill(-sleeper.pid, "SIGKILL"); } catch {}
        }
    },
);

test("delayed forced child exit remains within its bounded reap window",
    async () => {
        const child = new EventEmitter();
        child.exitCode = null;
        child.signalCode = null;
        const signals = [];
        child.kill = (signal) => {
            signals.push(signal);
            if (signal === "SIGKILL") {
                setTimeout(() => {
                    child.signalCode = signal;
                    child.emit("exit", null, signal);
                }, 45);
            }
            return true;
        };
        await stopChild(child, 10, 100);
        assert.deepEqual(signals, ["SIGTERM", "SIGKILL"]);
        assert.equal(child.listenerCount("exit"), 0);
        assert.equal(child.listenerCount("error"), 0);
    },
);

test("stubborn child aborts bounded shutdown without lingering observers",
    async () => {
        const child = new EventEmitter();
        child.exitCode = null;
        child.signalCode = null;
        const signals = [];
        child.kill = (signal) => {
            signals.push(signal);
            return true;
        };
        await assert.rejects(stopChild(child, 10, 10),
            /did not exit after SIGKILL/);
        assert.deepEqual(signals, ["SIGTERM", "SIGKILL"]);
        assert.equal(child.listenerCount("exit"), 0);
        assert.equal(child.listenerCount("error"), 0);
        await stopChild(Object.assign(new EventEmitter(), {
            exitCode: 0,
            signalCode: null,
        }), 10, 10);
    },
);

test("disposable child shutdown refuses unsuccessful signals", async () => {
    const failedTerm = new EventEmitter();
    failedTerm.exitCode = null;
    failedTerm.signalCode = null;
    const termSignals = [];
    failedTerm.kill = (signal) => {
        termSignals.push(signal);
        return false;
    };
    await assert.rejects(stopChild(failedTerm, 10, 20),
        /disposable child SIGTERM signal failed/);
    assert.deepEqual(termSignals, ["SIGTERM"]);
    assert.equal(failedTerm.listenerCount("exit"), 0);
    assert.equal(failedTerm.listenerCount("error"), 0);

    const failedKill = new EventEmitter();
    failedKill.exitCode = null;
    failedKill.signalCode = null;
    const killSignals = [];
    failedKill.kill = (signal) => {
        killSignals.push(signal);
        return signal === "SIGTERM";
    };
    await assert.rejects(stopChild(failedKill, 10, 20),
        /disposable child SIGKILL signal failed/);
    assert.deepEqual(killSignals, ["SIGTERM", "SIGKILL"]);
    assert.equal(failedKill.listenerCount("exit"), 0);
    assert.equal(failedKill.listenerCount("error"), 0);
});

test("disposable child shutdown errors redact private process details",
    async () => {
        const marker = "private-child-process-marker-3881";
        for (const signalToFail of ["SIGTERM", "SIGKILL"]) {
            const child = new EventEmitter();
            child.exitCode = null;
            child.signalCode = null;
            const signals = [];
            child.kill = (signal) => {
                signals.push(signal);
                if (signal === signalToFail) {
                    throw new Error(`private process ${marker}`);
                }
                return true;
            };
            await assert.rejects(stopChild(child, 10, 10), (error) => {
                assert.equal(error.message.includes(marker), false);
                assert.equal("cause" in error, false);
                assert.equal(error.message,
                    `disposable child ${signalToFail} signal failed`);
                return true;
            });
            assert.deepEqual(signals, signalToFail === "SIGTERM"
                ? ["SIGTERM"] : ["SIGTERM", "SIGKILL"]);
            assert.equal(child.listenerCount("exit"), 0);
            assert.equal(child.listenerCount("error"), 0);
        }
        const child = new EventEmitter();
        child.exitCode = null;
        child.signalCode = null;
        child.kill = () => {
            queueMicrotask(() => child.emit("error",
                new Error(`private process ${marker}`)));
            return true;
        };
        await assert.rejects(stopChild(child, 100, 100), (error) => {
            assert.equal(error.message.includes(marker), false);
            assert.equal("cause" in error, false);
            assert.match(error.message, /disposable child process failed/);
            return true;
        });
        assert.equal(child.listenerCount("exit"), 0);
        assert.equal(child.listenerCount("error"), 0);
    },
);

// Fixture shutdown must not wait for a browser that still owns an HTTP
// request. Close the listening socket first, then active HTTP connections.
async function closeFixtureServer(server, timeoutMs = 5_000) {
    if (server === undefined || !server.listening) {
        return;
    }
    // Node's closeAllConnections does not terminate upgraded WebSockets.
    // Keep teardown bounded and report ownership leaks to the test runner.
    await new Promise((resolve, reject) => {
        let settled = false;
        let closeReported = false;
        let closeFailed = false;
        let connectionsAborted = false;
        const finish = (error = null) => {
            if (settled) return;
            settled = true;
            clearTimeout(timer);
            if (error === null) resolve();
            else reject(error);
        };
        const fixedError = () => new Error(
            "disposable HTTP fixture close failed",
        );
        const timer = setTimeout(() => finish(new Error(
            "timed out closing disposable HTTP fixture server",
        )), timeoutMs);
        try {
            server.close((error) => {
                // Native errors and synchronous throws can contain private
                // socket metadata. Never publish them or their causes.
                closeReported = true;
                closeFailed = Boolean(error);
                if (connectionsAborted) {
                    finish(closeFailed ? fixedError() : null);
                }
            });
            server.closeAllConnections();
            connectionsAborted = true;
            if (closeReported) finish(closeFailed ? fixedError() : null);
        } catch {
            finish(fixedError());
        }
    });
}

// Unlike HTTP fixtures, raw TCP servers have no closeAllConnections API.
// Callers must destroy owned peer sockets first, then close within a deadline
// without printing native errors that may contain private request metadata.
function closeTcpFixtureServer(server, timeoutMs = 5_000) {
    if (!server.listening) return Promise.resolve();
    return new Promise((resolve, reject) => {
        let settled = false;
        let closeReturned = false;
        let closeReported = false;
        let closeFailed = false;
        const fixedError = () => new Error(
            "disposable TCP fixture close failed",
        );
        const finish = (error = null) => {
            if (settled) return;
            settled = true;
            clearTimeout(timer);
            if (error === null) resolve();
            else reject(error);
        };
        const timer = setTimeout(() => finish(new Error(
            "timed out closing disposable TCP fixture server",
        )), timeoutMs);
        try {
            server.close((error) => {
                closeReported = true;
                closeFailed = Boolean(error);
                // A synchronous callback may be followed by a throw from
                // close(). Do not announce success until the call returns.
                if (closeReturned) {
                    finish(closeFailed ? fixedError() : null);
                }
            });
            closeReturned = true;
            if (closeReported) finish(closeFailed ? fixedError() : null);
        } catch {
            finish(fixedError());
        }
    });
}

test("TCP fixture close failure and timeout remain bounded and private",
    async () => {
        const marker = "private-TCP-fixture-close-marker-4196";
        for (const cause of ["callback", "throw"]) {
            const server = {
                listening: true,
                close: (done) => {
                    if (cause === "throw") {
                        throw new Error(`close threw ${marker}`);
                    }
                    done(new Error(`close reported ${marker}`));
                },
            };
            await assert.rejects(closeTcpFixtureServer(server, 50),
                (error) => {
                    assert.equal(error.message.includes(marker), false);
                    assert.equal("cause" in error, false);
                    assert.equal(error.message,
                        "disposable TCP fixture close failed");
                    return true;
                });
        }
        // Even a synchronous success callback must not override a later
        // synchronous close throw from the same untrusted fixture owner.
        const successThenThrow = {
            listening: true,
            close: (done) => {
                done(null);
                throw new Error(`post-callback close failed: ${marker}`);
            },
        };
        await assert.rejects(closeTcpFixtureServer(successThenThrow, 50),
            (error) => {
                assert.equal(error.message.includes(marker), false);
                assert.equal("cause" in error, false);
                assert.equal(error.message,
                    "disposable TCP fixture close failed");
                return true;
            });
        const success = {
            listening: true,
            close: (done) => done(null),
        };
        await closeTcpFixtureServer(success, 50);
        let onLateClose;
        const hanging = {
            listening: true,
            close: (done) => { onLateClose = done; },
        };
        await assert.rejects(closeTcpFixtureServer(hanging, 20),
            /timed out closing disposable TCP fixture server/);
        onLateClose(new Error(`late close reported ${marker}`));
    },
);

test("fixture close errors never expose private socket diagnostics",
    async () => {
        const marker = "private-fixture-close-marker-7353";
        let closeCalls = 0;
        let abortCalls = 0;
        const server = {
            listening: true,
            close: (done) => {
                closeCalls += 1;
                done(new Error(`fixture close failed: ${marker}`));
            },
            closeAllConnections: () => { abortCalls += 1; },
        };
        await assert.rejects(closeFixtureServer(server, 100), (error) => {
            assert.equal(error.message.includes(marker), false);
            assert.equal("cause" in error, false);
            assert.equal(error.message,
                "disposable HTTP fixture close failed");
            return true;
        });
        assert.equal(closeCalls, 1);
        assert.equal(abortCalls, 1);
    },
);

test("fixture close synchronous failures never expose private details",
    async () => {
        const marker = "private-sync-fixture-close-marker-8291";
        for (const failedOperation of ["close", "closeAllConnections"]) {
            const calls = [];
            const server = {
                listening: true,
                close: (done) => {
                    calls.push("close");
                    if (failedOperation === "close") {
                        throw new Error(`close failed: ${marker}`);
                    }
                    queueMicrotask(() => done());
                },
                closeAllConnections: () => {
                    calls.push("abort");
                    if (failedOperation === "closeAllConnections") {
                        throw new Error(`abort failed: ${marker}`);
                    }
                },
            };
            await assert.rejects(closeFixtureServer(server, 50),
                (error) => {
                    assert.equal(error.message.includes(marker), false);
                    assert.equal("cause" in error, false);
                    assert.equal(error.message,
                        "disposable HTTP fixture close failed");
                    return true;
                });
            assert.ok(calls.includes("close"));
        }
    },
);

test(
    "readiness output handles fragmented lines and removes observers",
    async () => {
        const stream = new PassThrough();
        const readiness = waitForLine(stream, (line) => (
            line.startsWith("ready: ") ? line.slice(7) : null
        ));
        stream.write("noise\nrea");
        stream.write("dy: ws://127.0.0.1:1234\n");
        assert.equal(await readiness, "ws://127.0.0.1:1234");
        assert.equal(stream.listenerCount("data"), 0);
        assert.equal(stream.listenerCount("error"), 0);
        assert.equal(stream.listenerCount("close"), 0);
        stream.end();
    },
);

test("readiness selector ignores undefined until a valid selected line",
    async () => {
        const stream = new PassThrough();
        const pending = waitForLine(stream, (line) => {
            if (line === "ready: valid") {
                return "valid";
            }
            // An incomplete selector may omit its explicit null fallback.
        });
        stream.write("unrelated child output\n");
        stream.write("ready: valid\n");
        assert.equal(await pending, "valid");
        assert.equal(stream.listenerCount("data"), 0);
        stream.destroy();
    });

test("readiness selector refuses invalid selected values", async () => {
    for (const value of [0, false, "", { origin: "private" }]) {
        const stream = new PassThrough();
        const pending = waitForLine(stream, () => value);
        stream.write("ready: synthetic\n");
        await assert.rejects(pending, (error) => {
            assert.equal(error.message, "invalid readiness selector result");
            assert.equal("cause" in error, false);
            return true;
        });
        assert.equal(stream.listenerCount("data"), 0);
        stream.destroy();
    }
});

test(
    "readiness output fails promptly on closed or silent streams",
    async () => {
        const closed = new PassThrough();
        const pendingClose = waitForLine(closed, () => null);
        closed.end("partial output");
        await assert.rejects(pendingClose, /ended before readiness/);
        assert.equal(closed.listenerCount("data"), 0);
        const silent = new PassThrough();
        await assert.rejects(
            waitForLine(silent, () => null, 20),
            /timed out waiting for process output/,
        );
        assert.equal(silent.listenerCount("data"), 0);
        silent.destroy();
    },
);

test("readiness selector exceptions redact private lines", async () => {
    const marker = "private-selector-line-marker";
    const stream = new PassThrough();
    const pending = waitForLine(stream, (line) => {
        throw new Error(`invalid fixture line: ${line}`);
    });
    stream.write(`${marker}\n`);
    await assert.rejects(pending, (error) => {
        assert.equal(error.message.includes(marker), false);
        assert.match(error.message, /invalid readiness record/);
        return true;
    });
    assert.equal(stream.listenerCount("data"), 0);
    assert.equal(stream.listenerCount("error"), 0);
    stream.destroy();
});

test("readiness stream errors never disclose private error text", async () => {
    const marker = "private-stream-error-marker";
    const stream = new PassThrough();
    const pending = waitForLine(stream, () => null);
    stream.emit("error", new Error(`stream failed: ${marker}`));
    await assert.rejects(pending, (error) => {
        assert.equal(error.message.includes(marker), false);
        assert.match(error.message, /readiness stream failed/);
        return true;
    });
    assert.equal(stream.listenerCount("data"), 0);
    assert.equal(stream.listenerCount("error"), 0);
    stream.destroy();
});

test("readiness child errors cannot crash or expose test process text",
    async () => {
        const marker = "private-firefox-spawn-error-marker-3479";
        const child = new EventEmitter();
        const stream = new PassThrough();
        const pending = waitForLine(stream, () => null, 100, child);
        let escaped = false;
        try {
            child.emit("error", new Error(`spawn failed: ${marker}`));
        } catch {
            escaped = true;
            stream.end();
        }
        await assert.rejects(pending, (error) => {
            assert.equal(error.message.includes(marker), false);
            assert.equal("cause" in error, false);
            assert.equal(error.message, "readiness child process failed");
            return true;
        });
        assert.equal(escaped, false, "child failure needs an observer");
        assert.equal(child.listenerCount("error"), 0);
        assert.equal(stream.listenerCount("data"), 0);
        assert.equal(stream.listenerCount("error"), 0);
        stream.destroy();
    });

test("successful readiness releases the child process error observer",
    async () => {
        const child = new EventEmitter();
        const stream = new PassThrough();
        const pending = waitForLine(stream, (line) =>
            line === "ready: valid" ? "valid" : null, 100, child);
        stream.write("ready: valid\n");
        assert.equal(await pending, "valid");
        assert.equal(child.listenerCount("error"), 0);
        assert.equal(stream.listenerCount("data"), 0);
        stream.destroy();
    });

test("readiness failures never reflect untrusted process output", async () => {
    const marker = "private-process-output-marker";
    const closed = new PassThrough();
    const premature = waitForLine(closed, () => null);
    closed.end(marker);
    await assert.rejects(premature, (error) => {
        assert.match(error.message, /ended before readiness/);
        assert.equal(error.message.includes(marker), false);
        return true;
    });

    const silent = new PassThrough();
    const expired = waitForLine(silent, () => null, 20);
    silent.write(marker);
    await assert.rejects(expired, (error) => {
        assert.match(error.message, /timed out waiting for process output/);
        assert.equal(error.message.includes(marker), false);
        return true;
    });
    silent.destroy();
});

test("readiness output has a finite UTF-8 byte budget", async () => {
    const valid = new PassThrough();
    const accepted = waitForLine(valid, (line) =>
        line === "ready: ok" ? "ok" : null);
    valid.write("x".repeat(65_536 - "ready: ok\n".length - 1)
        + "\n");
    valid.write("ready: ok\n");
    assert.equal(await accepted, "ok");
    valid.destroy();

    const oversized = new PassThrough();
    const rejected = waitForLine(oversized, () => null, 50);
    // Counting UTF-16 characters instead of actual UTF-8 bytes would pass.
    oversized.write("á".repeat(32_769));
    await assert.rejects(rejected, /readiness output exceeds byte budget/);
    assert.equal(oversized.listenerCount("data"), 0);
    oversized.destroy();
});

test("readiness refuses malformed UTF-8 without replacing bytes", async () => {
    const invalid = new PassThrough();
    const rejected = waitForLine(invalid, (line) =>
        line.startsWith("ready: ") ? line.slice(7) : null);
    invalid.write(Buffer.concat([
        Buffer.from("ready: "),
        Buffer.from([0xff]),
        Buffer.from("\n"),
    ]));
    await assert.rejects(rejected, /invalid readiness output encoding/);
    assert.equal(invalid.listenerCount("data"), 0);
    assert.equal(invalid.listenerCount("error"), 0);
    invalid.destroy();

    // An upstream text decoder has already replaced invalid raw bytes.
    // Treat text-mode streams as untrusted rather than re-encoding them.
    const decoded = new PassThrough();
    decoded.setEncoding("utf8");
    const undecodable = waitForLine(decoded, (line) =>
        line.startsWith("ready: ") ? line.slice(7) : null);
    decoded.write(Buffer.concat([
        Buffer.from("ready: "), Buffer.from([0xff]),
        Buffer.from("\n"),
    ]));
    await assert.rejects(undecodable,
        /invalid readiness output encoding/);
    decoded.destroy();

    const valid = new PassThrough();
    const accepted = waitForLine(valid, (line) =>
        line.startsWith("ready: ") ? line.slice(7) : null);
    const bytes = Buffer.from("ready: á👩‍🔬\n");
    for (const byte of bytes) {
        valid.write(Buffer.from([byte]));
    }
    assert.equal(await accepted, "á👩‍🔬");
    valid.destroy();
});

test(
    "fixture shutdown aborts a pending HTTP request without browser exit",
    { timeout: 5_000 },
    async () => {
        let signalRequest;
        const observed = new Promise((resolve) => {
            signalRequest = resolve;
        });
        const server = http.createServer((_request, _response) => {
            signalRequest();
            // Deliberately leave this fixture request unfinished.
        });
        server.listen(0, "127.0.0.1");
        await once(server, "listening");
        const address = server.address();
        assert.notEqual(address, null);
        assert.notEqual(typeof address, "string");
        const request = http.get(`http://127.0.0.1:${address.port}/`);
        request.on("error", () => {});
        try {
            await observed;
            await closeFixtureServer(server);
            assert.equal(server.listening, false);
        } finally {
            request.destroy();
            await closeFixtureServer(server);
        }
    },
);

test("fixture server reports an upgraded connection that blocks shutdown",
    { timeout: 5_000 },
    async () => {
        const server = http.createServer((_request, response) => {
            response.end("ordinary response");
        });
        server.on("upgrade", (_request, peer) => {
            peer.write([
                "HTTP/1.1 101 Switching Protocols",
                "Connection: Upgrade",
                "Upgrade: websocket",
                "", "",
            ].join("\r\n"));
        });
        server.listen(0, "127.0.0.1");
        await once(server, "listening");
        const address = server.address();
        assert.notEqual(address, null);
        assert.notEqual(typeof address, "string");
        const socket = net.createConnection({
            host: "127.0.0.1",
            port: address.port,
        });
        socket.on("error", () => {});
        try {
            await waitForSocketConnection(socket, 500);
            const upgrade = once(socket, "data");
            socket.write([
                "GET /upgrade HTTP/1.1",
                `Host: 127.0.0.1:${address.port}`,
                "Connection: Upgrade",
                "Upgrade: websocket",
                "", "",
            ].join("\r\n"));
            await upgrade;
            await assert.rejects(closeFixtureServer(server, 25),
                /timed out closing disposable HTTP fixture/);
        } finally {
            socket.destroy();
            if (server.listening) {
                await closeFixtureServer(server);
            }
        }
    },
);

// A dead local Firefox endpoint must never leave a pending TCP connect with
// no deadline. The socket owner destroys the failed connection before exit.
function waitForSocketConnection(socket, timeoutMs = 10_000) {
    return new Promise((resolve, reject) => {
        const finish = (error) => {
            clearTimeout(timer);
            socket.off("connect", onConnect);
            socket.off("close", onClose);
            socket.off("error", onError);
            if (error === null) {
                resolve();
            } else {
                reject(error);
            }
        };
        const onConnect = () => finish(null);
        const onClose = () => finish(new Error(
            "remote debugging socket closed before TCP connect",
        ));
        const onError = () => finish(new Error(
            "remote debugging TCP connect failed",
        ));
        const timer = setTimeout(() => {
            socket.destroy();
            finish(new Error("timed out connecting to WebDriver socket"));
        }, timeoutMs);
        socket.once("connect", onConnect);
        socket.once("close", onClose);
        socket.once("error", onError);
    });
}

// Read one complete WebSocket upgrade header without dropping split TCP chunks
// or waiting indefinitely when a remote debugging endpoint stops responding.
function readWebSocketUpgrade(socket, timeoutMs = 10_000, signal = null) {
    return new Promise((resolve, reject) => {
        let received = Buffer.alloc(0);
        const finish = (error, result) => {
            clearTimeout(timer);
            socket.off("data", onData);
            socket.off("error", onError);
            socket.off("end", onEnd);
            socket.off("close", onEnd);
            signal?.removeEventListener("abort", onAbort);
            if (error === null) {
                resolve(result);
            } else {
                reject(error);
            }
        };
        const onAbort = () => finish(new Error(
            "WebSocket upgrade request cancelled",
        ));
        const onError = () => finish(new Error(
            "WebSocket upgrade socket failed",
        ));
        const onEnd = () => finish(new Error(
            "remote WebSocket endpoint closed before its upgrade response",
        ));
        const onData = (chunk) => {
            const maximumHeader = 16_384;
            // Inspect at most the header ceiling plus the four-byte marker.
            // A valid response may coalesce a large WebSocket data frame into
            // the same TCP chunk; that trailing data is not header content.
            const inspected = Math.min(chunk.length,
                Math.max(0, maximumHeader + 4 - received.length));
            received = Buffer.concat([received, chunk.subarray(0, inspected)]);
            const boundary = received.indexOf("\r\n\r\n");
            if (boundary > maximumHeader ||
                (boundary < 0 && received.length > maximumHeader)) {
                finish(new Error("WebSocket upgrade headers exceed limit"));
                return;
            }
            if (boundary < 0) {
                return;
            }
            // The upgrade header may share one TCP read with an attacker-sized
            // data frame. Refuse before copying the remainder into a second
            // buffer, even though header scanning itself is already bounded.
            const remainderBytes = received.length - boundary - 4
                + chunk.length - inspected;
            if (remainderBytes > MAX_BIDI_FRAME_BYTES + 10) {
                finish(new Error(
                    "WebSocket upgrade frame exceeds byte budget",
                ));
                return;
            }
            socket.pause();
            finish(null, {
                head: received.subarray(0, boundary).toString("latin1"),
                remainder: Buffer.concat([
                    received.subarray(boundary + 4),
                    chunk.subarray(inspected),
                ]),
            });
        };
        const timer = setTimeout(() => finish(new Error(
            "timed out waiting for WebSocket upgrade headers",
        )), timeoutMs);
        socket.on("data", onData);
        socket.once("error", onError);
        socket.once("end", onEnd);
        socket.once("close", onEnd);
        signal?.addEventListener("abort", onAbort, { once: true });
        if (signal?.aborted) {
            onAbort();
        }
    });
}

test("TCP debugging connection fails closed on early close and timeout",
    async () => {
        const connected = new PassThrough();
        const accepted = waitForSocketConnection(connected, 500);
        connected.emit("connect");
        await accepted;
        assert.equal(connected.listenerCount("connect"), 0);
        connected.destroy();
        const closed = new PassThrough();
        const premature = waitForSocketConnection(closed, 500);
        closed.destroy();
        await assert.rejects(premature, /closed before TCP connect/);
        assert.equal(closed.listenerCount("close"), 0);
        const silent = new PassThrough();
        await assert.rejects(waitForSocketConnection(silent, 20),
            /timed out connecting to WebDriver socket/);
        assert.equal(silent.destroyed, true);
        assert.equal(silent.listenerCount("connect"), 0);
    },
);

test("TCP connect socket errors redact private exception details",
    async () => {
        const marker = "private-bidi-connect-marker-1945";
        const stream = new PassThrough();
        const pending = waitForSocketConnection(stream, 500);
        stream.emit("error", new Error(`TCP connect error: ${marker}`));
        await assert.rejects(pending, (error) => {
            assert.equal(error.message.includes(marker), false);
            assert.equal("cause" in error, false);
            assert.match(error.message, /debugging TCP connect failed/);
            return true;
        });
        assert.equal(stream.listenerCount("connect"), 0);
        assert.equal(stream.listenerCount("error"), 0);
        assert.equal(stream.listenerCount("close"), 0);
        stream.destroy();
    },
);

test("WebSocket upgrade socket errors redact private exception details",
    async () => {
        const marker = "private-bidi-upgrade-marker-2145";
        const stream = new PassThrough();
        const pending = readWebSocketUpgrade(stream, 500);
        stream.emit("error", new Error(`upgrade error: ${marker}`));
        await assert.rejects(pending, (error) => {
            assert.equal(error.message.includes(marker), false);
            assert.equal("cause" in error, false);
            assert.match(error.message, /WebSocket upgrade socket failed/);
            return true;
        });
        assert.equal(stream.listenerCount("data"), 0);
        assert.equal(stream.listenerCount("error"), 0);
        assert.equal(stream.listenerCount("close"), 0);
        stream.destroy();
    },
);

test(
    "WebSocket upgrade collects fragmented headers and retains frame bytes",
    async () => {
        const stream = new PassThrough();
        const response = readWebSocketUpgrade(stream, 500);
        stream.write("HTTP/1.1 101 Switching Protocols\r\nUp");
        stream.write("grade: websocket\r\nConnection: Upgrade\r\n");
        stream.write(Buffer.concat([
            Buffer.from("\r\n"),
            Buffer.from([0x81, 0x02, 0x6f, 0x6b]),
        ]));
        const result = await response;
        assert.ok(result.head.startsWith("HTTP/1.1 101"));
        assert.deepEqual(result.remainder,
            Buffer.from([0x81, 0x02, 0x6f, 0x6b]));
        assert.equal(stream.listenerCount("data"), 0);
        assert.equal(stream.listenerCount("error"), 0);
        assert.equal(stream.listenerCount("close"), 0);
        stream.destroy();
    },
);

test("upgrade bounds header scanning without dropping coalesced frames",
    async () => {
        const stream = new PassThrough();
        const header = Buffer.from([
            "HTTP/1.1 101 Switching Protocols",
            "Upgrade: websocket",
            "Connection: Upgrade",
            "", "",
        ].join("\r\n"));
        const largeFrame = Buffer.alloc(256_000, 0xc5);
        const response = readWebSocketUpgrade(stream, 500);
        stream.write(Buffer.concat([header, largeFrame]));
        const accepted = await response;
        assert.equal(accepted.head,
            header.subarray(0, header.length - 4).toString("latin1"));
        assert.deepEqual(accepted.remainder, largeFrame);
        assert.equal(stream.listenerCount("data"), 0);
        stream.destroy();
        const invalid = new PassThrough();
        const rejected = readWebSocketUpgrade(invalid, 500);
        invalid.write(Buffer.alloc(256_000, 0x41));
        await assert.rejects(rejected, /headers exceed limit/);
        assert.equal(invalid.listenerCount("data"), 0);
        invalid.destroy();
    },
);

test("upgrade refuses an over-budget coalesced frame before copying",
    async () => {
        const header = Buffer.from([
            "HTTP/1.1 101 Switching Protocols",
            "Upgrade: websocket",
            "Connection: Upgrade",
            "", "",
        ].join("\r\n"));
        for (const [remainderSize, expectedAcceptance] of [
            [MAX_BIDI_FRAME_BYTES + 10, true],
            [MAX_BIDI_FRAME_BYTES + 11, false],
        ]) {
            const stream = new PassThrough();
            const pending = readWebSocketUpgrade(stream, 500);
            const bytes = Buffer.alloc(remainderSize, 0x6f);
            stream.write(Buffer.concat([header, bytes]));
            if (expectedAcceptance) {
                const accepted = await pending;
                assert.equal(accepted.remainder.length, remainderSize);
            } else {
                await assert.rejects(pending,
                    /WebSocket upgrade frame exceeds byte budget/);
            }
            assert.equal(stream.listenerCount("data"), 0);
            stream.destroy();
        }
    },
);

test("upgrade accepts the exact header-byte boundary across chunks",
    async () => {
        for (const [length, accepted] of [
            [16_384, true],
            [16_385, false],
        ]) {
            const stream = new PassThrough();
            const upgrade = readWebSocketUpgrade(stream, 500);
            stream.write("X".repeat(length));
            stream.write(Buffer.concat([
                Buffer.from("\r\n\r\n"),
                Buffer.from([0x81, 0x02, 0x6f, 0x6b]),
            ]));
            if (accepted) {
                const result = await upgrade;
                assert.equal(result.head.length, length);
                assert.deepEqual(result.remainder,
                    Buffer.from([0x81, 0x02, 0x6f, 0x6b]));
            } else {
                await assert.rejects(upgrade, /headers exceed limit/);
            }
            assert.equal(stream.listenerCount("data"), 0);
            stream.destroy();
        }
    },
);

test("WebSocket upgrade rejects missing or oversized response headers",
    async () => {
        const closed = new PassThrough();
        const closedResponse = readWebSocketUpgrade(closed, 500);
        closed.end("HTTP/1.1 101\r\n");
        await assert.rejects(closedResponse, /closed before its upgrade/);
        const oversized = new PassThrough();
        const oversizedResponse = readWebSocketUpgrade(oversized, 500);
        oversized.write("X".repeat(16_385));
        await assert.rejects(oversizedResponse, /exceed limit/);
        assert.equal(oversized.listenerCount("data"), 0);
        oversized.destroy();
        const silent = new PassThrough();
        await assert.rejects(readWebSocketUpgrade(silent, 20),
            /timed out waiting for WebSocket upgrade/);
        assert.equal(silent.listenerCount("data"), 0);
        silent.destroy();
    },
);

// A successful HTTP status alone does not establish a WebSocket peer.
function validateWebSocketUpgrade(head, key) {
    const lines = head.split("\r\n");
    // Never attach untrusted status text or headers to an AssertionError:
    // a Firefox peer can include private workspace content in either.
    if (!/^HTTP\/1\.1 101 [\t\x20-\x7e\x80-\xff]*$/.test(lines[0] ?? "")) {
        throw new Error("invalid WebSocket upgrade status");
    }
    const headers = new Map();
    for (const line of lines.slice(1)) {
        const colon = line.indexOf(":");
        if (colon <= 0) {
            throw new Error("invalid WebSocket upgrade header syntax");
        }
        const rawName = line.slice(0, colon);
        if (!/^[!#$%&'*+.^_`|~0-9A-Za-z-]+$/.test(rawName)) {
            throw new Error("invalid WebSocket upgrade header name");
        }
        const rawValue = line.slice(colon + 1);
        if (/[\x00-\x08\x0a-\x1f\x7f]/.test(rawValue)) {
            throw new Error("invalid WebSocket upgrade header value");
        }
        const name = rawName.toLowerCase();
        if (headers.has(name)) {
            throw new Error("duplicate WebSocket upgrade header");
        }
        headers.set(name, rawValue.replace(/^[ \t]+|[ \t]+$/g, ""));
    }
    if (headers.get("upgrade")?.toLowerCase() !== "websocket") {
        throw new Error("invalid WebSocket upgrade token");
    }
    const connection = headers.get("connection")?.toLowerCase();
    if (connection === undefined || !connection.split(",").some((token) =>
        token.replace(/^[ \t]+|[ \t]+$/g, "") === "upgrade")) {
        throw new Error("invalid WebSocket connection token");
    }
    const expected = crypto.createHash("sha1")
        .update(`${key}258EAFA5-E914-47DA-95CA-C5AB0DC85B11`)
        .digest("base64");
    if (headers.get("sec-websocket-accept") !== expected) {
        throw new Error("invalid WebSocket accept challenge");
    }
    // No extension or subprotocol was offered in the client request.
    if (headers.has("sec-websocket-extensions")) {
        throw new Error("unrequested WebSocket extension");
    }
    if (headers.has("sec-websocket-protocol")) {
        throw new Error("unrequested WebSocket subprotocol");
    }
}

// Firefox may announce BiDi readiness and create a session before the first
// tab can complete navigation on a highly contended workstation. Keep the
// ordinary command deadline unchanged, but permit the same bounded startup
// budget for tab creation and navigation as for session.new.
function defaultBidiCommandTimeout(method) {
    return method === "browsingContext.create"
        || method === "browsingContext.navigate"
        ? FIREFOX_BIDI_READY_TIMEOUT_MS
        : 10_000;
}

class BidiClient {
    constructor(url) {
        this.buffer = Buffer.alloc(0);
        this.closed = false;
        this.events = [];
        this.fragments = null;
        this.fragmentedBytes = 0;
        this.nextId = 1;
        this.pending = new Map();
        this.socket = null;
        // The endpoint comes from Firefox stderr, not a trusted local
        // configuration. Never open a debugger socket on another host (or
        // accept URL credentials or routing metadata) from that output.
        let endpoint;
        try {
            endpoint = new URL(url);
        } catch {
            // Node's URL error can expose the untrusted input as metadata.
            throw new Error("invalid Firefox BiDi loopback endpoint");
        }
        if (url !== endpoint.href
            || endpoint.protocol !== "ws:"
            || endpoint.hostname !== "127.0.0.1"
            || endpoint.port === ""
            || Number(endpoint.port) === 0
            || endpoint.pathname !== "/session"
            || endpoint.username !== ""
            || endpoint.password !== ""
            || endpoint.search !== ""
            || endpoint.hash !== "") {
            throw new Error("invalid Firefox BiDi loopback endpoint");
        }
        this.url = endpoint;
    }

    async connect() {
        this.socket = net.createConnection({
            host: this.url.hostname,
            port: Number(this.url.port),
        });
        try {
            await waitForSocketConnection(this.socket);
        } catch (error) {
            this.socket.destroy();
            throw error;
        }
        const key = crypto.randomBytes(16).toString("base64");
        const upgradeAbort = new AbortController();
        const upgrade = readWebSocketUpgrade(
            this.socket,
            FIREFOX_BIDI_READY_TIMEOUT_MS,
            upgradeAbort.signal,
        );
        try {
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
        } catch {
            // The socket can throw untrusted process/request text before the
            // upgrade promise is awaited. Cancel it to avoid an unhandled
            // rejection, release observers, and emit only a fixed error.
            upgradeAbort.abort();
            await upgrade.catch(() => {});
            this.socket.destroy();
            throw new Error("WebDriver BiDi upgrade request write failed");
        }
        let response;
        try {
            response = await upgrade;
            validateWebSocketUpgrade(response.head, key);
        } catch (error) {
            this.socket.destroy();
            throw error;
        }
        this.buffer = response.remainder;
        this.socket.on("data", (chunk) => {
            if (this.closed) {
                return;
            }
            if (chunk.length > MAX_BIDI_FRAME_BYTES + 10
                || this.buffer.length > MAX_BIDI_FRAME_BYTES + 10
                    - chunk.length) {
                this.close(new Error(
                    "WebDriver BiDi input exceeds byte limit",
                ));
                return;
            }
            this.buffer = Buffer.concat([this.buffer, chunk]);
            this.drain();
        });
        // A dead peer must settle pending commands immediately instead of
        // leaving every caller to wait for its separate command timeout.
        this.socket.on("error", () => this.close(new Error(
            "WebDriver BiDi socket failed",
        )));
        this.socket.on("close", () => this.close(new Error(
            "WebDriver BiDi socket closed before command completion",
        )));
        this.drain();
        this.socket.resume();
    }

    close(error = new Error("WebDriver BiDi client closed")) {
        if (this.closed) {
            return;
        }
        this.closed = true;
        for (const pending of this.pending.values()) {
            pending.reject(error);
        }
        this.pending.clear();
        this.socket?.destroy?.();
        this.buffer = Buffer.alloc(0);
        this.fragments = null;
        this.fragmentedBytes = 0;
    }

    command(method, params = {},
        timeoutMs = defaultBidiCommandTimeout(method)) {
        if (this.closed) {
            return Promise.reject(new Error("WebDriver BiDi client closed"));
        }
        const id = this.nextId;
        this.nextId += 1;
        return new Promise((resolve, reject) => {
            const timer = setTimeout(() => {
                if (this.pending.delete(id)) {
                    reject(new Error(`${method} timed out`));
                }
            }, timeoutMs);
            this.pending.set(id, {
                reject: (error) => {
                    clearTimeout(timer);
                    reject(error);
                },
                resolve: (result) => {
                    clearTimeout(timer);
                    resolve(result);
                },
            });
            try {
                this.socket.write(this.textFrame(JSON.stringify({
                    id,
                    method,
                    params,
                })));
            } catch {
                // A failed write invalidates every outstanding command.
                // Never reflect socket exceptions, which can echo the request.
                this.close(new Error("WebDriver BiDi socket write failed"));
            }
        });
    }

    drain() {
        while (this.buffer.length >= 2) {
            const opcode = this.buffer[0] & 0x0f;
            const final = (this.buffer[0] & 0x80) !== 0;
            // No WebSocket extensions were requested in the client upgrade.
            // An RSV bit cannot be interpreted as an ordinary text frame.
            if ((this.buffer[0] & 0x70) !== 0) {
                this.close(new Error("reserved WebDriver BiDi frame bits set"));
                return;
            }
            if ((this.buffer[1] & 0x80) !== 0) {
                this.close(new Error("masked WebDriver server frame refused"));
                return;
            }
            let length = this.buffer[1] & 0x7f;
            let offset = 2;
            if (length === 126) {
                if (this.buffer.length < 4) {
                    return;
                }
                length = this.buffer.readUInt16BE(2);
                if (length < 126) {
                    this.close(new Error("nonminimal WebDriver BiDi length"));
                    return;
                }
                offset = 4;
            } else if (length === 127) {
                if (this.buffer.length < 10) {
                    return;
                }
                const declared = this.buffer.readBigUInt64BE(2);
                if (declared < 65_536n) {
                    this.close(new Error("nonminimal WebDriver BiDi length"));
                    return;
                }
                if (declared > BigInt(MAX_BIDI_FRAME_BYTES)) {
                    this.close(new Error("WebDriver BiDi frame too large"));
                    return;
                }
                length = Number(declared);
                offset = 10;
            }
            if (length > MAX_BIDI_FRAME_BYTES) {
                this.close(new Error("WebDriver BiDi frame too large"));
                return;
            }
            if ((opcode & 0x8) !== 0 && (!final || length > 125)) {
                this.close(new Error("invalid WebDriver BiDi control frame"));
                return;
            }
            if (this.buffer.length < offset + length) {
                return;
            }
            let payload = this.buffer.subarray(offset, offset + length);
            this.buffer = this.buffer.subarray(offset + length);
            if (opcode === 0x8) {
                this.close(new Error("WebDriver BiDi peer sent close frame"));
                return;
            }
            // Control frames may occur between fragments of one text result.
            if (opcode === 0x9) {
                try {
                    this.socket.write(this.textFrame(payload, 0xa));
                } catch {
                    // Socket exceptions can contain untrusted ping bytes.
                    this.close(new Error("WebDriver BiDi socket write failed"));
                    return;
                }
                continue;
            }
            if (opcode === 0xa) {
                continue;
            }
            if (opcode === 0x1) {
                if (this.fragments !== null) {
                    this.close(new Error(
                        "overlapping WebDriver BiDi text frames",
                    ));
                    return;
                }
                if (!final) {
                    this.fragments = [payload];
                    this.fragmentedBytes = payload.length;
                    continue;
                }
            } else if (opcode === 0x0) {
                if (this.fragments === null) {
                    this.close(new Error("unexpected WebDriver continuation"));
                    return;
                }
                const total = this.fragmentedBytes + payload.length;
                if (total > MAX_BIDI_FRAME_BYTES) {
                    this.close(new Error("WebDriver BiDi message too large"));
                    return;
                }
                if (this.fragments.length >= MAX_BIDI_FRAGMENTS) {
                    this.close(new Error("too many WebDriver BiDi fragments"));
                    return;
                }
                this.fragments.push(payload);
                this.fragmentedBytes = total;
                if (!final) {
                    continue;
                }
                payload = Buffer.concat(this.fragments, total);
                this.fragments = null;
                this.fragmentedBytes = 0;
            } else {
                this.close(new Error("unsupported WebDriver BiDi data frame"));
                return;
            }
            // WebSocket text frames require valid UTF-8. Buffer.toString()
            // silently replaces malformed bytes and can turn a broken peer
            // response into syntactically valid JSON with altered content.
            let text;
            try {
                text = new TextDecoder("utf-8", { fatal: true })
                    .decode(payload);
            } catch {
                this.close(new Error("invalid WebDriver BiDi UTF-8 frame"));
                return;
            }
            let message;
            try {
                message = JSON.parse(text);
            } catch {
                this.close(new Error("invalid WebDriver BiDi JSON frame"));
                return;
            }
            if (message === null || typeof message !== "object") {
                this.close(new Error("invalid WebDriver BiDi message value"));
                return;
            }
            if (message.type === "success") {
                // Every BiDi command success has an object result. A bare
                // success marker must not confirm test session readiness.
                if (message.result === null
                    || typeof message.result !== "object"
                    || Array.isArray(message.result)) {
                    this.close(new Error(
                        "invalid WebDriver BiDi success result",
                    ));
                    return;
                }
            }
            if (message.type === "success" || message.type === "error") {
                const pending = this.pending.get(message.id);
                if (pending === undefined) {
                    continue;
                }
                this.pending.delete(message.id);
                if (message.type === "success") {
                    pending.resolve(message.result);
                } else {
                    const category = SAFE_BIDI_ERROR_CODES.has(message.error)
                        ? message.error : "unrecognized remote error";
                    pending.reject(new Error(
                        `WebDriver BiDi command failed: ${category}`,
                    ));
                }
            } else {
                if (this.events.length >= MAX_BIDI_EVENTS) {
                    this.events.shift();
                }
                this.events.push(message);
            }
        }
    }

    textFrame(text, opcode = 0x1) {
        const payload = Buffer.from(text);
        const mask = crypto.randomBytes(4);
        let header;
        if (payload.length < 126) {
            header = Buffer.from([0x80 | opcode, 0x80 | payload.length]);
        } else {
            assert.ok(payload.length < 65_536);
            header = Buffer.from([
                0x80 | opcode,
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

test("Firefox BiDi client refuses off-loopback debugging endpoints", () => {
    const allowed = new BidiClient("ws://127.0.0.1:1234/session");
    assert.equal(allowed.url.hostname, "127.0.0.1");
    const marker = "malformed-debugger-marker-secret";
    assert.throws(() => new BidiClient(marker), (error) => {
        assert.match(error.message, /invalid Firefox BiDi loopback endpoint/);
        assert.equal(error.message.includes(marker), false);
        assert.equal("input" in error, false);
        return true;
    });
    for (const endpoint of [
        "ws://127.1:1234/session",
        "ws://0x7f000001:1234/session",
        "WS://127.0.0.1:1234/session",
        " ws://127.0.0.1:1234/session",
        "ws://example.com:1234/session",
        "ws://192.0.2.1:1234/session",
        "ws://localhost:1234/session",
        "ws://[::1]:1234/session",
        "wss://127.0.0.1:1234/session",
        "http://127.0.0.1:1234/session",
        "ws://user:password@127.0.0.1:1234/session",
        "ws://127.0.0.1:1234/session?access=marker",
        "ws://127.0.0.1:1234/session#fragment",
        "ws://127.0.0.1/session",
        "ws://127.0.0.1:0/session",
    ]) {
        assert.throws(() => new BidiClient(endpoint),
            /invalid Firefox BiDi loopback endpoint/,
            endpoint);
    }
});

test("WebDriver upgrade write failures redact private socket errors",
    async () => {
        const marker = "private-upgrade-write-marker-7331";
        const socket = new PassThrough();
        const originalCreateConnection = net.createConnection;
        socket.write = () => {
            throw new Error(`failed upgrade request: ${marker}`);
        };
        net.createConnection = () => {
            queueMicrotask(() => socket.emit("connect"));
            return socket;
        };
        try {
            const bidi = new BidiClient("ws://127.0.0.1:1234/session");
            await assert.rejects(bidi.connect(), (error) => {
                assert.equal(error.message.includes(marker), false);
                assert.equal("cause" in error, false);
                assert.match(error.message, /upgrade request write failed/);
                return true;
            });
            assert.equal(socket.destroyed, true);
            assert.equal(socket.listenerCount("data"), 0);
            assert.equal(socket.listenerCount("error"), 0);
            assert.equal(socket.listenerCount("close"), 0);
        } finally {
            net.createConnection = originalCreateConnection;
            socket.destroy();
        }
    },
);

test("WebDriver upgrade headers use finite Firefox startup budget",
    async () => {
    const server = net.createServer((peer) => {
        peer.once("data", () => peer.end(
            "HTTP/1.1 403 Forbidden\r\nConnection: close\r\n\r\n",
        ));
    });
    server.listen(0, "127.0.0.1");
    await once(server, "listening");
    const address = server.address();
    assert.notEqual(address, null);
    assert.notEqual(typeof address, "string");
    const bidi = new BidiClient(
        `ws://127.0.0.1:${address.port}/session`,
    );
    const originalSetTimeout = globalThis.setTimeout;
    const deadlines = [];
    try {
        globalThis.setTimeout = (callback, delay, ...args) => {
            deadlines.push(delay);
            return originalSetTimeout(callback, delay, ...args);
        };
        await assert.rejects(bidi.connect());
    } finally {
        globalThis.setTimeout = originalSetTimeout;
        bidi.close();
        await closeTcpFixtureServer(server);
    }
    assert.ok(deadlines.includes(FIREFOX_BIDI_READY_TIMEOUT_MS),
        "Firefox WebSocket upgrade must use a finite startup budget");
});

test(
    "WebDriver initial tab creation and navigation use readiness budget",
    async () => {
        const originalSetTimeout = globalThis.setTimeout;
        const observed = [];
        try {
            globalThis.setTimeout = (callback, delay) => {
                observed.push(delay);
                return originalSetTimeout(callback, delay);
            };
            for (const [method, timeout] of [
                ["browsingContext.create", undefined],
                ["browsingContext.navigate", undefined],
                ["script.evaluate", undefined],
                ["browsingContext.create", 75],
            ]) {
                const bidi = new BidiClient("ws://127.0.0.1:1234/session");
                bidi.socket = { write() {} };
                const pending = timeout === undefined
                    ? bidi.command(method)
                    : bidi.command(method, {}, timeout);
                bidi.close();
                await assert.rejects(pending, /WebDriver BiDi client closed/);
            }
        } finally {
            globalThis.setTimeout = originalSetTimeout;
        }
        assert.deepEqual(observed, [FIREFOX_BIDI_READY_TIMEOUT_MS,
            FIREFOX_BIDI_READY_TIMEOUT_MS, 10_000, 75]);
    },
);

test("WebSocket upgrade rejects false status and forged accept headers",
    () => {
        const key = "dGhlIHNhbXBsZSBub25jZQ==";
        const accepted = [
            "HTTP/1.1 101 Switching Protocols",
            "Upgrade: websocket",
            "Connection: keep-alive, Upgrade",
            "Sec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=",
        ].join("\r\n");
        assert.doesNotThrow(() => validateWebSocketUpgrade(accepted, key));
        assert.throws(() => validateWebSocketUpgrade(
            accepted.replace("101 Switching", "1010 Switching"), key,
        ));
        assert.throws(() => validateWebSocketUpgrade(
            accepted.replace("Connection: keep-alive, Upgrade",
                "Connection: keep-alive"), key,
        ));
        assert.throws(() => validateWebSocketUpgrade(
            accepted.replace("s3pPLMBiTxaQ9kYGzzhZRbK+xOo=", "invalid"), key,
        ));
        assert.throws(() => validateWebSocketUpgrade(
            `${accepted}\r\nUpgrade: websocket`, key,
        ));
    },
);

test("WebSocket upgrade rejection omits untrusted response text", () => {
    const secretMarker = "synthetic-private-websocket-reply-1967";
    const key = "dGhlIHNhbXBsZSBub25jZQ==";
    const accepted = [
        "HTTP/1.1 101 Switching Protocols",
        "Upgrade: websocket",
        "Connection: Upgrade",
        "Sec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=",
    ].join("\r\n");
    const forged = [
        accepted.replace("Switching Protocols", `\n${secretMarker}`),
        accepted.replace("Upgrade: websocket", `Upgrade: ${secretMarker}`),
        accepted.replace("Upgrade: websocket",
            `X ${secretMarker}: websocket`),
        accepted.replace("Connection: Upgrade",
            `Connection: ${secretMarker}`),
        accepted.replace("s3pPLMBiTxaQ9kYGzzhZRbK+xOo=", secretMarker),
    ];
    for (const head of forged) {
        assert.throws(() => validateWebSocketUpgrade(head, key), (error) => {
            assert.equal(error.message.includes(secretMarker), false);
            assert.equal(JSON.stringify(error).includes(secretMarker), false);
            assert.equal("actual" in error, false);
            assert.equal("expected" in error, false);
            return true;
        });
    }
});

test("WebSocket upgrade refuses malformed field names and values", () => {
    const key = "dGhlIHNhbXBsZSBub25jZQ==";
    const accepted = [
        "HTTP/1.1 101 Switching Protocols",
        "Upgrade: websocket",
        "Connection: Upgrade",
        "Sec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=",
    ].join("\r\n");
    for (const malformed of [
        accepted.replace("Upgrade:", "Upgrade :"),
        accepted.replace("Upgrade:", " Upgrade:"),
        accepted.replace("Connection: Upgrade",
            "Connection: Upgrade\u0000, Upgrade"),
        accepted.replace("Connection: Upgrade",
            "Connection: Upgrade\n, Upgrade"),
        accepted.replace("Upgrade: websocket",
            "Upgrade: \u00a0websocket\u00a0"),
        accepted.replace("Connection: Upgrade",
            "Connection: \u00a0Upgrade\u00a0"),
    ]) {
        assert.throws(() => validateWebSocketUpgrade(malformed, key));
    }
    // SP and HTAB are the only outer HTTP whitespace that may be trimmed.
    assert.doesNotThrow(() => validateWebSocketUpgrade(
        accepted.replace("Upgrade: websocket", "Upgrade:\twebsocket\t"),
        key,
    ));
});

test("WebSocket upgrade refuses malformed status-line controls", () => {
    const key = "dGhlIHNhbXBsZSBub25jZQ==";
    const accepted = [
        "HTTP/1.1 101 Switching Protocols",
        "Upgrade: websocket",
        "Connection: Upgrade",
        "Sec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=",
    ].join("\r\n");
    for (const replacement of [
        "101\nSwitching Protocols",
        "101\tSwitching Protocols",
        "101 Switching\u0000Protocols",
        "101",
    ]) {
        assert.throws(() => validateWebSocketUpgrade(
            accepted.replace("101 Switching Protocols", replacement), key,
        ));
    }
});

test("WebSocket upgrade refuses unsolicited extension and subprotocol", () => {
    const key = "dGhlIHNhbXBsZSBub25jZQ==";
    const accepted = [
        "HTTP/1.1 101 Switching Protocols",
        "Upgrade: websocket",
        "Connection: Upgrade",
        "Sec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=",
    ].join("\r\n");
    for (const header of [
        "Sec-WebSocket-Extensions: permessage-deflate",
        "Sec-WebSocket-Protocol: webdriver-bidi",
    ]) {
        assert.throws(() => validateWebSocketUpgrade(
            `${accepted}\r\n${header}`, key,
        ), /unrequested WebSocket/);
    }
});

test(
    "WebDriver client rejects invalid upgrade status and closes its socket",
    { timeout: 5_000 },
    async () => {
        const server = net.createServer((peer) => {
            peer.once("data", () => {
                peer.write([
                    "HTTP/1.1 403 Forbidden",
                    "Content-Length: 0",
                    "",
                    "",
                ].join("\r\n"));
            });
        });
        server.listen(0, "127.0.0.1");
        await once(server, "listening");
        const address = server.address();
        assert.notEqual(address, null);
        assert.notEqual(typeof address, "string");
        const client = new BidiClient(`ws://127.0.0.1:${address.port}/session`);
        try {
            await assert.rejects(client.connect(),
                /invalid WebSocket upgrade status/);
            assert.equal(client.socket.destroyed, true);
        } finally {
            client.socket?.destroy();
            await closeTcpFixtureServer(server);
        }
    },
);

test("WebDriver errors and timeouts release their pending command slots",
    async () => {
        const client = new BidiClient("ws://127.0.0.1:1234/session");
        const requests = [];
        client.socket = { write: (frame) => requests.push(frame) };
        const rejected = client.command("script.evaluate", {}, 100);
        assert.equal(client.pending.size, 1);
        const result = Buffer.from(JSON.stringify({
            id: 1,
            type: "error",
            error: "invalid argument",
        }));
        client.buffer = Buffer.concat([
            Buffer.from([0x81, result.length]), result,
        ]);
        client.drain();
        await assert.rejects(rejected, /invalid argument/);
        assert.equal(client.pending.size, 0);
        const accepted = client.command("session.status", {}, 100);
        const success = Buffer.from(JSON.stringify({
            id: 2,
            type: "success",
            result: { ready: true },
        }));
        client.buffer = Buffer.concat([
            Buffer.from([0x81, success.length]), success,
        ]);
        client.drain();
        assert.deepEqual(await accepted, { ready: true });
        assert.equal(client.pending.size, 0);
        assert.equal(requests.length, 2);
        await assert.rejects(client.command("never.responds", {}, 20),
            /never.responds timed out/);
        assert.equal(client.pending.size, 0);
    },
);

test("WebDriver BiDi command errors never reflect remote private fields",
    async () => {
        const privateText = "synthetic-private-bidi-error-marker-7529";
        for (const [errorCode, expectedCode] of [
            ["invalid argument", "invalid argument"],
            [privateText, "unrecognized remote error"],
        ]) {
            const client = new BidiClient("ws://127.0.0.1:1234/session");
            client.socket = { write: () => {}, destroy: () => {} };
            const pending = client.command("script.evaluate", {}, 100);
            const errorReply = {
                id: 1,
                type: "error",
                error: errorCode,
                message: privateText,
                stacktrace: privateText,
                data: { draft: privateText },
            };
            client.buffer = testServerFrame(JSON.stringify(errorReply));
            client.drain();
            await assert.rejects(pending, (error) => {
                assert.match(error.message, /WebDriver BiDi command failed/u);
                assert.ok(error.message.includes(expectedCode));
                assert.equal(error.message.includes(privateText), false);
                assert.deepEqual(Object.keys(error), []);
                return true;
            });
            assert.equal(client.pending.size, 0);
            client.close();
        }
    },
);

function testServerFrame(payload, opcode = 0x1, final = true) {
    const bytes = Buffer.from(payload);
    const firstByte = opcode | (final ? 0x80 : 0);
    if (bytes.length < 126) {
        return Buffer.concat([
            Buffer.from([firstByte, bytes.length]), bytes,
        ]);
    }
    if (bytes.length <= 65_535) {
        const prefix = Buffer.from([firstByte, 126, 0, 0]);
        prefix.writeUInt16BE(bytes.length, 2);
        return Buffer.concat([prefix, bytes]);
    }
    const prefix = Buffer.from([firstByte, 127, 0, 0, 0, 0, 0, 0, 0, 0]);
    prefix.writeBigUInt64BE(BigInt(bytes.length), 2);
    return Buffer.concat([prefix, bytes]);
}

test("WebDriver BiDi refuses unnegotiated reserved frame bits", async () => {
    const response = JSON.stringify({
        id: 1, type: "success", result: { accepted: true },
    });
    for (const bit of [0x40, 0x20, 0x10]) {
        const client = new BidiClient("ws://127.0.0.1:1234/session");
        let destroyed = 0;
        client.socket = {
            write: () => {}, destroy: () => { destroyed += 1; },
        };
        const pending = client.command("script.evaluate", {}, 30_000);
        const frame = testServerFrame(response);
        frame[0] |= bit;
        client.buffer = frame;
        client.drain();
        await assert.rejects(pending, /reserved WebDriver BiDi frame bits/);
        assert.equal(client.closed, true);
        assert.equal(client.pending.size, 0);
        assert.equal(destroyed, 1);
    }
});

test("WebDriver BiDi refuses nonminimal length encodings", async () => {
    const message = Buffer.from(JSON.stringify({
        id: 1, type: "success", result: { accepted: true },
    }));
    assert.ok(message.length < 126);
    for (const wide of [false, true]) {
        const length = wide ? 127 : 126;
        const prefix = Buffer.alloc(wide ? 10 : 4);
        prefix[0] = 0x81;
        prefix[1] = length;
        if (wide) {
            prefix.writeBigUInt64BE(BigInt(message.length), 2);
        } else {
            prefix.writeUInt16BE(message.length, 2);
        }
        const client = new BidiClient("ws://127.0.0.1:1234/session");
        let destroyed = 0;
        client.socket = {
            write: () => {}, destroy: () => { destroyed += 1; },
        };
        const pending = client.command("script.evaluate", {}, 30_000);
        client.buffer = Buffer.concat([prefix, message]);
        client.drain();
        await assert.rejects(pending, /nonminimal WebDriver BiDi length/);
        assert.equal(client.closed, true);
        assert.equal(client.pending.size, 0);
        assert.equal(destroyed, 1);
    }
});

test("WebDriver BiDi refuses gigantic declared and masked inbound frames",
    async () => {
        for (const input of [
            Buffer.from([0x81, 0x7f, 0, 0, 0, 0, 0, 0x80, 0, 1]),
            Buffer.from([0x81, 0xfe, 0, 1, 0, 0, 0, 0, 0]),
        ]) {
            const client = new BidiClient("ws://127.0.0.1:1234/session");
            let destroyed = 0;
            client.socket = {
                write: () => {}, destroy: () => { destroyed += 1; },
            };
            const pending = client.command("script.evaluate", {}, 30_000);
            client.buffer = input;
            client.drain();
            await assert.rejects(pending,
                /frame too large|masked WebDriver server frame/);
            assert.equal(client.pending.size, 0);
            assert.equal(client.buffer.length, 0);
            assert.equal(destroyed, 1);
        }
    },
);

test("WebDriver BiDi rejects malformed JSON, null data and close frames",
    async () => {
        for (const [frame, expected] of [
            [testServerFrame("{invalid"), /invalid WebDriver BiDi JSON/],
            [testServerFrame("null"), /invalid WebDriver BiDi message/],
            [testServerFrame(Buffer.alloc(0), 0x8), /peer sent close frame/],
        ]) {
            const client = new BidiClient("ws://127.0.0.1:1234/session");
            client.socket = {
                write: () => {}, destroy: () => {},
            };
            const pending = client.command("script.evaluate", {}, 30_000);
            client.buffer = frame;
            client.drain();
            await assert.rejects(pending, expected);
            assert.equal(client.pending.size, 0);
            assert.equal(client.closed, true);
        }
    },
);

test("WebDriver BiDi rejects malformed successful command results",
    async () => {
        const malformed = [
            { id: 1, type: "success" },
            ...[null, [], "accepted", 17, true].map((result) => ({
                id: 1, type: "success", result,
            })),
        ];
        for (const message of malformed) {
            const client = new BidiClient("ws://127.0.0.1:1234/session");
            let destroyed = 0;
            client.socket = {
                write: () => {}, destroy: () => { destroyed += 1; },
            };
            const pending = client.command("session.new", {}, 30_000);
            client.buffer = testServerFrame(JSON.stringify(message));
            client.drain();
            await assert.rejects(pending,
                /invalid WebDriver BiDi success result/);
            assert.equal(client.closed, true);
            assert.equal(client.pending.size, 0);
            assert.equal(destroyed, 1);
        }
    },
);

test("WebDriver BiDi refuses malformed UTF-8 in JSON messages", async () => {
    const prefix = Buffer.from(
        '{"id":1,"type":"success","result":{"value":"',
    );
    const suffix = Buffer.from('"}}');
    const malformed = Buffer.concat([
        prefix, Buffer.from([0xe2, 0x28]), suffix,
    ]);
    for (const frames of [
        [testServerFrame(malformed)],
        [
            testServerFrame(Buffer.concat([prefix, Buffer.from([0xe2])]),
                0x1, false),
            testServerFrame(Buffer.concat([Buffer.from([0x28]), suffix]),
                0x0, true),
        ],
    ]) {
        const client = new BidiClient("ws://127.0.0.1:1234/session");
        let destroyed = 0;
        client.socket = {
            write: () => {}, destroy: () => { destroyed += 1; },
        };
        const pending = client.command("script.evaluate", {}, 30_000);
        for (const frame of frames) {
            client.buffer = Buffer.concat([client.buffer, frame]);
            client.drain();
        }
        await assert.rejects(pending, /invalid WebDriver BiDi UTF-8/);
        assert.equal(client.closed, true);
        assert.equal(client.pending.size, 0);
        assert.equal(destroyed, 1);
    }
});

test("WebDriver BiDi parses bounded extended and split frames losslessly",
    async () => {
        for (const count of [1, 126, 512, 65_536]) {
            const client = new BidiClient("ws://127.0.0.1:1234/session");
            client.socket = {
                write: () => {}, destroy: () => {},
            };
            const pending = client.command("script.evaluate", {}, 30_000);
            const large = "a".repeat(count);
            const encoded = testServerFrame(JSON.stringify({
                id: 1, type: "success", result: { value: large },
            }));
            const midway = Math.floor(encoded.length / 2);
            client.buffer = encoded.subarray(0, midway);
            client.drain();
            assert.equal(client.pending.size, 1);
            client.buffer = Buffer.concat([
                client.buffer, encoded.subarray(midway),
            ]);
            client.drain();
            assert.deepEqual(await pending, { value: large });
            assert.equal(client.pending.size, 0);
            assert.equal(client.closed, false);
            client.close();
        }
    },
);

test("WebDriver BiDi unsolicited event retention is bounded",
    () => {
        const client = new BidiClient("ws://127.0.0.1:1234/session");
        client.socket = {
            write: () => {}, destroy: () => {},
        };
        for (let index = 0; index < 256; index += 1) {
            client.buffer = testServerFrame(JSON.stringify({
                type: "event", id: index,
            }));
            client.drain();
        }
        assert.equal(client.events.length, MAX_BIDI_EVENTS);
        assert.deepEqual(client.events.map((event) => event.id),
            Array.from({ length: MAX_BIDI_EVENTS },
                (_, offset) => 256 - MAX_BIDI_EVENTS + offset));
        client.close();
    },
);

test("WebDriver BiDi assembles fragmented JSON across continuation frames",
    async () => {
        const client = new BidiClient("ws://127.0.0.1:1234/session");
        client.socket = { write: () => {}, destroy: () => {} };
        const pending = client.command("script.evaluate", {}, 30_000);
        const complete = Buffer.from(JSON.stringify({
            id: 1, type: "success", result: { value: "á 👩‍🔬" },
        }));
        // Place continuation boundaries *inside* the four-byte emoji,
        // proving UTF-8 decoding waits for the complete assembled message.
        const scalarOffset = complete.indexOf(Buffer.from("👩"));
        assert.ok(scalarOffset > 0);
        const parts = [
            complete.subarray(0, scalarOffset + 1),
            complete.subarray(scalarOffset + 1, scalarOffset + 3),
            complete.subarray(scalarOffset + 3),
        ];
        const first = testServerFrame(parts[0], 0x1, false);
        const pong = testServerFrame("", 0xa);
        const middle = testServerFrame(parts[1], 0x0, false);
        const last = testServerFrame(parts[2], 0x0, true);
        client.buffer = Buffer.concat([first, pong, middle]);
        client.drain();
        assert.equal(client.pending.size, 1);
        assert.equal(client.fragmentedBytes, parts[0].length + parts[1].length);
        client.buffer = last.subarray(0, 2);
        client.drain();
        assert.equal(client.pending.size, 1);
        client.buffer = Buffer.concat([
            client.buffer, last.subarray(2),
        ]);
        client.drain();
        assert.deepEqual(await pending, { value: "á 👩‍🔬" });
        assert.equal(client.fragments, null);
        assert.equal(client.fragmentedBytes, 0);
        client.close();
    },
);

test("WebDriver BiDi rejects orphan, overlapping and binary data frames",
    async () => {
        for (const [frame, expected, openFragment] of [
            [testServerFrame("{}", 0x0),
                /unexpected WebDriver continuation/, false],
            [testServerFrame("{}", 0x1), /overlapping WebDriver BiDi/, true],
            [testServerFrame("{}", 0x2),
                /unsupported WebDriver BiDi data/, false],
            [testServerFrame("", 0x9, false),
                /invalid WebDriver BiDi control/, false],
        ]) {
            const client = new BidiClient("ws://127.0.0.1:1234/session");
            client.socket = { write: () => {}, destroy: () => {} };
            const pending = client.command("script.evaluate", {}, 30_000);
            if (openFragment) {
                client.fragments = [Buffer.from("{")];
                client.fragmentedBytes = 1;
            }
            client.buffer = frame;
            client.drain();
            await assert.rejects(pending, expected);
            assert.equal(client.pending.size, 0);
            assert.equal(client.fragments, null);
            assert.equal(client.fragmentedBytes, 0);
        }
    },
);

test("WebDriver BiDi reassembles JSON at every bytewise split point",
    async () => {
        const source = Buffer.from(JSON.stringify({
            id: 1,
            type: "success",
            result: { value: "NFC=á; NFD=a\u0301; ZWJ=👩‍🔬" },
        }));
        for (let split = 1; split < source.length; split += 1) {
            const client = new BidiClient("ws://127.0.0.1:1234/session");
            client.socket = { write: () => {}, destroy: () => {} };
            const pending = client.command("script.evaluate", {}, 30_000);
            const frames = Buffer.concat([
                testServerFrame(source.subarray(0, split), 0x1, false),
                testServerFrame(source.subarray(split), 0x0, true),
            ]);
            client.buffer = frames;
            client.drain();
            const observed = await pending;
            assert.equal(observed.value, "NFC=á; NFD=a\u0301; ZWJ=👩‍🔬");
            assert.equal(client.fragments, null);
            assert.equal(client.pending.size, 0);
            client.close();
        }
        assert.ok(source.length > 50);
    },
);

test("WebDriver BiDi answers server ping with an RFC 6455 masked pong",
    () => {
        const client = new BidiClient("ws://127.0.0.1:1234/session");
        const writes = [];
        client.socket = {
            write: (bytes) => { writes.push(bytes); },
            destroy: () => {},
        };
        client.buffer = testServerFrame("live", 0x9);
        client.drain();
        assert.equal(writes.length, 1);
        const pong = writes[0];
        assert.equal(pong[0], 0x8a);
        assert.equal(pong[1] & 0x80, 0x80);
        assert.equal(pong[1] & 0x7f, 4);
        const mask = pong.subarray(2, 6);
        const decoded = pong.subarray(6).map((value, index) =>
            value ^ mask[index % 4]);
        assert.deepEqual(decoded, Buffer.from("live"));
        assert.equal(client.closed, false);
        client.close();
    },
);

test("WebDriver BiDi refuses unlimited empty continuation fragments",
    async () => {
        const client = new BidiClient("ws://127.0.0.1:1234/session");
        let destroyed = 0;
        client.socket = {
            write: () => {},
            destroy: () => { destroyed += 1; },
        };
        const pending = client.command("script.evaluate", {}, 30_000);
        client.buffer = testServerFrame("{", 0x1, false);
        client.drain();
        for (let index = 1; index < MAX_BIDI_FRAGMENTS; index += 1) {
            client.buffer = testServerFrame("", 0x0, false);
            client.drain();
            assert.equal(client.closed, false);
        }
        assert.equal(client.fragments.length, MAX_BIDI_FRAGMENTS);
        assert.equal(client.fragmentedBytes, 1);
        client.buffer = testServerFrame("", 0x0, true);
        client.drain();
        await assert.rejects(pending, /too many WebDriver BiDi fragments/);
        assert.equal(client.fragments, null);
        assert.equal(client.fragmentedBytes, 0);
        assert.equal(client.pending.size, 0);
        assert.equal(destroyed, 1);
    },
);

test("WebDriver BiDi bounds aggregate continuation bytes before joining",
    async () => {
        const client = new BidiClient("ws://127.0.0.1:1234/session");
        client.socket = { write: () => {}, destroy: () => {} };
        const pending = client.command("script.evaluate", {}, 30_000);
        // Synthetic state avoids allocating the multi-megabyte adversarial
        // prefix while testing the exact arithmetic refusal boundary.
        client.fragments = [Buffer.from("x")];
        client.fragmentedBytes = MAX_BIDI_FRAME_BYTES - 1;
        client.buffer = testServerFrame("12", 0x0, true);
        client.drain();
        await assert.rejects(pending, /WebDriver BiDi message too large/);
        assert.equal(client.fragments, null);
        assert.equal(client.pending.size, 0);
        assert.equal(client.closed, true);
    },
);

test("WebDriver client close rejects commands and destroys its socket",
    async () => {
        const client = new BidiClient("ws://127.0.0.1:1234/session");
        const sent = [];
        let destroyed = 0;
        client.socket = {
            write: (frame) => sent.push(frame),
            destroy: () => { destroyed += 1; },
        };
        const pending = client.command("script.evaluate", {}, 30_000);
        assert.equal(client.pending.size, 1);
        client.close();
        await assert.rejects(pending, /client closed/);
        assert.equal(client.pending.size, 0);
        assert.equal(destroyed, 1);
        assert.equal(sent.length, 1);
        client.close();
        assert.equal(destroyed, 1, "shutdown is idempotent");
        await assert.rejects(client.command("session.status"), /client closed/);
    },
);

test("WebDriver socket write failures redact private request data",
    async () => {
        const marker = "private-command-write-marker-8442";
        const client = new BidiClient("ws://127.0.0.1:1234/session");
        let destroyed = 0;
        let writes = 0;
        client.socket = {
            write: () => {
                writes += 1;
                if (writes === 2) {
                    throw new Error(`socket rejected ${marker}`);
                }
            },
            destroy: () => { destroyed += 1; },
        };
        const waiting = client.command("session.status", {}, 30_000);
        const redacted = (error) => {
            assert.equal(error.message.includes(marker), false);
            assert.equal("cause" in error, false);
            assert.match(error.message, /WebDriver BiDi socket write failed/);
            return true;
        };
        const waitingRejected = assert.rejects(waiting, redacted);
        const failed = client.command("script.evaluate", {
            expression: marker,
        }, 30_000);
        const failedRejected = assert.rejects(failed, redacted);
        await Promise.all([waitingRejected, failedRejected]);
        assert.equal(writes, 2);
        assert.equal(client.pending.size, 0);
        assert.equal(client.closed, true);
        assert.equal(destroyed, 1);
    },
);

test("WebDriver ping-write failures redact private remote payloads",
    async () => {
        const marker = "private-ping-write-marker-9231";
        const client = new BidiClient("ws://127.0.0.1:1234/session");
        let failWrite = false;
        let destroyed = 0;
        client.socket = {
            write: () => {
                if (failWrite) {
                    throw new Error(`socket rejected ${marker}`);
                }
            },
            destroy: () => { destroyed += 1; },
        };
        const pending = client.command("session.status", {}, 30_000);
        failWrite = true;
        const ping = Buffer.from(marker);
        client.buffer = Buffer.concat([
            Buffer.from([0x89, ping.length]), ping,
        ]);
        client.drain();
        await assert.rejects(pending, (error) => {
            assert.equal(error.message.includes(marker), false);
            assert.equal("cause" in error, false);
            assert.match(error.message, /WebDriver BiDi socket write failed/);
            return true;
        });
        assert.equal(client.pending.size, 0);
        assert.equal(client.closed, true);
        assert.equal(destroyed, 1);
    },
);

test("WebDriver peer disconnect rejects its pending command immediately",
    { timeout: 5_000 },
    async () => {
        const peers = new Set();
        const server = net.createServer((peer) => {
            peers.add(peer);
            peer.once("data", (request) => {
                const key = request.toString("latin1").match(
                    /Sec-WebSocket-Key: ([^\r\n]+)/i,
                )?.[1];
                if (key === undefined) {
                    peer.destroy();
                    return;
                }
                const accepted = crypto.createHash("sha1")
                    .update(`${key}258EAFA5-E914-47DA-95CA-C5AB0DC85B11`)
                    .digest("base64");
                peer.write([
                    "HTTP/1.1 101 Switching Protocols",
                    "Upgrade: websocket",
                    "Connection: Upgrade",
                    `Sec-WebSocket-Accept: ${accepted}`,
                    "", "",
                ].join("\r\n"));
            });
            peer.on("close", () => peers.delete(peer));
        });
        server.listen(0, "127.0.0.1");
        await once(server, "listening");
        const address = server.address();
        assert.notEqual(address, null);
        assert.notEqual(typeof address, "string");
        const client = new BidiClient(`ws://127.0.0.1:${address.port}/session`);
        try {
            await client.connect();
            const command = client.command("session.status", {}, 30_000);
            assert.equal(client.pending.size, 1);
            for (const peer of peers) {
                peer.destroy();
            }
            await assert.rejects(
                command,
                /socket closed before command|socket failed/,
            );
            assert.equal(client.pending.size, 0);
            assert.equal(client.closed, true);
        } finally {
            client.close();
            for (const peer of peers) {
                peer.destroy();
            }
            await closeTcpFixtureServer(server);
        }
    },
);

test("WebDriver socket error events redact private peer exceptions",
    { timeout: 5_000 },
    async () => {
        const marker = "private-bidi-socket-error-marker-4239";
        const peers = new Set();
        const server = net.createServer((peer) => {
            peers.add(peer);
            peer.once("data", (request) => {
                const key = request.toString("latin1").match(
                    /Sec-WebSocket-Key: ([^\r\n]+)/i,
                )?.[1];
                if (key === undefined) {
                    peer.destroy();
                    return;
                }
                const accepted = crypto.createHash("sha1")
                    .update(`${key}258EAFA5-E914-47DA-95CA-C5AB0DC85B11`)
                    .digest("base64");
                peer.write([
                    "HTTP/1.1 101 Switching Protocols",
                    "Upgrade: websocket",
                    "Connection: Upgrade",
                    `Sec-WebSocket-Accept: ${accepted}`,
                    "", "",
                ].join("\r\n"));
            });
            peer.on("close", () => peers.delete(peer));
        });
        server.listen(0, "127.0.0.1");
        await once(server, "listening");
        const address = server.address();
        assert.notEqual(address, null);
        assert.notEqual(typeof address, "string");
        const client = new BidiClient(`ws://127.0.0.1:${address.port}/session`);
        try {
            await client.connect();
            const command = client.command("script.evaluate", {
                expression: marker,
            }, 30_000);
            assert.equal(client.pending.size, 1);
            client.socket.emit("error", new Error(`socket failed ${marker}`));
            await assert.rejects(command, (error) => {
                assert.equal(error.message.includes(marker), false);
                assert.equal("cause" in error, false);
                assert.match(error.message, /WebDriver BiDi socket failed/);
                return true;
            });
            assert.equal(client.pending.size, 0);
            assert.equal(client.closed, true);
            assert.equal(client.socket.destroyed, true);
        } finally {
            client.close();
            for (const peer of peers) {
                peer.destroy();
            }
            await closeTcpFixtureServer(server);
        }
    },
);

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

async function waitForWorkspaceDocument(client, context) {
    let documentState = null;
    for (let attempt = 0; attempt < 40; attempt += 1) {
        documentState = await evaluateDocument(client, context);
        if (documentState.hasControl === false
            && documentState.hasWorkspace === true
            && documentState.title === "Atrament") {
            return documentState;
        }
        await new Promise((resolve) => setTimeout(resolve, 250));
    }
    // Document title and remote fields may contain private page content.
    throw new Error("workspace navigation did not settle");
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
    // A failed frame is not permission to print remote document metadata.
    throw new Error("framed navigation did not settle");
}

async function navigateFromDocument(client, context, url) {
    await client.command("script.evaluate", {
        awaitPromise: false,
        expression: `location.assign(${JSON.stringify(url)})`,
        resultOwnership: "none",
        target: { context },
    });
}

test("Firefox direct workspace navigation survives transient blanks",
    async () => {
    const source = fs.readFileSync(new URL(import.meta.url), "utf8");
    const start = source.indexOf("async function evaluateDocument(");
    const end = source.indexOf("async function waitForChildContext(", start);
    assert.ok(start >= 0 && end > start);
    const wait = runInNewContext(
        `${source.slice(start, end)}; waitForWorkspaceDocument`,
        {
            assert,
            setTimeout(callback) { callback(); },
        },
    );
    const blank = { hasControl: false, hasWorkspace: false, title: "" };
    const workspace = {
        hasControl: false,
        hasWorkspace: true,
        title: "Atrament",
    };
    let observed = 0;
    const client = {
        async command(method) {
            assert.equal(method, "script.evaluate");
            observed += 1;
            return {
                type: "success",
                result: {
                    type: "string",
                    value: JSON.stringify(observed <= 21 ? blank : workspace),
                },
            };
        },
    };
    assert.equal(
        JSON.stringify(await wait(client, "direct-context")),
        JSON.stringify(workspace),
    );
    assert.equal(observed, 22);

    // An unrelated document must not satisfy readiness by merely having a
    // title; missing workspace content remains a bounded failure.
    const unrelated = {
        hasControl: true,
        hasWorkspace: false,
        title: "Permissive frame",
    };
    await assert.rejects(
        wait({
            async command() {
                return {
                    type: "success",
                    result: {
                        type: "string",
                        value: JSON.stringify(unrelated),
                    },
                };
            },
        }, "wrong-document"),
        /workspace navigation did not settle/,
    );
});

test("browser readiness failures redact untrusted page fields", async () => {
    const source = fs.readFileSync(new URL(import.meta.url), "utf8");
    const documentStart = source.indexOf("async function evaluateDocument(");
    const documentEnd = source.indexOf("async function navigateFromDocument(",
        documentStart);
    const sessionStart = source.lastIndexOf(
        "async function evaluateSessionDocument(");
    const sessionEnd = source.indexOf("async function performTrustedKey(",
        sessionStart);
    assert.ok(documentStart >= 0 && documentEnd > documentStart);
    assert.ok(sessionStart >= 0 && sessionEnd > sessionStart);
    const context = {
        assert,
        setTimeout(callback) { callback(); },
    };
    const documentWaiters = runInNewContext(
        `${source.slice(documentStart, documentEnd)};` +
            `({ waitForWorkspaceDocument, waitForFramedDocument })`,
        context,
    );
    const waitForSession = runInNewContext(
        `${source.slice(sessionStart, sessionEnd)}; waitForSessionStatus`,
        context,
    );
    const privateText = `test-private-readiness-` +
        crypto.randomBytes(16).toString("hex");
    const remote = (state) => ({
        async command(method) {
            assert.equal(method, "script.evaluate");
            return {
                type: "success",
                result: { type: "string", value: JSON.stringify(state) },
            };
        },
    });
    const failedSession = remote({
        hash: `#session=${privateText}`,
        status: privateText,
        task: { disabled: false, value: privateText },
        source: { disabled: false, value: privateText },
        candidate: { disabled: false, value: privateText },
    });
    const failedDocument = remote({
        title: privateText,
        hasControl: false,
        hasWorkspace: false,
    });
    const failedFrame = remote({
        title: "",
        hasControl: false,
        hasWorkspace: false,
        extraUntrustedField: privateText,
    });
    for (const startWait of [
        () => waitForSession(failedSession, "tab",
            `${privateText}-expected`, 2),
        () => documentWaiters.waitForWorkspaceDocument(failedDocument, "tab"),
        () => documentWaiters.waitForFramedDocument(failedFrame, "tab"),
    ]) {
        await assert.rejects(startWait(), (error) => {
            assert.equal(error.message.includes(privateText), false,
                "timeout messages must never reflect browser page content");
            assert.equal(JSON.stringify(error).includes(privateText), false);
            assert.equal("actual" in error, false);
            assert.equal("expected" in error, false);
            return true;
        });
    }
});

test("observedWithin releases timers after settlement", async () => {
    const source = fs.readFileSync(new URL(import.meta.url), "utf8");
    const start = source.lastIndexOf("async function observedWithin(");
    const end = source.indexOf("async function initializeBidiSession(", start);
    assert.ok(start >= 0 && end > start);
    const timers = new Map();
    let nextTimerId = 0;
    const observed = runInNewContext(
        `${source.slice(start, end)}; observedWithin`,
        {
            Error,
            Promise,
            setTimeout(callback, duration) {
                assert.equal(duration, 5_000);
                const id = ++nextTimerId;
                timers.set(id, callback);
                return id;
            },
            clearTimeout(id) { timers.delete(id); },
        },
    );
    assert.equal(await observed(Promise.resolve("seen"), "early"), "seen");
    assert.equal(timers.size, 0, "resolved observation must clear timer");

    const failure = new Error("request failed early");
    await assert.rejects(observed(Promise.reject(failure), "error"),
        (error) => error === failure);
    assert.equal(timers.size, 0, "rejected observation must clear timer");

    const timedOut = observed(new Promise(() => {}), "late");
    assert.equal(timers.size, 1);
    [...timers.values()][0]();
    await assert.rejects(timedOut, /late request was not observed/);
    assert.equal(timers.size, 0, "expired timer must be cleared");
});

async function observedWithin(observation, label) {
    let timer;
    const deadline = new Promise((_resolve, reject) => {
        timer = setTimeout(() => {
            reject(new Error(`${label} request was not observed`));
        }, 5_000);
    });
    try {
        return await Promise.race([observation, deadline]);
    } finally {
        clearTimeout(timer);
    }
}

async function initializeBidiSession(bidi) {
    await bidi.connect();
    await bidi.command(
        "session.new",
        { capabilities: { alwaysMatch: {} } },
        FIREFOX_BIDI_READY_TIMEOUT_MS,
    );
}

test("Firefox BiDi startup applies the same finite readiness budget",
    async () => {
        const events = [];
        const bidi = {
            connect: async () => { events.push(["connect"]); },
            command: async (method, params, timeout) => {
                events.push(["command", method, timeout]);
                assert.deepEqual(params,
                    { capabilities: { alwaysMatch: {} } });
            },
        };
        await initializeBidiSession(bidi);
        assert.deepEqual(events, [
            ["connect"],
            ["command", "session.new", FIREFOX_BIDI_READY_TIMEOUT_MS],
        ]);
    },
);

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
        {
            detached: process.platform === "linux",
            stdio: ["ignore", "ignore", "pipe"],
        },
    );
    firefox.fixtureProcessGroup = process.platform === "linux";
    trackFirefoxFixtureGroup(firefox);
    children.push(firefox);
    const remote = await waitForLine(firefox.stderr, (line) => {
        const prefix = "WebDriver BiDi listening on ";
        return line.startsWith(prefix)
            ? line.slice(prefix.length)
            : null;
    }, FIREFOX_BIDI_READY_TIMEOUT_MS, firefox);
    const bidi = new BidiClient(`${remote}/session`);
    try {
        await initializeBidiSession(bidi);
        return bidi;
    } catch (error) {
        // The caller only receives successfully initialized clients. Keep
        // partially initialized WebDriver sockets owned by this helper.
        bidi.close();
        throw error;
    }
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
        "paper-profile.js",
        "handwriting-profile.js",
        "glyph-strokes.js",
        "markdown-export.js",
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

function createSessionFixtureServer(
    apiRequests,
    onDraftMutation = null,
    onSessionEnd = null,
    onHandshake = null,
    onDraftRead = null,
    onDraftContentType = null,
    onHandshakeContentType = null,
    onDraftBody = null,
    onHandshakeBody = null,
    onHandshakeResponse = null,
    onDraftResponse = null,
    onEndResponse = null,
) {
    const generated = new Set([
        "main.js",
        "session-diagnostic.js",
        "session-draft.js",
        "session-fragment.js",
        "session-handshake.js",
        "paper-profile.js",
        "handwriting-profile.js",
        "glyph-strokes.js",
        "markdown-export.js",
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
        if (pathname === "/api/session/end") {
            apiRequests.push(`${request.method} ${pathname}`);

            if (
                request.method !== "POST"
                || request.headers.authorization
                    !== `Bearer ${REFRESH_SECRET}`
                || request.headers.origin
                    !== `http://${request.headers.host}`
                || Number(request.headers["content-length"] ?? 0) !== 0
            ) {
                response.writeHead(401);
                response.end();
                return;
            }
            if (onEndResponse?.(request, response) === true) {
                return;
            }
            const status = onSessionEnd?.() ?? 204;
            response.writeHead(status, { "Cache-Control": "no-store" });
            response.end();
            return;
        }
        if (pathname === "/api/handshake") {
            apiRequests.push(`${request.method} ${pathname}`);

            if (
                request.method !== "POST"
                || request.headers.authorization
                    !== `Bearer ${REFRESH_SECRET}`
                || request.headers.origin
                    !== `http://${request.headers.host}`
            ) {
                response.writeHead(401);
                response.end();
                return;
            }
            const status = onHandshake?.() ?? 200;
            if (onHandshakeResponse?.(request, response, status) === true) {
                return;
            }
            response.writeHead(status, {
                "Cache-Control": "no-store",
                "Content-Type": onHandshakeContentType?.()
                    ?? "application/json; charset=utf-8",
            });
            response.end(status === 200
                ? onHandshakeBody?.() ?? COMPATIBLE_HANDSHAKE : "");
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
            if (
                request.method === "POST"
                && request.headers.origin
                    !== `http://${request.headers.host}`
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
            const status = onDraftRead?.(field) ?? 200;
            if (onDraftResponse?.(request, response, field, status) === true) {
                return;
            }
            const contentType = onDraftContentType?.(field)
                ?? "text/plain; charset=utf-8";
            response.writeHead(status, {
                "Cache-Control": "no-store",
                "Content-Type": contentType,
            });
            response.end(status === 200
                ? (onDraftBody?.(field) ?? REFRESH_DRAFT[field])
                : "");
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

async function waitForSessionStatus(client, context, expected, attempts = 100) {
    let state = null;
    for (let attempt = 0; attempt < attempts; attempt += 1) {
        state = await evaluateSessionDocument(client, context);
        if (state.status === expected) {
            return state;
        }
        await new Promise((resolve) => setTimeout(resolve, 25));
    }
    // The observed hash and draft fields are untrusted session-private data.
    // Even the caller's expected label is unnecessary in diagnostic output.
    throw new Error("session status did not reach expected state");
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
    "Firefox retains full Rust u64 JSON seeds in a numeric text field",
    { skip: !FIREFOX_AVAILABLE, timeout: FIREFOX_CASE_TIMEOUT_MS },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/workspace-u64-");
        const children = [];
        let bidi;
        try {
            bidi = await startBidiFirefox(profile, children);
            const tab = await bidi.command("browsingContext.create", {
                type: "tab",
            });
            await bidi.command("browsingContext.navigate", {
                context: tab.context,
                url: "about:blank",
                wait: "complete",
            });
            const remote = await bidi.command("script.evaluate", {
                awaitPromise: false,
                expression: `JSON.stringify((() => {
                    const maximum = "18446744073709551615";
                    const parsed = JSON.parse('{"seed":' + maximum + '}',
                        (key, value, context) =>
                            key === "seed" ? context?.source : value);
                    const field = document.createElement("input");
                    field.type = "text";
                    field.inputMode = "numeric";
                    field.value = parsed.seed;
                    return {
                        seed: parsed.seed,
                        field: field.value,
                        sent: '{"seed":' + field.value + '}',
                    };
                })())`,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.equal(remote.result?.type, "string");
            assert.deepEqual(JSON.parse(remote.result.value), {
                seed: "18446744073709551615",
                field: "18446744073709551615",
                sent: '{"seed":18446744073709551615}',
            });
        } finally {
            bidi?.close();
            for (const child of children.reverse()) {
                await stopChild(child);
            }
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox edits, saves and reopens both profiles through Rust localhost",
    { skip: !FIREFOX_AVAILABLE, timeout: 110_000 },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const root = fs.mkdtempSync(".temp/profile-journey-");
        const browserProfile = path.join(root, "firefox");
        const profiles = path.join(root, "profiles");
        const bin = path.join(root, "bin");
        const capturedUrlFile = path.join(root, "test-launch-url");
        for (const dir of [browserProfile, profiles, bin]) {
            fs.mkdirSync(dir, { mode: 0o700 });
        }
        // This test owns both the opener and its disposable session. Never
        // publish the captured fragment or reuse an unrelated live session.
        fs.writeFileSync(path.join(bin, "xdg-open"),
            '#!/bin/sh\numask 077\nprintf "%s\\n" "$1" > '
            + '"$ATRAMENT_TEST_LAUNCH_URL_FILE"\n', { mode: 0o700 });
        const children = [];
        let bidi;
        try {
            const build = spawnSync(PINNED_CARGO, [
                "build", "--offline", "--quiet", "-p",
                "atrament_runtime_bootstrap", "--bin", "atrament",
            ], {
                env: { ...process.env, CARGO_TARGET_DIR: TARGET_DIR },
                stdio: "inherit",
            });
            assert.equal(build.status, 0, "Rust localhost binary builds");
            const runtimeEnv = {
                ...process.env,
                DISPLAY: ":atrament-test",
                PATH: `${bin}:${process.env.PATH ?? ""}`,
                ATRAMENT_TEST_LAUNCH_URL_FILE: capturedUrlFile,
            };
            delete runtimeEnv.WAYLAND_DISPLAY;
            const runtime = spawn(RUNTIME_BINARY,
                ["--profiles-dir", profiles], {
                    env: runtimeEnv,
                    stdio: ["ignore", "pipe", "pipe"],
                });
            children.push(runtime);
            const origin = await waitForLine(runtime.stdout, (line) => {
                try {
                    const item = JSON.parse(line);
                    return item.state === "ready" ? item.origin : null;
                } catch { return null; }
            }, 15_000, runtime);
            const launchUrl = fs.readFileSync(capturedUrlFile, "utf8").trim();
            fs.unlinkSync(capturedUrlFile);
            assert.equal(launchUrl.startsWith(`${origin}#session=`), true);
            assert.equal(/^http:\/\/127\.0\.0\.1:\d+#session=[a-f0-9]{64}$/
                .test(launchUrl), true, "private test session URL is bounded");
            bidi = await startBidiFirefox(browserProfile, children);
            const tab = await bidi.command("browsingContext.create", {
                type: "tab",
            });
            await bidi.command("browsingContext.navigate", {
                context: tab.context, url: launchUrl, wait: "complete",
            });
            await waitForSessionStatus(bidi, tab.context, "Session ready");

            async function browserScript(expression) {
                const reply = await bidi.command("script.evaluate", {
                    expression, awaitPromise: false,
                    resultOwnership: "none", target: { context: tab.context },
                });
                assert.equal(reply.type, "success");
                assert.equal(reply.result.type, "string");
                return JSON.parse(reply.result.value);
            }
            async function waitStatus(id, expected) {
                for (let attempt = 0; attempt < 100; attempt += 1) {
                    const value = await browserScript(
                        `JSON.stringify(document.getElementById(`
                        + `${JSON.stringify(id)}).textContent)`);
                    if (value === expected) return;
                    await new Promise((resolve) => setTimeout(resolve, 50));
                }
                throw new Error("Rust-backed profile action did not settle");
            }
            await waitStatus("handwriting-status",
                "Native statistics loaded · edit, validate, "
                + "and download JSON. No glyph rendering yet.");
            await waitStatus("paper-status",
                "Paper profile loaded · edit fields, then save or export.");
            await waitStatus("notebook-status",
                "Edit metadata, then save JSON.");

            const variationState = await browserScript(`JSON.stringify((() => {
                const field = (id, value) => {
                    const input = document.getElementById(id);
                    input.value = value;
                    input.dispatchEvent(new Event("input", { bubbles: true }));
                };
                const svg = document.getElementById("glyph-variation-preview");
                const ink = () => [...svg.querySelectorAll("line")].map(
                    (line) => ["x1", "y1", "x2", "y2", "stroke-width"]
                        .map((axis) => line.getAttribute(axis)).join(","),
                );
                const initiallyDisabled = document.getElementById(
                    "glyph-variation-seed"
                ).disabled;
                field("glyph-character", "ó");
                field("glyph-form", "ó");
                field("glyph-circle-radius", "800");
                document.getElementById("glyph-construct").click();
                const initial = ink();
                field("glyph-variation-seed", "123");
                const resampled = ink();
                field("glyph-variation-width", "21");
                const refused = svg.querySelectorAll("line").length;
                const warning = document.getElementById(
                    "glyph-variation-status"
                ).textContent;
                return {
                    initiallyDisabled,
                    initialCount: initial.length,
                    idealUnchanged: initial.slice(0, 147).join(";")
                        === resampled.slice(0, 147).join(";"),
                    variedChanged: initial.slice(147).join(";")
                        !== resampled.slice(147).join(";"),
                    refused,
                    warning,
                    authoredCount: document.getElementById(
                        "glyph-preview"
                    ).querySelectorAll("line").length
                };
            })())`);
            assert.equal(variationState.initiallyDisabled, false);
            assert.equal(variationState.initialCount, 294);
            assert.equal(variationState.idealUnchanged, true);
            assert.equal(variationState.variedChanged, true);
            assert.equal(variationState.refused, 0);
            assert.match(variationState.warning, /outside 0\.\.20/);
            assert.equal(variationState.authoredCount, 49);

            // Exercise both a numeric input and its slider, plus an exact
            // Spanish grapheme override. The browser never owns admission.
            await browserScript(`JSON.stringify((() => {
                const edit = (id, value, type = "input") => {
                    const el = document.getElementById(id);
                    el.value = value;
                    el.dispatchEvent(new Event(type, { bubbles: true }));
                };
                edit("handwriting-seed", "18446744073709551615");
                edit("handwriting-dimension", "height", "change");
                edit("handwriting-scope", "glyph", "change");
                edit("handwriting-new-glyph", "ñ");
                document.getElementById("handwriting-add-glyph").click();
                edit("handwriting-minimum", "2600");
                edit("handwriting-maximum", "3400");
                edit("handwriting-mean-range", "3100");
                edit("handwriting-median", "3000");
                edit("handwriting-standard_deviation-range", "60");
                edit("handwriting-scope", "global", "change");
                edit("handwriting-dimension", "width", "change");
                edit("handwriting-minimum", "2100");
                edit("handwriting-maximum", "2500");
                edit("handwriting-mean-range", "2300");
                edit("handwriting-median", "2300");
                edit("handwriting-standard_deviation-range", "80");
                edit("handwriting-scope", "glyph", "change");
                edit("handwriting-dimension", "height", "change");
                edit("notebook-date-style", "line");
                edit("notebook-date-placement", "top-left");
                edit("notebook-date-text", "2026-11-03");
                edit("notebook-radius-range", "3");
                document.getElementById("notebook-page-numbers").checked = true;
                document.getElementById("notebook-printed-sheet")
                    .checked = true;
                document.getElementById("handwriting-local-name").value =
                    "journey.json";
                document.getElementById("notebook-local-name").value =
                    "journey.json";
                document.getElementById("handwriting-local-save").click();
                return true;
            })())`);
            await waitStatus("handwriting-status",
                "Native handwriting saved in local directory.");
            await browserScript(`JSON.stringify((() => {
                const spacing = document.getElementById("paper-spacing-range");
                spacing.value = "6";
                spacing.dispatchEvent(new Event(
                    "input", { bubbles: true }));
                const upper = document.getElementById("paper-upper");
                upper.value = "25";
                upper.dispatchEvent(new Event(
                    "input", { bubbles: true }));
                document.getElementById("notebook-use-paper").click();
                return true;
            })())`);
            await waitStatus("notebook-status",
                "Current sheet settings validated.");
            await browserScript(`JSON.stringify((() => {
                document.getElementById("notebook-local-save").click();
                return true;
            })())`);
            await waitStatus("notebook-status",
                "Native notebook saved in selected local directory.");
            const handwritingFile = path.join(profiles,
                "handwriting-journey.json");
            const notebookFile = path.join(profiles, "notebook-journey.json");
            const nativeHandwriting = fs.readFileSync(handwritingFile, "utf8");
            const nativeNotebook = JSON.parse(
                fs.readFileSync(notebookFile, "utf8"));
            // Check exact source bytes before converting bounded values to JS.
            assert.ok(nativeHandwriting.includes(
                '"seed":18446744073709551615'));
            const handwriting = JSON.parse(nativeHandwriting);
            const glyph = handwriting.glyphs.find(
                (item) => item.grapheme === "ñ");
            const height = glyph?.parameters.find(
                (entry) => entry.dimension === "height");
            assert.deepEqual(height, {
                dimension: "height", minimum: 2600, maximum: 3400,
                mean: 3100, median: 3000, standard_deviation: 60,
            });
            assert.deepEqual(handwriting.globals.find(
                (entry) => entry.dimension === "width"), {
                dimension: "width", minimum: 2100, maximum: 2500,
                mean: 2300, median: 2300, standard_deviation: 80,
            });
            assert.equal(nativeNotebook.border_radius_um, 3000);
            assert.equal(nativeNotebook.paper.spacing_um, 6000);
            assert.equal(nativeNotebook.paper.margin_top_um, 25000);
            assert.equal(nativeNotebook.date_style, "line");
            assert.equal(nativeNotebook.date_placement, "top-left");
            assert.equal(nativeNotebook.date_text, "2026-11-03");
            assert.equal(nativeNotebook.page_numbers, true);
            assert.equal(nativeNotebook.printed_sheet, true);

            // The exact notebook profile saved by the web editor must also
            // open in the separate Rust CLI and produce real paper-layer
            // PDF/PNG bytes. This is not a handwritten-content export.
            const cliBuild = spawnSync(PINNED_CARGO, [
                "build", "--offline", "--quiet",
                "-p", "atrament_profile_editor_cli",
                "--bin", "atrament-profiles",
            ], {
                env: { ...process.env, CARGO_TARGET_DIR: TARGET_DIR },
                stdio: "pipe",
            });
            assert.equal(cliBuild.status, 0, "profile CLI builds offline");
            const cli = path.join(TARGET_DIR, "debug", "atrament-profiles");
            // Capture the real explicit variant download in the admitted
            // browser, not a reimplementation of the UI export in Node.
            // Override the click only for this ephemeral glyph download;
            // restore browser APIs before the rest of the session journey.
            const savedVariation = await bidi.command("script.evaluate", {
                expression: `(async () => {
                    const create = URL.createObjectURL;
                    const click = HTMLAnchorElement.prototype.click;
                    let captured = null;
                    URL.createObjectURL = (blob) => {
                        captured = blob;
                        return create.call(URL, blob);
                    };
                    HTMLAnchorElement.prototype.click = function () {
                        if (this.download === "my-varied-glyph-strokes.tsv") {
                            return;
                        }
                        return click.call(this);
                    };
                    try {
                        const edit = (id, value) => {
                            const control = document.getElementById(id);
                            control.value = value;
                            control.dispatchEvent(new Event("input", {
                                bubbles: true
                            }));
                        };
                        edit("glyph-variation-width", "6");
                        edit("glyph-variation-sample", "2");
                        document.getElementById(
                            "glyph-variation-download"
                        ).click();
                        return captured ? await captured.text() : "";
                    } finally {
                        URL.createObjectURL = create;
                        HTMLAnchorElement.prototype.click = click;
                    }
                })()`,
                awaitPromise: true,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.equal(savedVariation.type, "success");
            assert.equal(savedVariation.result.type, "string");
            const variationText = savedVariation.result.value;
            assert.ok(variationText.startsWith(
                "atrament.glyph-strokes.v1\nó\t"));
            const variationFile = path.join(root, "browser-varied-glyph.tsv");
            fs.writeFileSync(variationFile, variationText);
            const readback = spawnSync(cli, [
                "glyphs", "--profile", variationFile,
            ], { encoding: "utf8", stdio: "pipe" });
            assert.equal(readback.status, 0);
            assert.equal(readback.stdout, variationText,
                "Rust accepts exact browser-selected sample bytes");
            const glyphAudit = spawnSync(cli, [
                "glyphs", "--profile", variationFile,
                "--audit-text", "ó",
            ], { encoding: "utf8", stdio: "pipe" });
            assert.equal(glyphAudit.status, 0);
            assert.match(glyphAudit.stdout, /Missing authored glyphs: 0/);
            assert.match(glyphAudit.stdout, /49 authored strokes/);
            for (const format of ["pdf", "png"]) {
                const output = path.join(root,
                    `browser-varied-glyph.${format}`);
                const rendered = spawnSync(cli, [
                    "notebook", "--printed-sheet", "true",
                    "--export-paper", format,
                    ...(format === "png"
                        ? ["--png-page", "1", "--dpi", "72"] : []),
                    "--glyph-file", variationFile,
                    "--place-text", "1", "25400", "30000", "óó",
                    "--output", output,
                ], { stdio: "pipe" });
                assert.equal(rendered.status, 0,
                    `native ${format} accepts selected browser glyph ink`);
                const bytes = fs.readFileSync(output);
                assert.ok(bytes.length > 100);
                assert.equal(format === "pdf"
                    ? bytes.subarray(0, 5).toString("ascii")
                    : bytes.subarray(0, 8).toString("hex"),
                format === "pdf" ? "%PDF-" : "89504e470d0a1a0a");
            }
            // Trusted keyboard input owns the Markdown textarea and Enter
            // download. The isolated spy observes the app-created Blob and
            // filename without focusing, editing or clicking any control.
            const decoratedSource = "# óó\n\nó ó  \nóó";
            await tabToControl(bidi, tab.context, "source-input");
            await performTrustedKeyChord(
                bidi, tab.context, "\uE009", "a");
            const keystrokes = [...decoratedSource].flatMap((char) => {
                const key = char === "\n" ? "\uE007" : char;
                return [
                    { type: "keyDown", value: key },
                    { type: "keyUp", value: key },
                ];
            });
            await bidi.command("input.performActions", {
                actions: [{
                    actions: keystrokes, id: "keyboard", type: "key",
                }],
                context: tab.context,
            });
            const typedSource = await browserScript(`JSON.stringify({
                active: document.activeElement?.id,
                value: document.getElementById("source-input").value,
                enabled: !document.getElementById(
                    "source-markdown-export").disabled
            })`);
            assert.deepEqual(typedSource, {
                active: "source-input", value: decoratedSource, enabled: true,
            });
            const installed = await bidi.command("script.evaluate", {
                awaitPromise: false,
                expression: `(() => {
                    const create = URL.createObjectURL;
                    const click = HTMLAnchorElement.prototype.click;
                    window.__trustedMarkdownSpy = {
                        blob: null, filename: null, create, click
                    };
                    URL.createObjectURL = function (blob) {
                        window.__trustedMarkdownSpy.blob = blob;
                        return create.call(URL, blob);
                    };
                    HTMLAnchorElement.prototype.click = function () {
                        window.__trustedMarkdownSpy.filename = this.download;
                    };
                    return true;
                })()`,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.equal(installed.type, "success");
            await tabToControl(bidi, tab.context,
                "source-markdown-export");
            await performTrustedKey(bidi, tab.context, "\uE007");
            const markdownResponse = await bidi.command("script.evaluate", {
                awaitPromise: true,
                expression: `(async () => {
                    const spy = window.__trustedMarkdownSpy;
                    const result = {
                        enabled: !document.getElementById(
                            "source-markdown-export").disabled,
                        filename: spy.filename,
                        text: spy.blob ? await spy.blob.text() : null,
                        bytes: spy.blob?.size,
                        mediaType: spy.blob?.type,
                    };
                    URL.createObjectURL = spy.create;
                    HTMLAnchorElement.prototype.click = spy.click;
                    delete window.__trustedMarkdownSpy;
                    return JSON.stringify(result);
                })()`,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.equal(markdownResponse.type, "success");
            assert.equal(markdownResponse.result.type, "string");
            const downloadedSource = JSON.parse(markdownResponse.result.value);
            assert.equal(downloadedSource.enabled, true);
            assert.equal(downloadedSource.filename, "atrament-source.md");
            assert.equal(downloadedSource.mediaType,
                "text/markdown; charset=utf-8");
            assert.equal(downloadedSource.text, decoratedSource);
            const markdownFile = path.join(root, "browser-source.md");
            fs.writeFileSync(markdownFile, downloadedSource.text);
            assert.equal(downloadedSource.bytes,
                fs.readFileSync(markdownFile).length);
            assert.deepEqual(fs.readFileSync(markdownFile),
                Buffer.from(decoratedSource, "utf8"),
                "source must pass through the browser Blob byte-for-byte");
            const decoratedBase = [
                "notebook", "--profile", notebookFile,
                "--handwriting-profile", handwritingFile,
                "--glyph-file", variationFile,
                "--heading-style", "marker",
                "--heading-color", "#F8DE91",
                "--flow-markdown-file", markdownFile,
            ];
            const decoratedPdf = path.join(root,
                "browser-saved-markdown.pdf");
            const decoratedPng = path.join(root,
                "browser-saved-markdown.png");
            for (const [format, output, extra] of [
                ["pdf", decoratedPdf, ["--pages", "1"]],
                ["png", decoratedPng, ["--png-page", "1",
                    "--dpi", "72"]],
            ]) {
                const rendered = spawnSync(cli, [
                    ...decoratedBase, "--export-paper", format,
                    ...extra, "--output", output,
                ], { stdio: "pipe", timeout: 20_000 });
                assert.equal(rendered.status, 0,
                    `browser-saved profiles render Markdown ${format}`);
            }
            const decoratedPdfBytes = fs.readFileSync(decoratedPdf);
            const decoratedPngBytes = fs.readFileSync(decoratedPng);
            const pdfInk = decoratedPdfBytes.toString("latin1");
            assert.match(pdfInk, /^%PDF-1\.4/);
            assert.match(pdfInk, /\/Count 1\b/);
            assert.match(pdfInk, /0\.973 0\.871 0\.569 RG/,
                "browser-saved Markdown must draw an actual marker band");
            assert.deepEqual(decoratedPngBytes.subarray(0, 8),
                Buffer.from("89504e470d0a1a0a", "hex"));
            assert.equal(decoratedPngBytes[25], 6,
                "native Markdown preview must remain transparent RGBA");
            const markerChunks = [];
            for (let offset = 8; offset + 12 <= decoratedPngBytes.length;) {
                const length = decoratedPngBytes.readUInt32BE(offset);
                if (decoratedPngBytes.toString("ascii", offset + 4,
                    offset + 8) === "IDAT") {
                    markerChunks.push(decoratedPngBytes.subarray(
                        offset + 8, offset + 8 + length));
                }
                offset += length + 12;
            }
            assert.ok(markerChunks.length > 0);
            const markerPixels = inflateSync(Buffer.concat(markerChunks));
            assert.ok(markerPixels.includes(Buffer.from([248, 222, 145, 255])),
                "transparent PNG must include real pastel heading pigment");
            const refusedMarkdown = spawnSync(cli, [
                ...decoratedBase.slice(0, -2),
                "--flow-markdown", "# óó\n\nmissing glyph z",
                "--export-paper", "pdf", "--pages", "1",
                "--output", decoratedPdf, "--replace",
            ], { stdio: "pipe", timeout: 20_000 });
            assert.notEqual(refusedMarkdown.status, 0,
                "unknown authored shape must refuse Markdown before commit");
            assert.deepEqual(fs.readFileSync(decoratedPdf),
                decoratedPdfBytes,
                "failed Markdown export must preserve approved PDF bytes");
            assert.equal(fs.readFileSync(variationFile, "utf8"),
                variationText, "native export never rewrites browser ink");
            assert.deepEqual(fs.readFileSync(markdownFile),
                Buffer.from(decoratedSource, "utf8"),
                "native rejection cannot rewrite downloaded source");

            // Printer intent remains separate from the Firefox save and
            // native export. Review its exact PDF bytes without CUPS access.
            const printBuild = spawnSync(PINNED_CARGO, [
                "build", "--offline", "--quiet",
                "-p", "atrament_notebook_paper_cli",
                "--bin", "atrament-print",
            ], {
                env: { ...process.env, CARGO_TARGET_DIR: TARGET_DIR },
                stdio: "pipe", timeout: 90_000,
            });
            assert.equal(printBuild.status, 0,
                "native print review CLI builds without network access");
            const printCli = path.join(TARGET_DIR, "debug", "atrament-print");
            const reviewedHash = crypto.createHash("sha256")
                .update(decoratedPdfBytes).digest("hex");
            const printReview = spawnSync(printCli, [
                "--pdf", decoratedPdf,
                "--printer", "Unconnected_Test_Queue",
                "--dry-run", "--expect-sha256", reviewedHash.toUpperCase(),
            ], { encoding: "utf8", stdio: "pipe", timeout: 20_000 });
            assert.equal(printReview.status, 0,
                "browser-backed PDF is admitted for explicit print review");
            assert.match(printReview.stdout,
                new RegExp(`SHA-256 ${reviewedHash}`));
            assert.match(printReview.stdout, /no job submitted/);
            assert.deepEqual(fs.readFileSync(decoratedPdf),
                decoratedPdfBytes, "dry-run cannot alter the reviewed PDF");

            // Inventory checks exact graphemes without falsely claiming
            // visible strokes or rendering fallback for math symbols.
            const inventory = spawnSync(cli, [
                "handwriting", "--profile", handwritingFile,
                "--inventory-text", "¿Qué? ¡Pingüino! «ñ» n\u0303 ∞",
            ], { encoding: "utf8", stdio: "pipe" });
            assert.equal(inventory.status, 0);
            assert.ok(inventory.stdout.includes(
                "\"ñ\": text-baseline / statistical-override"));
            assert.ok(inventory.stdout.includes(
                "\"n\u0303\": text-baseline / global-statistics"));
            assert.ok(inventory.stdout.includes(
                "\"∞\": outside-text-baseline / global-statistics"));
            assert.ok(inventory.stdout.includes(
                "membership does not imply renderability"));
            const pdfFile = path.join(root, "browser-saved-paper.pdf");
            const pngFile = path.join(root, "browser-saved-page-2.png");
            for (const [format, output, argumentsAfterFormat] of [
                ["pdf", pdfFile, ["--pages", "2"]],
                ["png", pngFile, [
                    "--dpi", "150", "--png-page", "2",
                ]],
            ]) {
                const rendered = spawnSync(cli, [
                    "notebook", "--profile", notebookFile,
                    "--export-paper", format,
                    ...argumentsAfterFormat, "--output", output,
                ], { stdio: "pipe" });
                assert.equal(rendered.status, 0,
                    "Rust CLI accepted browser-saved notebook JSON");
            }
            const pdfBytes = fs.readFileSync(pdfFile);
            const pngBytes = fs.readFileSync(pngFile);
            assert.ok(pdfBytes.subarray(0, 8).toString("ascii")
                .startsWith("%PDF-"));
            assert.ok(pdfBytes.toString("latin1").includes("/Count 2"));
            const refusedExport = spawnSync(cli, [
                "notebook", "--profile", notebookFile,
                "--export-paper", "pdf", "--pages", "2",
                "--output", pdfFile,
            ], { stdio: "pipe" });
            assert.notEqual(refusedExport.status, 0,
                "existing PDF cannot be overwritten without --replace");
            assert.deepEqual(fs.readFileSync(pdfFile), pdfBytes);
            assert.ok(pngBytes.subarray(0, 8).equals(
                Buffer.from("89504e470d0a1a0a", "hex")));
            assert.ok(pngBytes.readUInt32BE(16) > 0);
            assert.ok(pngBytes.readUInt32BE(20) >
                pngBytes.readUInt32BE(16));
            assert.deepEqual(JSON.parse(
                fs.readFileSync(notebookFile, "utf8")), nativeNotebook,
                "CLI rendering does not alter the browser-saved profile");

            // Unsaved UI edits must not pollute the checked disk profile.
            await browserScript(`JSON.stringify((() => {
                const seed = document.getElementById("handwriting-seed");
                seed.value = "7";
                seed.dispatchEvent(new Event("input", { bubbles: true }));
                const radius = document.getElementById("notebook-radius");
                radius.value = "0";
                radius.dispatchEvent(new Event("input", { bubbles: true }));
                document.getElementById("handwriting-local-load").click();
                return true;
            })())`);
            await waitStatus("handwriting-status",
                "Handwriting reopened from local Rust file.");
            await browserScript(`JSON.stringify((() => {
                document.getElementById("notebook-local-load").click();
                return true;
            })())`);
            await waitStatus("notebook-status",
                "Notebook reopened from local Rust profile file.");
            const reopened = await browserScript(`JSON.stringify((() => ({
                seed: document.getElementById("handwriting-seed").value,
                glyph: document.getElementById("handwriting-glyph").value,
                mean: document.getElementById("handwriting-mean").value,
                slider: document.getElementById("handwriting-mean-range").value,
                radius: document.getElementById("notebook-radius").value,
                dateText: document.getElementById("notebook-date-text").value,
                radiusSlider: document.getElementById(
                    "notebook-radius-range").value,
            }))())`);
            assert.deepEqual(reopened, {
                seed: "18446744073709551615", glyph: "ñ",
                mean: "3100", slider: "3100", radius: "3",
                dateText: "2026-11-03", radiusSlider: "3",
            });
            // Numeric handwriting is a Rust-only estimate; the actual
            // drawable page preview is Rust PNG, not browser CSS geometry.
            await browserScript(`JSON.stringify((() => {
                document.getElementById("handwriting-sample-glyph").value = "ñ";
                document.getElementById("handwriting-sample-occurrence")
                    .value = "18446744073709551615";
                document.getElementById("handwriting-sample").click();
                return true;
            })())`);
            await waitStatus("handwriting-status",
                "Native numeric preview generated from unsaved "
                + "statistics. No profile was saved.");
            const sampled = await browserScript(`JSON.stringify(
                document.getElementById(
                    "handwriting-sample-result").textContent)`);
            assert.ok(sampled.includes("ñ"));
            assert.ok(sampled.includes("#18446744073709551615"));
            await browserScript(`JSON.stringify((() => {
                const page = document.getElementById("notebook-png-page");
                page.value = "2";
                page.dispatchEvent(new Event("input", { bubbles: true }));
                const originalCreateObjectURL = URL.createObjectURL;
                URL.createObjectURL = (blob) => {
                    window.atramentTestPreviewBlob = blob;
                    return originalCreateObjectURL.call(URL, blob);
                };
                document.getElementById("notebook-preview").click();
                return true;
            })())`);
            await waitStatus("notebook-status",
                "Rendered PNG preview for page 2.");
            const previewed = await browserScript(`JSON.stringify((() => ({
                visible: !document.getElementById(
                    "notebook-preview-frame").hidden,
                naturalWidth: document.getElementById(
                    "notebook-preview-image").naturalWidth,
                naturalHeight: document.getElementById(
                    "notebook-preview-image").naturalHeight,
            }))())`);
            assert.equal(previewed.visible, true);
            assert.ok(previewed.naturalWidth > 0);
            assert.ok(previewed.naturalHeight > 0);
            // The browser preview and CLI output must be identical Rust PNG
            // bytes for the same saved profile, physical DPI and page index.
            const browserPng = await bidi.command("script.evaluate", {
                expression: `(async () => {
                    const bytes = await window.atramentTestPreviewBlob
                        .arrayBuffer();
                    const digest = await crypto.subtle.digest(
                        "SHA-256", bytes);
                    return Array.from(new Uint8Array(digest)).map(
                        (item) => item.toString(16).padStart(2, "0"),
                    ).join("");
                })()`,
                awaitPromise: true, resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.equal(browserPng.type, "success");
            assert.equal(browserPng.result.type, "string");
            assert.equal(browserPng.result.value,
                crypto.createHash("sha256").update(pngBytes).digest("hex"));
            await browserScript(`JSON.stringify((() => {
                const mean = document.getElementById("handwriting-mean");
                mean.value = "99999";
                mean.dispatchEvent(new Event("input", { bubbles: true }));
                document.getElementById("handwriting-local-save").click();
                return true;
            })())`);
            await waitStatus("handwriting-status",
                "Invalid statistics or unavailable Rust backend; "
                + "no file saved.");
            assert.equal(fs.readFileSync(handwritingFile, "utf8"),
                nativeHandwriting, "refused statistics leave file unchanged");
            assert.deepEqual(JSON.parse(fs.readFileSync(notebookFile, "utf8")),
                nativeNotebook);
        } finally {
            bidi?.close();
            for (const child of children.reverse()) await stopChild(child);
            fs.rmSync(root, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox without JavaScript keeps both static workspace surfaces visible",
    { skip: !FIREFOX_AVAILABLE, timeout: FIREFOX_CASE_TIMEOUT_MS },
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
            bidi?.close();
            await closeFixtureServer(server);
            for (const child of children.reverse()) {
                await stopChild(child);
            }
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox keeps hostile prompt and response text inert",
    { skip: !FIREFOX_AVAILABLE, timeout: FIREFOX_CASE_TIMEOUT_MS },
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
            // Requests may arrive in any order; only their exact set matters.
            assert.deepEqual([...requests].sort(), [
                "/",
                "/generated/handwriting-profile.js",
                "/generated/glyph-strokes.js",
                "/generated/markdown-export.js",
                "/generated/main.js",
                "/generated/paper-profile.js",
                "/generated/session-diagnostic.js",
                "/generated/session-draft.js",
                "/generated/session-fragment.js",
                "/generated/session-handshake.js",
                "/workspace.css",
            ].sort());
        } finally {
            bidi?.close();
            await closeFixtureServer(server);
            for (const child of children.reverse()) {
                await stopChild(child);
            }
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox preserves large Unicode prompt data and canonical line endings",
    { skip: !FIREFOX_AVAILABLE, timeout: FIREFOX_CASE_TIMEOUT_MS },
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

            // Copying 1.4 MB can exceed the generic 10-second BiDi command
            // budget under workstation CPU load, so bound this probe to 25s.
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
            }, 25_000);
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
            bidi?.close();
            await closeFixtureServer(server);
            for (const child of children.reverse()) {
                await stopChild(child);
            }
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox Copy prompt ignores stale writes and reports current failures",
    { skip: !FIREFOX_AVAILABLE, timeout: FIREFOX_CASE_TIMEOUT_MS },
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
            bidi?.close();
            await closeFixtureServer(server);
            for (const child of children.reverse()) {
                await stopChild(child);
            }
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox serializes repeated prompt copy requests without UI churn",
    { skip: !FIREFOX_AVAILABLE, timeout: FIREFOX_CASE_TIMEOUT_MS },
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
            bidi?.close();
            await closeFixtureServer(server);
            for (const child of children.reverse()) {
                await stopChild(child);
            }
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox trusted Tab order follows compact divider availability",
    { skip: !FIREFOX_AVAILABLE, timeout: FIREFOX_CASE_TIMEOUT_MS },
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
            bidi?.close();
            await closeFixtureServer(server);
            for (const child of children.reverse()) {
                await stopChild(child);
            }
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

// Keyboard paths use WebDriver's trusted key actions only. Read-only DOM
// inspection may locate the current focus but must never focus or click a
// control on behalf of the user.
async function tabToControl(client, context, target, reverse = false) {
    for (let step = 0; step < 180; step += 1) {
        if ((await activeElementIdentity(client, context)).id === target) {
            return;
        }
        if (reverse) {
            await performTrustedKeyChord(
                client, context, "\uE008", "\uE004",
            );
        } else {
            await performTrustedKey(client, context, "\uE004");
        }
    }
    throw new Error(`Trusted Tab traversal did not reach ${target}`);
}

async function replaceWithTrustedKeys(client, context, value) {
    await performTrustedKeyChord(client, context, "\uE009", "a");
    await client.command("input.performActions", {
        actions: [{
            actions: [...value].flatMap((character) => [
                { type: "keyDown", value: character },
                { type: "keyUp", value: character },
            ]),
            id: "keyboard",
            type: "key",
        }],
        context,
    });
}

test(
    "Firefox trusted keyboard saves and reopens Rust notebook and ink profiles",
    { skip: !FIREFOX_AVAILABLE, timeout: 110_000 },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const root = fs.mkdtempSync(".temp/keyboard-profile-journey-");
        const browserProfile = path.join(root, "firefox");
        const profiles = path.join(root, "profiles");
        const bin = path.join(root, "bin");
        const capturedUrlFile = path.join(root, "test-launch-url");
        for (const dir of [browserProfile, profiles, bin]) {
            fs.mkdirSync(dir, { mode: 0o700 });
        }
        fs.writeFileSync(path.join(bin, "xdg-open"),
            '#!/bin/sh\numask 077\nprintf "%s\\n" "$1" > '
            + '"$ATRAMENT_TEST_LAUNCH_URL_FILE"\n', { mode: 0o700 });
        const children = [];
        let bidi;
        try {
            const build = spawnSync(PINNED_CARGO, [
                "build", "--offline", "--quiet", "-p",
                "atrament_runtime_bootstrap", "--bin", "atrament",
            ], {
                env: { ...process.env, CARGO_TARGET_DIR: TARGET_DIR },
                stdio: "inherit",
            });
            assert.equal(build.status, 0, "native localhost binary builds");
            const runtimeEnv = {
                ...process.env,
                DISPLAY: ":atrament-keyboard-test",
                PATH: `${bin}:${process.env.PATH ?? ""}`,
                ATRAMENT_TEST_LAUNCH_URL_FILE: capturedUrlFile,
            };
            delete runtimeEnv.WAYLAND_DISPLAY;
            const runtime = spawn(RUNTIME_BINARY,
                ["--profiles-dir", profiles], {
                    env: runtimeEnv,
                    stdio: ["ignore", "pipe", "pipe"],
                });
            children.push(runtime);
            const origin = await waitForLine(runtime.stdout, (line) => {
                try {
                    const item = JSON.parse(line);
                    return item.state === "ready" ? item.origin : null;
                } catch { return null; }
            }, 15_000, runtime);
            const launchUrl = fs.readFileSync(capturedUrlFile, "utf8").trim();
            fs.unlinkSync(capturedUrlFile);
            assert.match(launchUrl,
                new RegExp(`^${origin.replaceAll(".", "\\.")}`
                    + "#session=[a-f0-9]{64}$"));
            bidi = await startBidiFirefox(browserProfile, children);
            const tab = await bidi.command("browsingContext.create", {
                type: "tab",
            });
            await bidi.command("browsingContext.navigate", {
                context: tab.context, url: launchUrl, wait: "complete",
            });
            await waitForSessionStatus(bidi, tab.context, "Session ready");

            async function read(id, property = "value") {
                const reply = await bidi.command("script.evaluate", {
                    expression: `JSON.stringify(document.getElementById(`
                        + `${JSON.stringify(id)})[`
                        + `${JSON.stringify(property)}])`,
                    awaitPromise: false,
                    resultOwnership: "none",
                    target: { context: tab.context },
                });
                assert.equal(reply.type, "success");
                assert.equal(reply.result.type, "string");
                return JSON.parse(reply.result.value);
            }
            async function waitStatus(id, expected) {
                for (let attempt = 0; attempt < 100; attempt += 1) {
                    if (await read(id, "textContent") === expected) return;
                    await new Promise((resolve) => setTimeout(resolve, 50));
                }
                throw new Error(`Local profile action did not settle: ${id}`);
            }
            const context = tab.context;
            await waitStatus("notebook-status",
                "Edit metadata, then save JSON.");
            await waitStatus("handwriting-status",
                "Native statistics loaded · edit, validate, "
                + "and download JSON. No glyph rendering yet.");

            await tabToControl(bidi, context, "notebook-date-style");
            await performTrustedKey(bidi, context, "\uE015");
            assert.equal(await read("notebook-date-style"), "line");
            await tabToControl(bidi, context, "notebook-date-placement");
            await performTrustedKey(bidi, context, "\uE015");
            assert.equal(await read("notebook-date-placement"), "top-left");
            await tabToControl(bidi, context, "notebook-date-text");
            await replaceWithTrustedKeys(bidi, context, "2026-11-03");
            await tabToControl(bidi, context, "notebook-local-name");
            await replaceWithTrustedKeys(bidi, context, "keyboard.json");
            await tabToControl(bidi, context, "notebook-local-save");
            await performTrustedKey(bidi, context, "\uE007");
            await waitStatus("notebook-status",
                "Native notebook saved in selected local directory.");
            const notebookFile = path.join(profiles, "notebook-keyboard.json");
            const savedNotebook = fs.readFileSync(notebookFile, "utf8");
            const notebook = JSON.parse(savedNotebook);
            assert.deepEqual((({ date_style, date_placement, date_text }) => ({
                date_style, date_placement, date_text,
            }))(notebook), {
                date_style: "line", date_placement: "top-left",
                date_text: "2026-11-03",
            });

            await tabToControl(bidi, context, "notebook-date-text", true);
            await replaceWithTrustedKeys(bidi, context, "2026-11-04");
            assert.equal(await read("notebook-date-text"), "2026-11-04");
            await tabToControl(bidi, context, "notebook-local-load");
            await performTrustedKey(bidi, context, "\uE007");
            await waitStatus("notebook-status",
                "Notebook reopened from local Rust profile file.");
            assert.equal(await read("notebook-date-text"), "2026-11-03");
            assert.equal(fs.readFileSync(notebookFile, "utf8"), savedNotebook);

            // Invalid calendar text must also refuse from an actual
            // keyboard-triggered Save, preserving the prior native file.
            await tabToControl(bidi, context, "notebook-date-text", true);
            await replaceWithTrustedKeys(bidi, context, "2026-02-30");
            await tabToControl(bidi, context, "notebook-local-save");
            await performTrustedKey(bidi, context, "\uE007");
            await waitStatus("notebook-status",
                "Notebook refused; nothing saved.");
            assert.equal(fs.readFileSync(notebookFile, "utf8"), savedNotebook);

            await tabToControl(bidi, context, "handwriting-seed");
            await replaceWithTrustedKeys(bidi, context, "12345");
            await tabToControl(bidi, context, "handwriting-local-name");
            await replaceWithTrustedKeys(bidi, context, "keyboard.json");
            await tabToControl(bidi, context, "handwriting-local-save");
            await performTrustedKey(bidi, context, "\uE007");
            await waitStatus("handwriting-status",
                "Native handwriting saved in local directory.");
            const inkFile = path.join(profiles, "handwriting-keyboard.json");
            const savedInk = fs.readFileSync(inkFile, "utf8");
            assert.equal(JSON.parse(savedInk).seed, 12345);

            await tabToControl(bidi, context, "handwriting-seed", true);
            await replaceWithTrustedKeys(bidi, context, "54321");
            assert.equal(await read("handwriting-seed"), "54321");
            await tabToControl(bidi, context, "handwriting-local-load");
            await performTrustedKey(bidi, context, "\uE007");
            await waitStatus("handwriting-status",
                "Handwriting reopened from local Rust file.");
            assert.equal(await read("handwriting-seed"), "12345");
            assert.equal(fs.readFileSync(inkFile, "utf8"), savedInk);

            // The maximum unsigned seed is exact. One unit above it is
            // refused by Rust rather than overwriting the saved profile.
            await tabToControl(bidi, context, "handwriting-seed", true);
            await replaceWithTrustedKeys(bidi, context,
                "18446744073709551616");
            await tabToControl(bidi, context, "handwriting-local-save");
            await performTrustedKey(bidi, context, "\uE007");
            await waitStatus("handwriting-status",
                "Invalid statistics or unavailable Rust backend; "
                + "no file saved.");
            assert.equal(fs.readFileSync(inkFile, "utf8"), savedInk);
        } finally {
            bidi?.close();
            for (const child of children.reverse()) await stopChild(child);
            fs.rmSync(root, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox trusted keyboard activates native skip links without workspace JS",
    { skip: !FIREFOX_AVAILABLE, timeout: FIREFOX_CASE_TIMEOUT_MS },
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
            bidi?.close();
            await closeFixtureServer(server);
            for (const child of children.reverse()) {
                await stopChild(child);
            }
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox trusted divider keys and zoom boundaries preserve keyboard focus",
    { skip: !FIREFOX_AVAILABLE, timeout: FIREFOX_CASE_TIMEOUT_MS },
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
            bidi?.close();
            await closeFixtureServer(server);
            for (const child of children.reverse()) {
                await stopChild(child);
            }
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox text-spacing override keeps short workspace controls reachable",
    { skip: !FIREFOX_AVAILABLE, timeout: FIREFOX_CASE_TIMEOUT_MS },
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
            bidi?.close();
            await closeFixtureServer(server);
            for (const child of children.reverse()) {
                await stopChild(child);
            }
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox 200 percent text keeps compact skip links viewport bounded",
    { skip: !FIREFOX_AVAILABLE, timeout: FIREFOX_CASE_TIMEOUT_MS },
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
            bidi?.close();
            await closeFixtureServer(server);
            for (const child of children.reverse()) {
                await stopChild(child);
            }
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox compaction moves focused divider to the visible source heading",
    { skip: !FIREFOX_AVAILABLE, timeout: FIREFOX_CASE_TIMEOUT_MS },
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
            bidi?.close();
            await closeFixtureServer(server);
            for (const child of children.reverse()) {
                await stopChild(child);
            }
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox pointer-capture fallback preserves focus until completed click",
    { skip: !FIREFOX_AVAILABLE, timeout: FIREFOX_CASE_TIMEOUT_MS },
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
            bidi?.close();
            await closeFixtureServer(server);
            for (const child of children.reverse()) {
                await stopChild(child);
            }
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox resize interrupts pointer capture without poisoning the next drag",
    { skip: !FIREFOX_AVAILABLE, timeout: FIREFOX_CASE_TIMEOUT_MS },
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
            bidi?.close();
            await closeFixtureServer(server);
            for (const child of children.reverse()) {
                await stopChild(child);
            }
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox divider ARIA changes only when compact state changes",
    { skip: !FIREFOX_AVAILABLE, timeout: FIREFOX_CASE_TIMEOUT_MS },
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
            bidi?.close();
            await closeFixtureServer(server);
            for (const child of children.reverse()) {
                await stopChild(child);
            }
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox compact breakpoint matches viewport CSS and divider state",
    { skip: !FIREFOX_AVAILABLE, timeout: FIREFOX_CASE_TIMEOUT_MS },
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
            bidi?.close();
            await closeFixtureServer(server);
            for (const child of children.reverse()) {
                await stopChild(child);
            }
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox viewport matrix keeps both workspace surfaces reachable",
    { skip: !FIREFOX_AVAILABLE, timeout: FIREFOX_CASE_TIMEOUT_MS },
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
            bidi?.close();
            await closeFixtureServer(server);
            for (const child of children.reverse()) {
                await stopChild(child);
            }
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox enforces localhost frame, referrer, and resource policy",
    { skip: !FIREFOX_AVAILABLE, timeout: FIREFOX_CASE_TIMEOUT_MS },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/frame-ancestors-");
        const children = [];
        let hostile;
        let outboundRequests = 0;
        const assetRequests = [];
        let workerRequests = 0;
        let fontRequests = 0;
        let mediaRequests = 0;
        let objectRequests = 0;
        let sniffControlRequests = 0;
        let sniffProtectedRequests = 0;
        let frameChildRequests = 0;
        let formSubmissions = 0;
        let permissive;
        let bidi;
        try {
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
            }, 15_000, runtime);
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

            // Rehost the actual runtime response-level CSP on a document
            // without a meta policy, so this also verifies header enforcement
            // independently from the workspace's existing meta CSP.
            const runtimeResponse = await fetch(origin);
            assert.equal(runtimeResponse.status, 200);
            const responseCsp = runtimeResponse.headers.get(
                "content-security-policy",
            );
            assert.equal(typeof responseCsp, "string");
            const responseNoSniff = runtimeResponse.headers.get(
                "x-content-type-options",
            );
            assert.equal(responseNoSniff, "nosniff");
            const responseXfo = runtimeResponse.headers.get("x-frame-options");
            assert.equal(responseXfo, "DENY");
            assert.equal(runtimeResponse.headers.get(
                "cross-origin-opener-policy",
            ), "same-origin");
            assert.equal(runtimeResponse.headers.get(
                "cross-origin-resource-policy",
            ), "same-origin");
            const responsePermissions = runtimeResponse.headers.get(
                "permissions-policy",
            );
            assert.equal(typeof responsePermissions, "string");
            const deniedPermissions = new Set(
                responsePermissions.split(", "),
            );
            for (const capability of [
                "ambient-light-sensor", "attribution-reporting",
                "browsing-topics", "captured-surface-control",
                "ch-ua-high-entropy-values", "clipboard-read",
                "compute-pressure",
                "identity-credentials-get", "on-device-speech-recognition",
                "otp-credentials", "private-state-token-issuance",
                "private-state-token-redemption",
                "deferred-fetch", "deferred-fetch-minimal", "gamepad",
                "idle-detection", "keyboard-map", "local-fonts",
                "picture-in-picture",
                "publickey-credentials-create", "publickey-credentials-get",
                "screen-wake-lock", "speaker-selection", "storage-access",
                "window-management",
            ]) {
                assert.equal(deniedPermissions.has(`${capability}=()`), true,
                    `${capability} must be disallowed at the HTTP boundary`);
            }
            assert.equal(deniedPermissions.has("clipboard-write=()"), false,
                "the explicit Copy prompt must remain available");
            await runtimeResponse.body?.cancel();

            let resolveControlForm;
            const controlFormSubmitted = new Promise((resolve) => {
                resolveControlForm = resolve;
            });
            // One small, valid mono PCM wave is enough to load metadata
            // without requiring autoplay permission or external fixtures.
            const wave = Buffer.alloc(44 + 1024, 128);
            wave.write("RIFF", 0);
            wave.writeUInt32LE(wave.length - 8, 4);
            wave.write("WAVEfmt ", 8);
            wave.writeUInt32LE(16, 16);
            wave.writeUInt16LE(1, 20);
            wave.writeUInt16LE(1, 22);
            wave.writeUInt32LE(8000, 24);
            wave.writeUInt32LE(8000, 28);
            wave.writeUInt16LE(1, 32);
            wave.writeUInt16LE(8, 34);
            wave.write("data", 36);
            wave.writeUInt32LE(wave.length - 44, 40);
            permissive = http.createServer((request, response) => {
                if (request.url?.startsWith("/csp-mime-") === true) {
                    const protectedScript =
                        request.url === "/csp-mime-protected.js";
                    if (protectedScript) {
                        sniffProtectedRequests += 1;
                    } else if (request.url === "/csp-mime-control.js") {
                        sniffControlRequests += 1;
                    } else {
                        response.writeHead(404);
                        response.end();
                        return;
                    }
                    response.writeHead(200, {
                        "Cache-Control": "no-store",
                        "Content-Type": "text/plain; charset=utf-8",
                        ...(protectedScript ? {
                            "X-Content-Type-Options": responseNoSniff,
                        } : {}),
                    });
                    response.end("window.atramentSniffProbe = true;");
                    return;
                }
                if (request.url === "/csp-font.woff2") {
                    fontRequests += 1;
                    response.writeHead(200, {
                        "Cache-Control": "no-store",
                        "Content-Type": "font/woff2",
                    });
                    // Deliberately malformed: this probes fetch admission,
                    // not the font decoder or font-display layout.
                    response.end("not-a-woff2-font");
                    return;
                }
                if (request.url === "/csp-object.svg") {
                    objectRequests += 1;
                    response.writeHead(200, {
                        "Cache-Control": "no-store",
                        "Content-Type": "image/svg+xml",
                    });
                    response.end(
                        '<svg xmlns="http://www.w3.org/2000/svg" ' +
                        'width="2" height="2"><circle r="1"/></svg>',
                    );
                    return;
                }
                if (request.url === "/csp-media.wav") {
                    mediaRequests += 1;
                    response.writeHead(200, {
                        "Cache-Control": "no-store",
                        "Content-Type": "audio/wav",
                    });
                    response.end(wave);
                    return;
                }
                if (request.url === "/csp-frame-child") {
                    frameChildRequests += 1;
                    response.writeHead(200, {
                        "Content-Type": "text/html; charset=utf-8",
                    });
                    response.end("<!doctype html><p>Frame child loaded</p>");
                    return;
                }
                if (request.url === "/csp-form-submit") {
                    formSubmissions += 1;
                    resolveControlForm(request.method);
                    response.writeHead(200, {
                        "Content-Type": "text/html; charset=utf-8",
                    });
                    response.end("<!doctype html><p>Form submitted</p>");
                    return;
                }
                if (request.url === "/csp-worker.js") {
                    workerRequests += 1;
                    response.writeHead(200, {
                        "Cache-Control": "no-store",
                        "Content-Type": "text/javascript; charset=utf-8",
                    });
                    response.end('self.postMessage("ready");');
                    return;
                }
                if (request.url === "/xfo-header-only") {
                    response.writeHead(200, {
                        "Cache-Control": "no-store",
                        "Content-Type": "text/html; charset=utf-8",
                        "X-Frame-Options": responseXfo,
                    });
                    response.end([
                        "<!doctype html><title>XFO protected frame</title>",
                        '<main class="control-frame">Blocked by XFO</main>',
                    ].join(""));
                    return;
                }
                if (request.url === "/csp-header-only") {
                    response.writeHead(200, {
                        "Cache-Control": "no-store",
                        "Content-Type": "text/html; charset=utf-8",
                        "Content-Security-Policy": responseCsp,
                    });
                    response.end(
                        "<!doctype html><title>Header-only CSP</title>",
                    );
                    return;
                }
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
                if (request.url === "/corp-css-control") {
                    response.writeHead(200, {
                        "Content-Type": "text/css; charset=utf-8",
                    });
                    response.end("body { color: black; }");
                    return;
                }
                if (request.url === "/csp-probe") {
                    outboundRequests += 1;
                    response.writeHead(200, {
                        "Access-Control-Allow-Origin": "*",
                        "Content-Type": "text/plain; charset=utf-8",
                    });
                    response.end("external response");
                    return;
                }
                const assetResponses = {
                    "/csp-script": [
                        "text/javascript; charset=utf-8",
                        "window.atramentCspProbeLoaded = true;",
                    ],
                    "/csp-image": [
                        "image/svg+xml",
                        '<svg xmlns="http://www.w3.org/2000/svg" '
                            + 'width="2" height="2"/>',
                    ],
                    "/csp-style": [
                        "text/css; charset=utf-8",
                        ".atrament-csp-probe { display: block; }",
                    ],
                };
                const asset = Object.hasOwn(assetResponses, request.url)
                    ? assetResponses[request.url] : undefined;
                if (asset !== undefined) {
                    assetRequests.push(request.url);
                    response.writeHead(200, { "Content-Type": asset[0] });
                    response.end(asset[1]);
                    return;
                }
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
                const frameUrl = request.url === "/xfo-control"
                    ? `${permissiveOrigin}/`
                    : request.url === "/xfo-protected"
                        ? `${permissiveOrigin}/xfo-header-only`
                        : `${target}/`;
                response.writeHead(200, {
                    "Cache-Control": "no-store",
                    "Content-Type": "text/html; charset=utf-8",
                });
                response.end([
                    "<!doctype html><title>Hostile frame</title>",
                    `<iframe src="${frameUrl}"></iframe>`,
                ].join(""));
            });
            hostile.listen(0, "127.0.0.1");
            await once(hostile, "listening");
            const hostileAddress = hostile.address();
            assert.notEqual(typeof hostileAddress, "string");
            assert.notEqual(hostileAddress, null);
            const hostileOrigin =
                `http://127.0.0.1:${hostileAddress.port}`;
            const externalTarget = JSON.stringify(
                `${hostileOrigin}/csp-probe`,
            );

            bidi = await startBidiFirefox(profile, children);

            // The unprotected control confirms that CORS permits the exact
            // cross-origin request used by the protected documents.
            const networkControl = await bidi.command(
                "browsingContext.create", { type: "tab" },
            );
            await bidi.command("browsingContext.navigate", {
                context: networkControl.context,
                url: permissiveOrigin,
                wait: "complete",
            });
            const allowed = await bidi.command("script.evaluate", {
                awaitPromise: true,
                expression: `fetch(${externalTarget}, {
                    cache: "no-store"
                }).then(response => response.text())`,
                resultOwnership: "none",
                target: { context: networkControl.context },
            });
            assert.equal(allowed.type, "success");
            assert.equal(allowed.result.value, "external response");
            assert.equal(outboundRequests, 1);

            const headerOnly = await bidi.command(
                "browsingContext.create", { type: "tab" },
            );
            await bidi.command("browsingContext.navigate", {
                context: headerOnly.context,
                url: `${permissiveOrigin}/csp-header-only`,
                wait: "complete",
            });
            const headerProbe = await bidi.command("script.evaluate", {
                awaitPromise: true,
                expression: `(async () => {
                    try {
                        await fetch(${externalTarget}, {
                            cache: "no-store"
                        });
                        return false;
                    } catch {
                        return true;
                    }
                })()`,
                resultOwnership: "none",
                target: { context: headerOnly.context },
            });
            assert.equal(headerProbe.type, "success");
            assert.equal(headerProbe.result.value, true,
                "response header alone must prevent cross-origin fetch");
            assert.equal(outboundRequests, 1,
                "no-meta document must not contact external endpoint");

            // Stylesheets ordinarily load cross-origin without CORS. The
            // runtime's CORP must block that embedding, while an otherwise
            // equivalent host resource without CORP remains loadable.
            const stylesheetProbe = (target) => `(async () => {
                const node = document.createElement("link");
                node.rel = "stylesheet";
                node.href = ${JSON.stringify(target)};
                return await new Promise((resolve) => {
                    const timer = setTimeout(() => resolve(false), 3000);
                    const finish = (loaded) => {
                        clearTimeout(timer);
                        node.remove();
                        resolve(loaded);
                    };
                    node.onload = () => finish(true);
                    node.onerror = () => finish(false);
                    document.head.append(node);
                });
            })()`;
            const checkStyle = async (target) => {
                const result = await bidi.command("script.evaluate", {
                    awaitPromise: true,
                    expression: stylesheetProbe(target),
                    resultOwnership: "none",
                    target: { context: networkControl.context },
                });
                assert.equal(result.type, "success");
                return result.result.value;
            };
            assert.equal(await checkStyle(
                `${hostileOrigin}/corp-css-control`,
            ), true, "control cross-origin stylesheet must load");
            assert.equal(await checkStyle(`${origin}/workspace.css`), false,
                "runtime CORP must deny cross-origin CSS embedding");

            // A different-origin opener must lose its cross-window handle
            // when the actual runtime document declares COOP same-origin.
            const openerProbe = (target) => `(async () => {
                const opened = window.open(${JSON.stringify(target)}, "_blank");
                if (opened === null) return "blocked";
                await new Promise(resolve => setTimeout(resolve, 600));
                const severed = opened.closed;
                if (!severed) opened.close();
                return severed;
            })()`;
            const checkOpener = async (target) => {
                const result = await bidi.command("script.evaluate", {
                    awaitPromise: true,
                    expression: openerProbe(target),
                    resultOwnership: "none",
                    target: { context: networkControl.context },
                });
                assert.equal(result.type, "success");
                return result.result.value;
            };
            assert.equal(await checkOpener(permissiveOrigin), false,
                "control same-origin popup must retain its opener handle");
            assert.equal(await checkOpener(origin), true,
                "COOP must sever a hostile opener's workspace handle");

            const assetTargets = JSON.stringify({
                image: `${hostileOrigin}/csp-image`,
                script: `${hostileOrigin}/csp-script`,
                style: `${hostileOrigin}/csp-style`,
            });
            const probeAssets = `(async () => {
                const urls = ${assetTargets};
                const loaded = {};
                for (const kind of ["script", "image", "style"]) {
                    loaded[kind] = await new Promise((resolve, reject) => {
                        const node = document.createElement(
                            kind === "image" ? "img" :
                                kind === "style" ? "link" : "script"
                        );
                        if (kind === "style") {
                            node.rel = "stylesheet";
                            node.href = urls[kind];
                        } else {
                            node.src = urls[kind];
                        }
                        const timer = setTimeout(() => reject(
                            new Error("CSP asset probe did not settle")
                        ), 5000);
                        node.onload = () => {
                            clearTimeout(timer);
                            resolve(true);
                        };
                        node.onerror = () => {
                            clearTimeout(timer);
                            resolve(false);
                        };
                        document.body.append(node);
                    });
                }
                return JSON.stringify(loaded);
            })()`;
            const controlAssets = await bidi.command("script.evaluate", {
                awaitPromise: true,
                expression: probeAssets,
                resultOwnership: "none",
                target: { context: networkControl.context },
            });
            assert.equal(controlAssets.type, "success");
            assert.deepEqual(JSON.parse(controlAssets.result.value), {
                image: true, script: true, style: true,
            }, "permissive document must load each cross-origin asset");
            assert.deepEqual(assetRequests, [
                "/csp-script", "/csp-image", "/csp-style",
            ]);
            const blockedAssets = await bidi.command("script.evaluate", {
                awaitPromise: true,
                expression: probeAssets,
                resultOwnership: "none",
                target: { context: headerOnly.context },
            });
            assert.equal(blockedAssets.type, "success");
            assert.deepEqual(JSON.parse(blockedAssets.result.value), {
                image: false, script: false, style: false,
            }, "header-only CSP must block foreign scripts, images and CSS");
            assert.deepEqual(assetRequests, [
                "/csp-script", "/csp-image", "/csp-style",
            ], "protected document must not emit cross-origin asset requests");

            // Both scripts are allowed by script-src 'self'. The same-origin
            // text/plain variant without nosniff must execute in Firefox;
            // with the exact runtime nosniff header it must be refused.
            const mimeProbe = (url) => `(async () => {
                delete window.atramentSniffProbe;
                const node = document.createElement("script");
                node.src = ${JSON.stringify(url)};
                const outcome = await new Promise((resolve) => {
                    const timer = setTimeout(() => resolve(false), 3000);
                    const finish = () => {
                        clearTimeout(timer);
                        resolve(window.atramentSniffProbe === true);
                    };
                    node.onload = finish;
                    node.onerror = finish;
                    document.body.append(node);
                });
                node.remove();
                return outcome;
            })()`;
            const controlSniff = await bidi.command("script.evaluate", {
                awaitPromise: true,
                expression: mimeProbe(
                    `${permissiveOrigin}/csp-mime-control.js`,
                ),
                resultOwnership: "none",
                target: { context: networkControl.context },
            });
            assert.equal(controlSniff.type, "success");
            assert.equal(controlSniff.result.value, true,
                "control script must execute without nosniff");
            assert.equal(sniffControlRequests, 1);
            const blockedSniff = await bidi.command("script.evaluate", {
                awaitPromise: true,
                expression: mimeProbe(
                    `${permissiveOrigin}/csp-mime-protected.js`,
                ),
                resultOwnership: "none",
                target: { context: headerOnly.context },
            });
            assert.equal(blockedSniff.type, "success");
            assert.equal(blockedSniff.result.value, false,
                "runtime nosniff must deny misdeclared same-origin script");
            assert.equal(sniffProtectedRequests, 1,
                "MIME refusal must occur after the permitted script fetch");

            // An injected inline script or <base> must not acquire
            // execution authority from the no-meta document.
            const hostileBase = JSON.stringify(
                `${hostileOrigin}/injected-base/`,
            );
            const probeDomPolicy = `(() => {
                const inline = document.createElement("script");
                inline.textContent =
                    "window.atramentInlineCspProbe = true;";
                document.body.append(inline);
                const base = document.createElement("base");
                base.href = ${hostileBase};
                document.head.append(base);
                return JSON.stringify({
                    baseChanged: document.baseURI === ${hostileBase},
                    inlineExecuted:
                        window.atramentInlineCspProbe === true,
                });
            })()`;
            const controlDomPolicy = await bidi.command(
                "script.evaluate", {
                    awaitPromise: false,
                    expression: probeDomPolicy,
                    resultOwnership: "none",
                    target: { context: networkControl.context },
                },
            );
            assert.equal(controlDomPolicy.type, "success");
            assert.deepEqual(JSON.parse(controlDomPolicy.result.value), {
                baseChanged: true, inlineExecuted: true,
            }, "permissive control permits DOM script and base injection");
            const blockedDomPolicy = await bidi.command(
                "script.evaluate", {
                    awaitPromise: false,
                    expression: probeDomPolicy,
                    resultOwnership: "none",
                    target: { context: headerOnly.context },
                },
            );
            assert.equal(blockedDomPolicy.type, "success");
            assert.deepEqual(JSON.parse(blockedDomPolicy.result.value), {
                baseChanged: false, inlineExecuted: false,
            }, "HTTP CSP alone must reject inline script and foreign base");

            // A same-origin Worker is permitted by default-src 'self' but
            // must be refused by the explicit worker-src 'none' directive.
            const workerUrl = JSON.stringify(
                `${permissiveOrigin}/csp-worker.js`,
            );
            const probeWorker = `(async () => {
                let worker;
                try {
                    worker = new Worker(${workerUrl});
                    return await new Promise((resolve) => {
                        const timer = setTimeout(() => resolve(false), 3000);
                        const finish = (value) => {
                            clearTimeout(timer);
                            resolve(value);
                        };
                        worker.addEventListener("message", (event) => {
                            finish(event.data === "ready");
                        }, { once: true });
                        worker.addEventListener("error", () => {
                            finish(false);
                        }, { once: true });
                    });
                } catch {
                    return false;
                } finally {
                    worker?.terminate();
                }
            })()`;
            const controlWorker = await bidi.command("script.evaluate", {
                awaitPromise: true,
                expression: probeWorker,
                resultOwnership: "none",
                target: { context: networkControl.context },
            });
            assert.equal(controlWorker.type, "success");
            assert.equal(controlWorker.result.value, true);
            assert.equal(workerRequests, 1,
                "permissive control must request and start the worker");
            const blockedWorker = await bidi.command("script.evaluate", {
                awaitPromise: true,
                expression: probeWorker,
                resultOwnership: "none",
                target: { context: headerOnly.context },
            });
            assert.equal(blockedWorker.type, "success");
            assert.equal(blockedWorker.result.value, false,
                "HTTP worker-src must refuse even same-origin workers");
            assert.equal(workerRequests, 1,
                "CSP rejection must send no worker script request");

            // A malformed same-origin font must still be fetched in the
            // permissive control; font-src 'none' prevents even that GET.
            const fontUrl = JSON.stringify(
                `${permissiveOrigin}/csp-font.woff2`,
            );
            const probeFont = `(async () => {
                const font = new FontFace(
                    "AtramentCspProbe", "url(" + ${fontUrl} + ")"
                );
                try {
                    await font.load();
                } catch {
                    // Both pages reject malformed WOFF2 decoding; only the
                    // protected page must refuse the resource request.
                }
                return font.status;
            })()`;
            const controlFont = await bidi.command("script.evaluate", {
                awaitPromise: true,
                expression: probeFont,
                resultOwnership: "none",
                target: { context: networkControl.context },
            });
            assert.equal(controlFont.type, "success");
            assert.equal(controlFont.result.value, "error");
            assert.ok(fontRequests > 0,
                "permissive control must request the font before decode");
            const requestsBeforeBlockedFont = fontRequests;
            const blockedFont = await bidi.command("script.evaluate", {
                awaitPromise: true,
                expression: probeFont,
                resultOwnership: "none",
                target: { context: headerOnly.context },
            });
            assert.equal(blockedFont.type, "success");
            assert.equal(blockedFont.result.value, "error");
            assert.equal(fontRequests, requestsBeforeBlockedFont,
                "HTTP font-src must refuse same-origin font requests");

            // media-src 'none' denies even same-origin media. A valid WAV
            // must load in the control without relying on audio playback.
            const mediaUrl = JSON.stringify(
                `${permissiveOrigin}/csp-media.wav`,
            );
            const probeMedia = `(async () => {
                const audio = document.createElement("audio");
                audio.preload = "metadata";
                document.body.append(audio);
                const loaded = await new Promise((resolve) => {
                    const timer = setTimeout(() => resolve(false), 4000);
                    const finish = (value) => {
                        clearTimeout(timer);
                        resolve(value);
                    };
                    audio.addEventListener("loadedmetadata", () => {
                        finish(true);
                    }, { once: true });
                    audio.addEventListener("error", () => {
                        finish(false);
                    }, { once: true });
                    audio.src = ${mediaUrl};
                    audio.load();
                });
                audio.removeAttribute("src");
                audio.load();
                audio.remove();
                return loaded;
            })()`;
            const controlMedia = await bidi.command("script.evaluate", {
                awaitPromise: true,
                expression: probeMedia,
                resultOwnership: "none",
                target: { context: networkControl.context },
            });
            assert.equal(controlMedia.type, "success");
            assert.equal(controlMedia.result.value, true,
                "permissive control must decode same-origin WAV metadata");
            assert.ok(mediaRequests > 0,
                "permissive control must fetch the WAV resource");
            const requestsBeforeBlockedMedia = mediaRequests;
            const blockedMedia = await bidi.command("script.evaluate", {
                awaitPromise: true,
                expression: probeMedia,
                resultOwnership: "none",
                target: { context: headerOnly.context },
            });
            assert.equal(blockedMedia.type, "success");
            assert.equal(blockedMedia.result.value, false,
                "HTTP media-src must deny same-origin WAV metadata");
            assert.equal(mediaRequests, requestsBeforeBlockedMedia,
                "HTTP media-src must refuse without a media request");

            // object-src 'none' must block a same-origin embedded SVG even
            // though default-src 'self' would otherwise admit that source.
            const objectUrl = JSON.stringify(
                `${permissiveOrigin}/csp-object.svg`,
            );
            const probeObject = `(async () => {
                const node = document.createElement("object");
                node.type = "image/svg+xml";
                node.data = ${objectUrl};
                const loaded = await new Promise((resolve) => {
                    const timer = setTimeout(() => resolve(false), 3000);
                    const finish = (value) => {
                        clearTimeout(timer);
                        resolve(value);
                    };
                    node.onload = () => finish(
                        node.contentDocument?.documentElement?.localName
                            === "svg"
                    );
                    node.onerror = () => finish(false);
                    document.body.append(node);
                });
                node.remove();
                return loaded;
            })()`;
            const controlObject = await bidi.command("script.evaluate", {
                awaitPromise: true,
                expression: probeObject,
                resultOwnership: "none",
                target: { context: networkControl.context },
            });
            assert.equal(controlObject.type, "success");
            assert.equal(controlObject.result.value, true,
                "permissive control must load same-origin SVG object");
            assert.equal(objectRequests, 1);
            const blockedObject = await bidi.command("script.evaluate", {
                awaitPromise: true,
                expression: probeObject,
                resultOwnership: "none",
                target: { context: headerOnly.context },
            });
            assert.equal(blockedObject.type, "success");
            assert.equal(blockedObject.result.value, false,
                "HTTP object-src must deny same-origin embedded SVG");
            assert.equal(objectRequests, 1,
                "HTTP object-src must refuse without an SVG request");

            // frame-src 'none' must deny a child frame from the page's own
            // origin, which default-src 'self' would otherwise allow.
            const frameChildUrl = JSON.stringify(
                `${permissiveOrigin}/csp-frame-child`,
            );
            const probeChildFrame = `(async () => {
                const frame = document.createElement("iframe");
                frame.src = ${frameChildUrl};
                const loaded = await new Promise((resolve) => {
                    const timer = setTimeout(() => resolve(false), 3000);
                    frame.onload = () => {
                        clearTimeout(timer);
                        resolve(frame.contentDocument?.body?.textContent
                            ?.includes("Frame child loaded") === true);
                    };
                    frame.onerror = () => {
                        clearTimeout(timer);
                        resolve(false);
                    };
                    document.body.append(frame);
                });
                frame.remove();
                return loaded;
            })()`;
            const controlFrame = await bidi.command("script.evaluate", {
                awaitPromise: true,
                expression: probeChildFrame,
                resultOwnership: "none",
                target: { context: networkControl.context },
            });
            assert.equal(controlFrame.type, "success");
            assert.equal(controlFrame.result.value, true);
            assert.equal(frameChildRequests, 1,
                "permissive control loads the same-origin child frame");
            const blockedFrame = await bidi.command("script.evaluate", {
                awaitPromise: true,
                expression: probeChildFrame,
                resultOwnership: "none",
                target: { context: headerOnly.context },
            });
            assert.equal(blockedFrame.type, "success");
            assert.equal(blockedFrame.result.value, false);
            assert.equal(frameChildRequests, 1,
                "HTTP frame-src must refuse same-origin child requests");

            // form-action 'none' must block a same-origin POST even though
            // script.evaluate itself is privileged automation code.
            const formUrl = JSON.stringify(
                `${permissiveOrigin}/csp-form-submit`,
            );
            const submitForm = `(() => {
                const form = document.createElement("form");
                form.method = "POST";
                form.action = ${formUrl};
                document.body.append(form);
                form.submit();
                return true;
            })()`;
            const controlForm = await bidi.command("script.evaluate", {
                awaitPromise: false,
                expression: submitForm,
                resultOwnership: "none",
                target: { context: networkControl.context },
            });
            assert.equal(controlForm.type, "success");
            assert.equal(controlForm.result.value, true);
            assert.equal(await observedWithin(
                controlFormSubmitted, "permissive form submission",
            ), "POST");
            assert.equal(formSubmissions, 1);
            const blockedForm = await bidi.command("script.evaluate", {
                awaitPromise: true,
                expression: `(async () => {
                    try {
                        ${submitForm};
                    } catch {
                        // A synchronous CSP refusal is also a denial.
                    }
                    await new Promise(resolve => setTimeout(resolve, 300));
                    return location.pathname;
                })()`,
                resultOwnership: "none",
                target: { context: headerOnly.context },
            });
            assert.equal(blockedForm.type, "success");
            assert.equal(blockedForm.result.value, "/csp-header-only",
                "HTTP form-action must keep the protected document in place");
            assert.equal(formSubmissions, 1,
                "HTTP form-action must prevent same-origin POST emission");

            const direct = await bidi.command("browsingContext.create", {
                type: "tab",
            });
            await bidi.command("browsingContext.navigate", {
                context: direct.context,
                url: origin,
                wait: "complete",
            });
            assert.deepEqual(
                await waitForWorkspaceDocument(bidi, direct.context),
                {
                    hasControl: false,
                    hasWorkspace: true,
                    title: "Atrament",
                },
            );

            // CORS permits the other loopback origin deliberately; a refusal
            // here is from the workspace's header/meta CSP, not from CORS.
            const networkProbe = await bidi.command("script.evaluate", {
                awaitPromise: true,
                expression: `(async () => {
                    const local = await fetch("/health", { cache: "no-store" });
                    let externalBlocked = false;
                    try {
                        await fetch(${externalTarget}, {
                            cache: "no-store"
                        });
                    } catch {
                        externalBlocked = true;
                    }
                    return JSON.stringify({
                        localHealth: local.ok, externalBlocked
                    });
                })()`,
                resultOwnership: "none",
                target: { context: direct.context },
            });
            assert.equal(networkProbe.type, "success");
            assert.deepEqual(JSON.parse(networkProbe.result.value), {
                localHealth: true,
                externalBlocked: true,
            });
            assert.equal(outboundRequests, 1,
                "workspace must add no cross-origin request bytes");

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

            // Rehost the actual runtime X-Frame-Options header without
            // CSP, so this framing refusal is independent from
            // frame-ancestors and the existing document meta policy.
            const framedXfoDocument = async (suffix) => {
                const tab = await bidi.command("browsingContext.create", {
                    type: "tab",
                });
                await bidi.command("browsingContext.navigate", {
                    context: tab.context,
                    url: `${hostileOrigin}/${suffix}`,
                    wait: "complete",
                });
                const childContext = await waitForChildContext(
                    bidi, tab.context,
                );
                return waitForFramedDocument(bidi, childContext);
            };
            assert.deepEqual(await framedXfoDocument("xfo-control"), {
                hasControl: true,
                hasWorkspace: false,
                title: "Permissive frame",
            }, "cross-origin framing control must load without XFO");
            const xfoBlocked = await framedXfoDocument("xfo-protected");
            assert.equal(xfoBlocked.hasControl, false,
                "XFO must block cross-origin embedding without CSP");
            assert.notEqual(xfoBlocked.title, "XFO protected frame");

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
            bidi?.close();
            for (const server of [hostile, permissive]) {
                await closeFixtureServer(server);
            }
            for (const child of children.reverse()) {
                await stopChild(child);
            }
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox retries transient handshake and draft-load failures in place",
    { skip: !FIREFOX_AVAILABLE, timeout: FIREFOX_CASE_TIMEOUT_MS },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/session-retry-");
        const apiRequests = [];
        const mutations = [];
        let handshakeCalls = 0;
        let sourceReads = 0;
        const server = createSessionFixtureServer(
            apiRequests,
            (request, response, field) => {
                const chunks = [];
                request.on("data", (chunk) => chunks.push(chunk));
                request.on("end", () => {
                    mutations.push({
                        body: Buffer.concat(chunks).toString("utf8"), field,
                    });
                    const status = mutations.length === 1 ? 503
                        : mutations.length === 3 ? 413
                            : mutations.length === 4 ? 401 : 204;
                    response.writeHead(status, {
                        "Cache-Control": "no-store",
                        "Content-Type": "application/json; charset=utf-8",
                    });
                    response.end(status === 413 ? JSON.stringify({
                        error: "resource_limit",
                        diagnostics: {
                            version: "atrament.diagnostic/1",
                            completeness: "complete",
                            items: [{
                                code: "atrament.session-draft.resource-limit",
                            }],
                        },
                    }) : "");
                });
            },
            null,
            () => (++handshakeCalls === 1 ? 503
                : handshakeCalls === 2 ? 408 : 200),
            (field) => field === "source" && ++sourceReads === 1
                ? 503
                : 200,
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
            const inspectRetry = async () => {
                const result = await bidi.command("script.evaluate", {
                    awaitPromise: false,
                    expression: `(() => {
                        const retry = document.getElementById("retry-session");
                        return JSON.stringify({
                            disabled: retry.disabled,
                            hidden: retry.hidden,
                            label: retry.textContent
                        });
                    })()`,
                    resultOwnership: "none",
                    target: { context: tab.context },
                });
                assert.equal(result.type, "success");
                return JSON.parse(result.result.value);
            };
            const clickRetry = async () => {
                await bidi.command("script.evaluate", {
                    awaitPromise: false,
                    expression: `(() => {
                        const retry = document.getElementById("retry-session");
                        retry.click();
                        retry.click();
                    })()`,
                    resultOwnership: "none",
                    target: { context: tab.context },
                });
            };

            const handshakeFailure = await waitForSessionStatus(
                bidi,
                tab.context,
                "Invalid backend handshake · editing disabled",
            );
            assert.equal(handshakeFailure.hash, "");
            assert.deepEqual(await inspectRetry(), {
                disabled: false, hidden: false, label: "Retry connection",
            });
            await clickRetry();

            const timeoutFailure = await waitForSessionStatus(
                bidi,
                tab.context,
                "Invalid backend handshake · editing disabled",
            );
            assert.equal(timeoutFailure.hash, "");
            assert.equal(handshakeCalls, 2);
            assert.deepEqual(await inspectRetry(), {
                disabled: false, hidden: false, label: "Retry connection",
            });
            await clickRetry();

            const draftFailure = await waitForSessionStatus(
                bidi,
                tab.context,
                "Draft load rejected · editing off",
            );
            for (const field of ["task", "source", "candidate"]) {
                assert.deepEqual(draftFailure[field], {
                    disabled: true, value: "",
                });
            }
            assert.deepEqual(await inspectRetry(), {
                disabled: false, hidden: false, label: "Retry connection",
            });
            await clickRetry();

            const ready = await waitForSessionStatus(
                bidi,
                tab.context,
                "Session ready",
            );
            assert.equal(ready.hash, "");
            for (const field of ["task", "source", "candidate"]) {
                assert.deepEqual(ready[field], {
                    disabled: false, value: REFRESH_DRAFT[field],
                });
            }
            assert.deepEqual(await inspectRetry(), {
                disabled: true, hidden: true, label: "Retry connection",
            });
            assert.deepEqual(apiRequests, [
                "POST /api/handshake",
                "POST /api/handshake",
                "POST /api/handshake",
                "GET /api/session/task",
                "GET /api/session/source",
                "POST /api/handshake",
                "GET /api/session/task",
                "GET /api/session/source",
                "GET /api/session/candidate",
            ]);

            const editTask = async (value) => {
                await bidi.command("script.evaluate", {
                    awaitPromise: false,
                    expression: `(() => {
                        const task = document.getElementById("task-input");
                        task.value = ${JSON.stringify(value)};
                        task.dispatchEvent(new Event("input"));
                    })()`,
                    resultOwnership: "none",
                    target: { context: tab.context },
                });
            };
            await editTask("preserve this unsaved text exactly");
            const unsaved = await waitForSessionStatus(
                bidi,
                tab.context,
                "Draft sync rejected · retry edit",
            );
            assert.equal(
                unsaved.task.value,
                "preserve this unsaved text exactly",
            );
            assert.deepEqual(await inspectRetry(), {
                disabled: false, hidden: false, label: "Retry edits",
            });
            await clickRetry();
            const saved = await waitForSessionStatus(
                bidi,
                tab.context,
                "Session ready",
            );
            assert.equal(
                saved.task.value,
                "preserve this unsaved text exactly",
            );
            assert.equal(saved.source.value, REFRESH_DRAFT.source);
            assert.deepEqual(await inspectRetry(), {
                disabled: true, hidden: true, label: "Retry edits",
            });
            await editTask("too large to retry unchanged");
            const tooLarge = await waitForSessionStatus(
                bidi,
                tab.context,
                "Draft too large · reduce",
            );
            assert.equal(tooLarge.task.value, "too large to retry unchanged");
            assert.deepEqual(await inspectRetry(), {
                disabled: true, hidden: true, label: "Retry edits",
            });
            assert.deepEqual(mutations, [
                { body: "preserve this unsaved text exactly", field: "task" },
                { body: "preserve this unsaved text exactly", field: "task" },
                { body: "too large to retry unchanged", field: "task" },
            ]);
            await editTask("authorization revoked");
            const revoked = await waitForSessionStatus(
                bidi,
                tab.context,
                "Authorization failed",
            );
            for (const field of ["task", "source", "candidate"]) {
                assert.deepEqual(revoked[field], {
                    disabled: true,
                    value: "",
                });
            }
            assert.deepEqual(await inspectRetry(), {
                disabled: true, hidden: true, label: "Retry edits",
            });
            await clickRetry();
            assert.deepEqual(apiRequests.slice(9), [
                "POST /api/session/task",
                "POST /api/session/task",
                "POST /api/session/task",
                "POST /api/session/task",
            ]);
        } finally {
            bidi?.close();
            await closeFixtureServer(server);
            for (const child of children.reverse()) {
                await stopChild(child);
            }
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox rejects wrong-media 413 diagnostics while preserving draft text",
    { skip: !FIREFOX_AVAILABLE, timeout: FIREFOX_CASE_TIMEOUT_MS },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/session-diagnostic-media-");
        const apiRequests = [];
        const edits = [];
        const limitDiagnostic = JSON.stringify({
            error: "resource_limit",
            diagnostics: {
                version: "atrament.diagnostic/1",
                completeness: "complete",
                items: [{ code: "atrament.session-draft.resource-limit" }],
            },
        });
        const server = createSessionFixtureServer(
            apiRequests,
            (request, response, field) => {
                const chunks = [];
                request.on("data", (chunk) => chunks.push(chunk));
                request.on("end", () => {
                    edits.push({
                        field,
                        text: Buffer.concat(chunks).toString("utf8"),
                    });
                    response.writeHead(413, {
                        "Cache-Control": "no-store",
                        "Content-Type": edits.length === 1
                            ? "text/html; charset=utf-8"
                            : "application/json; charset=utf-8",
                    });
                    response.end(edits.length === 2
                        ? limitDiagnostic + " ".repeat(16_385)
                        : limitDiagnostic);
                });
            },
        );
        const children = [];
        let bidi;
        try {
            server.listen(0, "127.0.0.1");
            await once(server, "listening");
            const address = server.address();
            assert.notEqual(address, null);
            assert.notEqual(typeof address, "string");
            bidi = await startBidiFirefox(profile, children);
            const tab = await bidi.command("browsingContext.create", {
                type: "tab",
            });
            await bidi.command("browsingContext.navigate", {
                context: tab.context,
                url: `http://127.0.0.1:${address.port}/` +
                    `#session=${REFRESH_SECRET}`,
                wait: "complete",
            });
            await waitForSessionStatus(bidi, tab.context, "Session ready");
            for (const [value, expected] of [
                ["wrong-media draft", "Invalid draft diagnostic"],
                ["oversized-json draft", "Invalid draft diagnostic"],
                ["valid-media draft", "Draft too large · reduce"],
            ]) {
                await bidi.command("script.evaluate", {
                    awaitPromise: false,
                    expression: `(() => {
                        const task = document.getElementById("task-input");
                        task.value = ${JSON.stringify(value)};
                        task.dispatchEvent(new Event("input"));
                    })()`,
                    resultOwnership: "none",
                    target: { context: tab.context },
                });
                const state = await waitForSessionStatus(
                    bidi, tab.context, expected,
                );
                assert.deepEqual(state.task, {
                    disabled: false, value,
                });
            }
            assert.deepEqual(edits, [
                { field: "task", text: "wrong-media draft" },
                { field: "task", text: "oversized-json draft" },
                { field: "task", text: "valid-media draft" },
            ]);
            assert.deepEqual(apiRequests, [
                "POST /api/handshake",
                "GET /api/session/task",
                "GET /api/session/source",
                "GET /api/session/candidate",
                "POST /api/session/task",
                "POST /api/session/task",
                "POST /api/session/task",
            ]);
        } finally {
            bidi?.close();
            await closeFixtureServer(server);
            for (const child of children.reverse()) {
                await stopChild(child);
            }
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox rejects wrong-media handshake, then retries in the same tab",
    { skip: !FIREFOX_AVAILABLE, timeout: FIREFOX_CASE_TIMEOUT_MS },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/session-handshake-media-");
        const apiRequests = [];
        let handshakes = 0;
        const server = createSessionFixtureServer(
            apiRequests, null, null, null, null, null,
            () => ++handshakes === 1
                ? "text/html; charset=utf-8"
                : "application/json; charset=utf-8",
        );
        const children = [];
        let bidi;
        try {
            server.listen(0, "127.0.0.1");
            await once(server, "listening");
            const address = server.address();
            assert.notEqual(address, null);
            assert.notEqual(typeof address, "string");
            const origin = `http://127.0.0.1:${address.port}`;

            bidi = await startBidiFirefox(profile, children);
            const firstTab = await bidi.command("browsingContext.create", {
                type: "tab",
            });
            await bidi.command("browsingContext.navigate", {
                context: firstTab.context,
                url: `${origin}/#session=${REFRESH_SECRET}`,
                wait: "complete",
            });
            const rejected = await waitForSessionStatus(
                bidi, firstTab.context,
                "Invalid backend handshake · editing disabled",
            );
            assert.equal(rejected.hash, "");
            for (const field of ["task", "source", "candidate"]) {
                assert.deepEqual(rejected[field], {
                    disabled: true, value: "",
                });
            }
            assert.deepEqual(apiRequests, ["POST /api/handshake"]);

            // Recover without reintroducing the scrubbed credential into
            // any address bar or creating a new browsing context.
            const retry = await bidi.command("script.evaluate", {
                awaitPromise: false,
                expression: `(() => {
                    const control = document.getElementById("retry-session");
                    if (control.disabled || control.hidden
                        || control.textContent !== "Retry connection") {
                        throw new Error("invalid handshake has no retry");
                    }
                    control.click();
                    return true;
                })()`,
                resultOwnership: "none",
                target: { context: firstTab.context },
            });
            assert.equal(retry.type, "success");
            const accepted = await waitForSessionStatus(
                bidi, firstTab.context, "Session ready",
            );
            for (const field of ["task", "source", "candidate"]) {
                assert.deepEqual(accepted[field], {
                    disabled: false, value: REFRESH_DRAFT[field],
                });
            }
            assert.deepEqual(apiRequests, [
                "POST /api/handshake",
                "POST /api/handshake",
                "GET /api/session/task",
                "GET /api/session/source",
                "GET /api/session/candidate",
            ]);
        } finally {
            bidi?.close();
            await closeFixtureServer(server);
            for (const child of children.reverse()) {
                await stopChild(child);
            }
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox retries malformed HTTP 409 but terminates a typed mismatch",
    { skip: !FIREFOX_AVAILABLE, timeout: FIREFOX_CASE_TIMEOUT_MS },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/session-handshake-conflict-");
        const apiRequests = [];
        let handshakes = 0;
        const mismatch = JSON.stringify({
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
        });
        const server = createSessionFixtureServer(
            apiRequests, null, null, null, null, null, null, null, null,
            (_request, response) => {
                handshakes += 1;
                response.writeHead(409, {
                    "Cache-Control": "no-store",
                    "Content-Type": "application/json; charset=utf-8",
                });
                response.end(handshakes === 1
                    ? JSON.stringify({ result: "incompatible" })
                    : mismatch);
                return true;
            },
        );
        const children = [];
        let bidi;
        try {
            server.listen(0, "127.0.0.1");
            await once(server, "listening");
            const address = server.address();
            assert.notEqual(address, null);
            assert.notEqual(typeof address, "string");
            bidi = await startBidiFirefox(profile, children);
            const tab = await bidi.command("browsingContext.create", {
                type: "tab",
            });
            await bidi.command("browsingContext.navigate", {
                context: tab.context,
                url: `http://127.0.0.1:${address.port}/` +
                    `#session=${REFRESH_SECRET}`,
                wait: "complete",
            });
            const invalid = await waitForSessionStatus(
                bidi, tab.context,
                "Invalid backend handshake · editing disabled",
            );
            assert.equal(invalid.hash, "");
            for (const field of ["task", "source", "candidate"]) {
                assert.deepEqual(invalid[field], {
                    disabled: true, value: "",
                });
            }
            assert.deepEqual(apiRequests, ["POST /api/handshake"]);

            const retried = await bidi.command("script.evaluate", {
                awaitPromise: false,
                expression: `(() => {
                    const retry = document.getElementById("retry-session");
                    if (retry.hidden || retry.disabled
                        || retry.textContent !== "Retry connection") {
                        throw Error("malformed conflict must allow retry");
                    }
                    retry.click();
                    return true;
                })()`,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.equal(retried.type, "success");
            const incompatible = await waitForSessionStatus(
                bidi, tab.context,
                "Incompatible prompt · expected atrament.prompt/1",
            );
            assert.equal(incompatible.hash, "");
            for (const field of ["task", "source", "candidate"]) {
                assert.deepEqual(incompatible[field], {
                    disabled: true, value: "",
                });
            }
            const terminal = await bidi.command("script.evaluate", {
                awaitPromise: false,
                expression: `(() => {
                    const retry = document.getElementById("retry-session");
                    if (!retry.hidden || !retry.disabled) {
                        throw Error("typed mismatch exposed a retry");
                    }
                    retry.dispatchEvent(new MouseEvent("click"));
                    return true;
                })()`,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.equal(terminal.type, "success");
            await new Promise((resolve) => setTimeout(resolve, 100));
            assert.deepEqual(apiRequests, [
                "POST /api/handshake",
                "POST /api/handshake",
            ]);
        } finally {
            bidi?.close();
            await closeFixtureServer(server);
            for (const child of children.reverse()) {
                await stopChild(child);
            }
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox aborts a stalled wrong-media handshake before in-tab retry",
    { skip: !FIREFOX_AVAILABLE, timeout: FIREFOX_CASE_TIMEOUT_MS },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/session-handshake-cancel-");
        const apiRequests = [];
        let handshakes = 0;
        let streamClosed;
        const firstStreamClosed = new Promise((resolve) => {
            streamClosed = resolve;
        });
        const server = createSessionFixtureServer(
            apiRequests, null, null, null, null, null, null, null, null,
            (request, response) => {
                handshakes += 1;
                if (handshakes !== 1) {
                    return false;
                }
                assert.equal(request.method, "POST");
                response.writeHead(200, {
                    "Cache-Control": "no-store",
                    "Content-Type": "text/html; charset=utf-8",
                });
                // Keep the response open forever unless the browser actively
                // discards the body. A completed short response would not
                // prove fetch stream cancellation.
                response.write("<html>untrusted response never ends");
                response.once("close", streamClosed);
                return true;
            },
        );
        const children = [];
        let bidi;
        try {
            server.listen(0, "127.0.0.1");
            await once(server, "listening");
            const address = server.address();
            assert.notEqual(address, null);
            assert.notEqual(typeof address, "string");
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
            const rejected = await waitForSessionStatus(
                bidi, tab.context,
                "Invalid backend handshake · editing disabled",
            );
            assert.equal(rejected.hash, "");
            for (const field of ["task", "source", "candidate"]) {
                assert.deepEqual(rejected[field], {
                    disabled: true, value: "",
                });
            }
            assert.deepEqual(apiRequests, ["POST /api/handshake"]);
            await observedWithin(firstStreamClosed,
                "unread wrong-media handshake abort");
            const retry = await bidi.command("script.evaluate", {
                awaitPromise: false,
                expression: `(() => {
                    const retry = document.getElementById("retry-session");
                    if (retry.disabled || retry.hidden
                        || retry.textContent !== "Retry connection") {
                        throw Error("stalled handshake retry unavailable");
                    }
                    retry.click();
                    return true;
                })()`,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.equal(retry.type, "success");
            const accepted = await waitForSessionStatus(
                bidi, tab.context, "Session ready",
            );
            assert.equal(accepted.hash, "");
            for (const field of ["task", "source", "candidate"]) {
                assert.deepEqual(accepted[field], {
                    disabled: false, value: REFRESH_DRAFT[field],
                });
            }
            assert.deepEqual(apiRequests, [
                "POST /api/handshake",
                "POST /api/handshake",
                "GET /api/session/task",
                "GET /api/session/source",
                "GET /api/session/candidate",
            ]);
        } finally {
            bidi?.close();
            await closeFixtureServer(server);
            for (const child of children.reverse()) {
                await stopChild(child);
            }
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox times out stalled valid-media session streams and retries",
    { skip: !FIREFOX_AVAILABLE, timeout: FIREFOX_CASE_TIMEOUT_MS },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/session-stream-timeout-");
        const apiRequests = [];
        let handshakes = 0;
        let taskReads = 0;
        let taskWrites = 0;
        let handshakeClosed;
        let draftClosed;
        let diagnosticClosed;
        const stalledDiagnosticClosed = new Promise((resolve) => {
            diagnosticClosed = resolve;
        });
        const stalledHandshakeClosed = new Promise((resolve) => {
            handshakeClosed = resolve;
        });
        const stalledDraftClosed = new Promise((resolve) => {
            draftClosed = resolve;
        });
        const server = createSessionFixtureServer(
            apiRequests,
            (_request, response, field) => {
                assert.equal(field, "task");
                if (++taskWrites !== 1) {
                    response.writeHead(204);
                    response.end();
                    return;
                }
                response.writeHead(413, {
                    "Cache-Control": "no-store",
                    "Content-Type": "application/json; charset=utf-8",
                });
                response.write('{"error":"resource_limit"');
                response.once("close", diagnosticClosed);
            },
            null, null, null, null, null, null, null,
            (_request, response) => {
                if (++handshakes !== 1) {
                    return false;
                }
                response.writeHead(200, {
                    "Cache-Control": "no-store",
                    "Content-Type": "application/json; charset=utf-8",
                });
                response.write('{"result":"compatible"');
                response.once("close", handshakeClosed);
                return true;
            },
            (_request, response, field) => {
                if (field !== "task" || ++taskReads !== 1) {
                    return false;
                }
                response.writeHead(200, {
                    "Cache-Control": "no-store",
                    "Content-Type": "text/plain; charset=utf-8",
                });
                response.write("partial private draft");
                response.once("close", draftClosed);
                return true;
            },
        );
        const children = [];
        let bidi;
        try {
            server.listen(0, "127.0.0.1");
            await once(server, "listening");
            const address = server.address();
            assert.notEqual(address, null);
            assert.notEqual(typeof address, "string");
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
            const retrySessionInSameTab = async () => {
                const retry = await bidi.command("script.evaluate", {
                    awaitPromise: false,
                    expression: `(() => {
                        const retry = document.getElementById("retry-session");
                        if (retry.hidden || retry.disabled
                            || retry.textContent !== "Retry connection") {
                            throw Error("stalled response retry unavailable");
                        }
                        retry.click();
                        return true;
                    })()`,
                    resultOwnership: "none",
                    target: { context: tab.context },
                });
                assert.equal(retry.type, "success");
            };
            const assertNotHydrated = (state) => {
                assert.equal(state.hash, "");
                for (const field of ["task", "source", "candidate"]) {
                    assert.deepEqual(state[field], {
                        disabled: true, value: "",
                    });
                }
            };
            const invalidHandshake = await waitForSessionStatus(
                bidi, tab.context,
                "Invalid backend handshake · editing disabled", 400,
            );
            assertNotHydrated(invalidHandshake);
            await observedWithin(stalledHandshakeClosed,
                "valid-media stalled handshake abort");
            await retrySessionInSameTab();
            const unavailableDraft = await waitForSessionStatus(
                bidi, tab.context, "Draft load unavailable · editing off", 400,
            );
            assertNotHydrated(unavailableDraft);
            await observedWithin(stalledDraftClosed,
                "valid-media stalled draft abort");
            await retrySessionInSameTab();
            const ready = await waitForSessionStatus(
                bidi, tab.context, "Session ready",
            );
            assert.equal(ready.hash, "");
            for (const field of ["task", "source", "candidate"]) {
                assert.deepEqual(ready[field], {
                    disabled: false, value: REFRESH_DRAFT[field],
                });
            }
            const edit = await bidi.command("script.evaluate", {
                awaitPromise: false,
                expression: `(() => {
                    const task = document.getElementById("task-input");
                    task.value = "pending private edit";
                    task.dispatchEvent(new Event("input"));
                })()`,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.equal(edit.type, "success");
            const failedEdit = await waitForSessionStatus(
                bidi, tab.context, "Draft offline · retry edit", 400,
            );
            assert.deepEqual(failedEdit.task, {
                disabled: false, value: "pending private edit",
            });
            await observedWithin(stalledDiagnosticClosed,
                "valid-media stalled diagnostic abort");
            const retryEdit = await bidi.command("script.evaluate", {
                awaitPromise: false,
                expression: `(() => {
                    const retry = document.getElementById("retry-session");
                    if (retry.hidden || retry.disabled
                        || retry.textContent !== "Retry edits") {
                        throw Error("stalled diagnostic retry unavailable");
                    }
                    retry.click();
                })()`,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.equal(retryEdit.type, "success");
            const synced = await waitForSessionStatus(
                bidi, tab.context, "Session ready",
            );
            assert.deepEqual(synced.task, {
                disabled: false, value: "pending private edit",
            });
            assert.deepEqual(apiRequests, [
                "POST /api/handshake",
                "POST /api/handshake",
                "GET /api/session/task",
                "POST /api/handshake",
                "GET /api/session/task",
                "GET /api/session/source",
                "GET /api/session/candidate",
                "POST /api/session/task",
                "POST /api/session/task",
            ]);
        } finally {
            bidi?.close();
            await closeFixtureServer(server);
            for (const child of children.reverse()) {
                await stopChild(child);
            }
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox refuses oversized handshake JSON until in-tab retry",
    { skip: !FIREFOX_AVAILABLE, timeout: FIREFOX_CASE_TIMEOUT_MS },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/session-handshake-bytes-");
        const apiRequests = [];
        let handshakes = 0;
        const server = createSessionFixtureServer(
            apiRequests, null, null, null, null, null, null, null,
            () => ++handshakes === 1
                ? COMPATIBLE_HANDSHAKE + " ".repeat(16_385)
                : COMPATIBLE_HANDSHAKE,
        );
        const children = [];
        let bidi;
        try {
            server.listen(0, "127.0.0.1");
            await once(server, "listening");
            const address = server.address();
            assert.notEqual(address, null);
            assert.notEqual(typeof address, "string");
            const origin = `http://127.0.0.1:${address.port}`;

            bidi = await startBidiFirefox(profile, children);
            const firstTab = await bidi.command("browsingContext.create", {
                type: "tab",
            });
            await bidi.command("browsingContext.navigate", {
                context: firstTab.context,
                url: `${origin}/#session=${REFRESH_SECRET}`,
                wait: "complete",
            });
            const rejected = await waitForSessionStatus(
                bidi, firstTab.context,
                "Invalid backend handshake · editing disabled",
            );
            assert.equal(rejected.hash, "");
            for (const field of ["task", "source", "candidate"]) {
                assert.deepEqual(rejected[field], {
                    disabled: true, value: "",
                });
            }
            assert.deepEqual(apiRequests, ["POST /api/handshake"]);

            // Recover without reintroducing the scrubbed credential into
            // any address bar or creating a new browsing context.
            const retry = await bidi.command("script.evaluate", {
                awaitPromise: false,
                expression: `(() => {
                    const control = document.getElementById("retry-session");
                    if (control.disabled || control.hidden
                        || control.textContent !== "Retry connection") {
                        throw new Error("invalid handshake has no retry");
                    }
                    control.click();
                    return true;
                })()`,
                resultOwnership: "none",
                target: { context: firstTab.context },
            });
            assert.equal(retry.type, "success");
            const accepted = await waitForSessionStatus(
                bidi, firstTab.context, "Session ready",
            );
            for (const field of ["task", "source", "candidate"]) {
                assert.deepEqual(accepted[field], {
                    disabled: false, value: REFRESH_DRAFT[field],
                });
            }
            assert.deepEqual(apiRequests, [
                "POST /api/handshake",
                "POST /api/handshake",
                "GET /api/session/task",
                "GET /api/session/source",
                "GET /api/session/candidate",
            ]);
        } finally {
            bidi?.close();
            await closeFixtureServer(server);
            for (const child of children.reverse()) {
                await stopChild(child);
            }
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox rejects wrong draft media then hydrates on valid retry",
    { skip: !FIREFOX_AVAILABLE, timeout: FIREFOX_CASE_TIMEOUT_MS },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/session-draft-media-");
        const apiRequests = [];
        let sourceReads = 0;
        const server = createSessionFixtureServer(
            apiRequests,
            null,
            null,
            null,
            null,
            (field) => field === "source" && ++sourceReads === 1
                ? "text/html; charset=utf-8"
                : "text/plain; charset=utf-8",
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
            const rejected = await waitForSessionStatus(
                bidi, tab.context, "Draft load rejected · editing off",
            );
            assert.equal(rejected.hash, "");
            for (const field of ["task", "source", "candidate"]) {
                assert.deepEqual(rejected[field], {
                    disabled: true, value: "",
                });
            }
            assert.deepEqual(apiRequests, [
                "POST /api/handshake",
                "GET /api/session/task",
                "GET /api/session/source",
            ]);
            const retry = await bidi.command("script.evaluate", {
                awaitPromise: false,
                expression: `(() => {
                    const control = document.getElementById("retry-session");
                    const visible = !control.hidden && !control.disabled;
                    if (visible) control.click();
                    return JSON.stringify({
                        visible,
                        label: control.textContent,
                    });
                })()`,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.equal(retry.type, "success");
            assert.deepEqual(JSON.parse(retry.result.value), {
                visible: true, label: "Retry connection",
            });

            const ready = await waitForSessionStatus(
                bidi, tab.context, "Session ready",
            );
            for (const field of ["task", "source", "candidate"]) {
                assert.deepEqual(ready[field], {
                    disabled: false, value: REFRESH_DRAFT[field],
                });
            }
            assert.equal(sourceReads, 2);
            assert.deepEqual(apiRequests, [
                "POST /api/handshake",
                "GET /api/session/task",
                "GET /api/session/source",
                "POST /api/handshake",
                "GET /api/session/task",
                "GET /api/session/source",
                "GET /api/session/candidate",
            ]);
        } finally {
            bidi?.close();
            await closeFixtureServer(server);
            for (const child of children.reverse()) {
                await stopChild(child);
            }
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox rejects corrupt and oversized draft bytes before a valid retry",
    { skip: !FIREFOX_AVAILABLE, timeout: FIREFOX_CASE_TIMEOUT_MS },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/session-draft-utf8-");
        const apiRequests = [];
        let sourceMode = "corrupt";
        const sourceText = "\uFEFFNotebook";
        const server = createSessionFixtureServer(
            apiRequests, null, null, null, null, null, null,
            (field) => field === "source"
                ? (sourceMode === "healthy"
                    ? sourceText
                    : sourceMode === "oversized"
                        ? Buffer.alloc(1_048_577, 0x61)
                        : Buffer.from([0xc3, 0x28]))
                : null,
        );
        const children = [];
        let bidi;
        try {
            server.listen(0, "127.0.0.1");
            await once(server, "listening");
            const address = server.address();
            assert.notEqual(address, null);
            assert.notEqual(typeof address, "string");
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
            const rejected = await waitForSessionStatus(
                bidi, tab.context, "Draft load rejected · editing off",
            );
            assert.equal(rejected.hash, "");
            for (const field of ["task", "source", "candidate"]) {
                assert.deepEqual(rejected[field], {
                    disabled: true, value: "",
                });
            }
            assert.deepEqual(apiRequests, [
                "POST /api/handshake",
                "GET /api/session/task",
                "GET /api/session/source",
            ]);
            sourceMode = "oversized";
            const oversizeRetry = await bidi.command("script.evaluate", {
                awaitPromise: false,
                expression: `(() => {
                    const button = document.getElementById("retry-session");
                    const active = !button.hidden && !button.disabled;
                    if (active) button.click();
                    return active;
                })()`,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.equal(oversizeRetry.type, "success");
            assert.deepEqual(oversizeRetry.result, {
                type: "boolean", value: true,
            });
            const tooLarge = await waitForSessionStatus(
                bidi, tab.context, "Draft load rejected · editing off",
            );
            for (const field of ["task", "source", "candidate"]) {
                assert.deepEqual(tooLarge[field], {
                    disabled: true, value: "",
                });
            }
            assert.deepEqual(apiRequests, [
                "POST /api/handshake",
                "GET /api/session/task",
                "GET /api/session/source",
                "POST /api/handshake",
                "GET /api/session/task",
                "GET /api/session/source",
            ]);
            sourceMode = "healthy";
            const retry = await bidi.command("script.evaluate", {
                awaitPromise: false,
                expression: `(() => {
                    const button = document.getElementById("retry-session");
                    const active = !button.hidden && !button.disabled;
                    if (active) button.click();
                    return active;
                })()`,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.equal(retry.type, "success");
            assert.deepEqual(retry.result, { type: "boolean", value: true });
            const ready = await waitForSessionStatus(
                bidi, tab.context, "Session ready",
            );
            assert.deepEqual(ready.task, {
                disabled: false, value: REFRESH_DRAFT.task,
            });
            assert.deepEqual(ready.source, {
                disabled: false, value: sourceText,
            });
            assert.deepEqual(ready.candidate, {
                disabled: false, value: REFRESH_DRAFT.candidate,
            });
            assert.deepEqual(apiRequests, [
                "POST /api/handshake",
                "GET /api/session/task",
                "GET /api/session/source",
                "POST /api/handshake",
                "GET /api/session/task",
                "GET /api/session/source",
                "POST /api/handshake",
                "GET /api/session/task",
                "GET /api/session/source",
                "GET /api/session/candidate",
            ]);
        } finally {
            bidi?.close();
            await closeFixtureServer(server);
            for (const child of children.reverse()) {
                await stopChild(child);
            }
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox refresh cannot reuse the scrubbed session credential",
    { skip: !FIREFOX_AVAILABLE, timeout: FIREFOX_CASE_TIMEOUT_MS },
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
            bidi?.close();
            await closeFixtureServer(server);
            for (const child of children.reverse()) {
                await stopChild(child);
            }
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox history return clears page and leaves no literal draft",
    { skip: !FIREFOX_AVAILABLE, timeout: FIREFOX_CASE_TIMEOUT_MS },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/session-back-");
        const apiRequests = [];
        const drafts = Object.fromEntries(
            ["task", "source", "candidate"].map((field) => [
                field, `atrament-${field}-history-private-` +
                    crypto.randomBytes(24).toString("hex"),
            ]),
        );
        const server = createSessionFixtureServer(
            apiRequests, null, null, null, null, null, null,
            (field) => drafts[field],
        );
        const children = [];
        let bidi;
        try {
            server.listen(0, "127.0.0.1");
            await once(server, "listening");
            const address = server.address();
            assert.notEqual(address, null);
            assert.notEqual(typeof address, "string");
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
                bidi, tab.context, "Session ready",
            );
            for (const [field, marker] of Object.entries(drafts)) {
                assert.equal(ready[field].value === marker, true,
                    `${field} was present before browser navigation`);
            }
            assert.equal(ready.hash, "");
            // Use real document navigation so pagehide and browser history
            // restoration run under Firefox rather than a synthetic event.
            await bidi.command("browsingContext.navigate", {
                context: tab.context,
                url: `${origin}/missing`,
                wait: "complete",
            });
            const away = await bidi.command("script.evaluate", {
                awaitPromise: false,
                expression: `JSON.stringify({
                    path: location.pathname,
                    hasTask: document.querySelector("#task-input") !== null
                })`,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.equal(away.type, "success");
            assert.equal(away.result.type, "string");
            assert.deepEqual(JSON.parse(away.result.value), {
                path: "/missing", hasTask: false,
            });
            await bidi.command("browsingContext.traverseHistory", {
                context: tab.context, delta: -1,
            });
            const returned = await waitForSessionStatus(
                bidi, tab.context, "Frontend ready · credential unavailable",
            );
            assert.equal(returned.hash, "");
            for (const field of ["task", "source", "candidate"]) {
                assert.deepEqual(returned[field], {
                    disabled: true, value: "",
                });
            }
            await new Promise((resolve) => setTimeout(resolve, 100));
            assert.deepEqual(apiRequests, [
                "POST /api/handshake",
                "GET /api/session/task",
                "GET /api/session/source",
                "GET /api/session/candidate",
            ]);
            await bidi.command("browsingContext.close", {
                context: tab.context,
            });
            bidi.close();
            bidi = null;
            for (const child of children.reverse()) await stopChild(child);
            children.length = 0;

            const control = path.join(profile, "history-control.bin");
            fs.writeFileSync(control, Object.values(drafts).join("\n"),
                { mode: 0o600 });
            for (const marker of Object.values(drafts)) {
                assert.equal(firefoxProfileContainsPlaintext(profile, marker),
                    true, "test-owned history marker must be detectable");
            }
            fs.rmSync(control);
            for (const [field, marker] of Object.entries(drafts)) {
                assert.equal(firefoxProfileContainsPlaintext(profile, marker),
                    false, `history navigation retained ${field} draft`);
            }
        } finally {
            bidi?.close();
            await closeFixtureServer(server);
            for (const child of children.reverse()) await stopChild(child);
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox scrubs credential fragments added after session startup",
    { skip: !FIREFOX_AVAILABLE, timeout: FIREFOX_CASE_TIMEOUT_MS },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/session-late-fragment-");
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
                bidi, tab.context, "Session ready",
            );
            assert.equal(ready.hash, "");
            const navigateHash = async (hash) => {
                const response = await bidi.command("script.evaluate", {
                    awaitPromise: true,
                    expression: `new Promise((resolve) => {
                        window.addEventListener("hashchange", () => {
                            window.setTimeout(
                                () => resolve(window.location.hash), 0,
                            );
                        }, { once: true });
                        window.location.hash = ${JSON.stringify(hash)};
                    })`,
                    resultOwnership: "none",
                    target: { context: tab.context },
                });
                assert.equal(response.type, "success");
                assert.equal(response.result.type, "string");
                assert.equal(response.result.value, "",
                    `credential-like navigation retained ${hash}`);
            };
            const admittedRequests = [...apiRequests];
            for (const hash of [
                `#session=${REFRESH_SECRET}`,
                `#session%3D${REFRESH_SECRET}`,
                `#%73ession=${REFRESH_SECRET}`,
                `#SESSION=${REFRESH_SECRET}`,
            ]) {
                await navigateHash(hash);
                const current = await evaluateSessionDocument(
                    bidi, tab.context,
                );
                assert.equal(current.status, "Session ready");
                assert.deepEqual(current.task, ready.task);
                assert.deepEqual(current.source, ready.source);
                assert.deepEqual(current.candidate, ready.candidate);
            }
            assert.deepEqual(apiRequests, admittedRequests,
                "hash navigation must not trigger new authentication");
            const ordinary = await bidi.command("script.evaluate", {
                awaitPromise: true,
                expression: `new Promise((resolve) => {
                    window.addEventListener("hashchange", () => {
                        window.setTimeout(
                            () => resolve(window.location.hash), 0,
                        );
                    }, { once: true });
                    window.location.hash = "#source-panel";
                })`,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.equal(ordinary.type, "success");
            assert.equal(ordinary.result.type, "string");
            assert.equal(ordinary.result.value, "#source-panel");

            // The first handoff is not reusable after a tab refresh. A
            // subsequent hashchange must neither preserve the URL secret nor
            // turn the refreshed credentialless document into a session.
            await bidi.command("browsingContext.reload", {
                context: tab.context,
                wait: "complete",
            });
            const unavailable = await waitForSessionStatus(
                bidi, tab.context,
                "Frontend ready · credential unavailable",
            );
            for (const field of ["task", "source", "candidate"]) {
                assert.deepEqual(unavailable[field], {
                    disabled: true, value: "",
                });
            }
            for (const hash of [
                `#session=${REFRESH_SECRET}`,
                `#session%3D${REFRESH_SECRET}`,
                `#%73ession=${REFRESH_SECRET}`,
            ]) {
                await navigateHash(hash);
                const stillUnavailable = await evaluateSessionDocument(
                    bidi, tab.context,
                );
                assert.equal(
                    stillUnavailable.status,
                    "Frontend ready · credential unavailable",
                );
                for (const field of ["task", "source", "candidate"]) {
                    assert.deepEqual(stillUnavailable[field], {
                        disabled: true, value: "",
                    });
                }
            }
            assert.deepEqual(apiRequests, admittedRequests,
                "credential injection after refresh must not reauthorize");
        } finally {
            bidi?.close();
            await closeFixtureServer(server);
            for (const child of children.reverse()) {
                await stopChild(child);
            }
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox scrubs encoded session aliases without authorization",
    { skip: !FIREFOX_AVAILABLE, timeout: FIREFOX_CASE_TIMEOUT_MS },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/session-encoded-alias-");
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
            const aliases = [
                `#session%3D${REFRESH_SECRET}`,
                `#%73ession=${REFRESH_SECRET}`,
                `#SESSION=${REFRESH_SECRET}`,
            ];
            let firstTab = null;
            for (const hash of aliases) {
                const tab = await bidi.command("browsingContext.create", {
                    type: "tab",
                });
                firstTab ??= tab.context;
                await bidi.command("browsingContext.navigate", {
                    context: tab.context,
                    url: `${origin}/${hash}`,
                    wait: "complete",
                });
                const disabled = await waitForSessionStatus(
                    bidi,
                    tab.context,
                    "Frontend ready · credential unavailable",
                );
                assert.equal(disabled.hash, "", `alias ${hash} remains in URL`);
                for (const field of ["task", "source", "candidate"]) {
                    assert.deepEqual(disabled[field], {
                        disabled: true, value: "",
                    });
                }
                assert.deepEqual(
                    apiRequests,
                    [],
                    "alias must not authenticate",
                );
            }
            assert.notEqual(firstTab, null);
            await bidi.command("browsingContext.reload", {
                context: firstTab,
                wait: "complete",
            });
            const refreshed = await waitForSessionStatus(
                bidi,
                firstTab,
                "Frontend ready · credential unavailable",
            );
            assert.equal(refreshed.hash, "");
            assert.deepEqual(apiRequests, [], "refresh must not authenticate");
        } finally {
            bidi?.close();
            await closeFixtureServer(server);
            for (const child of children.reverse()) {
                await stopChild(child);
            }
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox retries a newer draft after an obsolete resource limit",
    { skip: !FIREFOX_AVAILABLE, timeout: FIREFOX_CASE_TIMEOUT_MS },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/session-draft-coalesce-");
        const apiRequests = [];
        const mutationBodies = [];
        let resolveFirstMutation;
        const firstMutation = new Promise((resolve) => {
            resolveFirstMutation = resolve;
        });
        let resolveSecondMutation;
        const secondMutation = new Promise((resolve) => {
            resolveSecondMutation = resolve;
        });
        let firstResponse = null;
        const server = createSessionFixtureServer(
            apiRequests,
            (request, response, field) => {
                const chunks = [];
                request.on("data", (chunk) => chunks.push(chunk));
                request.on("end", () => {
                    const body = Buffer.concat(chunks).toString("utf8");
                    mutationBodies.push({ body, field });
                    if (mutationBodies.length === 1) {
                        firstResponse = response;
                        resolveFirstMutation(body);
                        return;
                    }
                    response.writeHead(204, { "Cache-Control": "no-store" });
                    response.end();
                    resolveSecondMutation(body);
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
                    input.value = "obsolete oversized value";
                    input.dispatchEvent(new Event("input", { bubbles: true }));
                })()`,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.equal(
                await observedWithin(firstMutation, "first draft mutation"),
                "obsolete oversized value",
            );

            await bidi.command("script.evaluate", {
                awaitPromise: false,
                expression: `(() => {
                    const input = document.querySelector("#task-input");
                    input.value = "current valid value";
                    input.dispatchEvent(new Event("input", { bubbles: true }));
                })()`,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.notEqual(firstResponse, null);
            firstResponse.writeHead(413, {
                "Cache-Control": "no-store",
                "Content-Type": "application/json; charset=utf-8",
            });
            firstResponse.end(JSON.stringify({
                diagnostics: {
                    completeness: "complete",
                    items: [{
                        code: "atrament.session-draft.resource-limit",
                    }],
                    version: "atrament.diagnostic/1",
                },
                error: "resource_limit",
            }));

            assert.equal(
                await observedWithin(secondMutation, "second draft mutation"),
                "current valid value",
            );
            const settled = await waitForSessionStatus(
                bidi,
                tab.context,
                "Session ready",
            );
            assert.equal(settled.task.value, "current valid value");
            assert.deepEqual(mutationBodies, [
                { body: "obsolete oversized value", field: "task" },
                { body: "current valid value", field: "task" },
            ]);
            assert.deepEqual(apiRequests, [
                "POST /api/handshake",
                "GET /api/session/task",
                "GET /api/session/source",
                "GET /api/session/candidate",
                "POST /api/session/task",
                "POST /api/session/task",
            ]);
        } finally {
            if (firstResponse !== null && !firstResponse.writableEnded) {
                firstResponse.destroy();
            }
            bidi?.close();
            await closeFixtureServer(server);
            for (const child of children.reverse()) {
                await stopChild(child);
            }
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox close aborts an in-flight private draft replacement",
    { skip: !FIREFOX_AVAILABLE, timeout: FIREFOX_CASE_TIMEOUT_MS },
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
            bidi?.close();
            await closeFixtureServer(server);
            for (const child of children.reverse()) {
                await stopChild(child);
            }
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox divider never publishes nonfinite shares on missing geometry",
    { skip: !FIREFOX_AVAILABLE, timeout: FIREFOX_CASE_TIMEOUT_MS },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/workspace-degenerate-share-");
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
            await setFirefoxViewport(bidi, tab.context, 800, 600);
            const state = await bidi.command("script.evaluate", {
                awaitPromise: false,
                expression: `JSON.stringify((() => {
                    const divider = document.querySelector(
                        "#workspace-divider"
                    );
                    const workspace = document.querySelector(
                        ".workspace-grid"
                    );
                    const width = divider.getBoundingClientRect().width;
                    workspace.getBoundingClientRect = () => ({
                        left: 0,
                        width
                    });
                    divider.setPointerCapture = () => {
                        throw new Error("pointer capture unavailable");
                    };
                    divider.dispatchEvent(new PointerEvent("pointerdown", {
                        bubbles: true,
                        button: 0,
                        isPrimary: true,
                        pointerId: 6,
                    }));
                    divider.dispatchEvent(new MouseEvent("click", {
                        bubbles: true,
                        clientX: 120,
                        detail: 1,
                    }));
                    const afterPointer = divider.getAttribute("aria-valuenow");
                    divider.setAttribute("aria-valuenow", "NaN");
                    divider.dispatchEvent(new KeyboardEvent("keydown", {
                        bubbles: true,
                        key: "ArrowRight",
                    }));
                    return {
                        afterPointer,
                        afterKeyboard: divider.getAttribute("aria-valuenow"),
                        editor: document.documentElement.style.getPropertyValue(
                            "--editor-track"
                        ),
                        preview: document.documentElement.style
                            .getPropertyValue("--preview-track")
                    };
                })())`,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.equal(state.type, "success");
            assert.equal(state.result.type, "string");
            assert.deepEqual(JSON.parse(state.result.value), {
                afterPointer: "46",
                afterKeyboard: "46",
                editor: "46fr",
                preview: "54fr",
            });
            assert.equal(
                requests.some((request) => request.startsWith("/api/")),
                false,
            );
        } finally {
            bidi?.close();
            await closeFixtureServer(server);
            for (const child of children.reverse()) {
                await stopChild(child);
            }
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox explicit end requires confirmation and clears session state",
    { skip: !FIREFOX_AVAILABLE, timeout: FIREFOX_CASE_TIMEOUT_MS },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/session-end-control-");
        const apiRequests = [];
        let signalEnd;
        const ended = new Promise((resolve) => { signalEnd = resolve; });
        const server = createSessionFixtureServer(
            apiRequests, null, () => signalEnd(),
        );
        const children = [];
        let bidi;
        try {
            server.listen(0, "127.0.0.1");
            await once(server, "listening");
            const address = server.address();
            assert.notEqual(address, null);
            assert.notEqual(typeof address, "string");
            bidi = await startBidiFirefox(profile, children);
            const tab = await bidi.command("browsingContext.create", {
                type: "tab",
            });
            await bidi.command("browsingContext.navigate", {
                context: tab.context,
                url: `http://127.0.0.1:${address.port}/` +
                    `#session=${REFRESH_SECRET}`,
                wait: "complete",
            });
            await waitForSessionStatus(bidi, tab.context, "Session ready");
            const first = await bidi.command("script.evaluate", {
                awaitPromise: false,
                expression: `JSON.stringify((() => {
                    const button = document.querySelector("#end-session");
                    button.click();
                    return [button.disabled, button.textContent,
                        document.querySelector("#session-status").textContent];
                })())`,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.equal(first.type, "success");
            assert.deepEqual(JSON.parse(first.result.value), [
                false, "Confirm end session", "Ending discards this notebook",
            ]);
            assert.ok(!apiRequests.includes("POST /api/session/end"));
            await bidi.command("script.evaluate", {
                awaitPromise: false,
                expression: 'document.querySelector("#end-session").focus()',
                resultOwnership: "none",
                target: { context: tab.context },
            });
            await performTrustedKey(bidi, tab.context, "\uE00C");
            const escapeCancelled = await bidi.command("script.evaluate", {
                awaitPromise: false,
                expression: `JSON.stringify([
                    document.querySelector("#end-session").textContent,
                    document.querySelector("#session-status").textContent
                ])`,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.deepEqual(JSON.parse(escapeCancelled.result.value), [
                "End session", "Session ready",
            ]);
            assert.ok(!apiRequests.includes("POST /api/session/end"));
            await bidi.command("script.evaluate", {
                awaitPromise: false,
                expression: `(() => {
                    const button = document.querySelector("#end-session");
                    button.click();
                    button.blur();
                })()`,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            const blurCancelled = await bidi.command("script.evaluate", {
                awaitPromise: false,
                expression: `JSON.stringify([
                    document.querySelector("#end-session").textContent,
                    document.querySelector("#session-status").textContent
                ])`,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.deepEqual(JSON.parse(blurCancelled.result.value), [
                "End session", "Session ready",
            ]);
            assert.ok(!apiRequests.includes("POST /api/session/end"));
            await bidi.command("script.evaluate", {
                awaitPromise: false,
                expression: `(() => {
                    const button = document.querySelector("#end-session");
                    button.click();
                    button.click();
                })()`,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            await observedWithin(ended, "explicit session close");
            const state = await waitForSessionStatus(
                bidi, tab.context, "Session ended",
            );
            for (const field of ["task", "source", "candidate"]) {
                assert.deepEqual(state[field], { disabled: true, value: "" });
            }
            assert.equal(apiRequests.at(-1), "POST /api/session/end");
        } finally {
            bidi?.close();
            await closeFixtureServer(server);
            for (const child of children.reverse()) await stopChild(child);
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

// Direct browser evidence complements the static source inventory: a draft
// replacement must never materialize in script-visible persistent stores.
// The disposable Firefox profile is intentionally empty at startup.
async function browserStorageInventory(client, context) {
    const observed = await client.command("script.evaluate", {
        awaitPromise: true,
        expression: `(async () => JSON.stringify({
            localStorage: localStorage.length,
            sessionStorage: sessionStorage.length,
            cookie: document.cookie,
            indexedDB: typeof indexedDB.databases === "function"
                ? (await indexedDB.databases()).length : null,
            cacheStorage: typeof caches?.keys === "function"
                ? (await caches.keys()).length : null,
        }))()`,
        resultOwnership: "none",
        target: { context },
    });
    assert.equal(observed.type, "success");
    assert.equal(observed.result.type, "string");
    return JSON.parse(observed.result.value);
}

// Inspect only regular files in a test-owned Firefox profile, without
// following links. A fixed total-byte budget and chunk overlap prevent both
// unbounded memory use and false negatives at read boundaries. This checks
// literal bytes, not compressed or encrypted browser storage.
function firefoxProfileContainsPlaintext(profile, marker) {
    const needle = Buffer.from(marker, "utf8");
    assert.ok(needle.length > 0 && needle.length < 1024);
    const chunk = Buffer.alloc(64 * 1024);
    let scannedBytes = 0;
    let scannedFiles = 0;
    const inspect = (directory) => {
        const entries = fs.readdirSync(directory, { withFileTypes: true });
        for (const entry of entries) {
            const fullPath = path.join(directory, entry.name);
            if (entry.isDirectory()) {
                if (inspect(fullPath)) return true;
                continue;
            }
            if (!entry.isFile()) continue;
            scannedFiles += 1;
            assert.ok(scannedFiles <= 4096,
                "Firefox plaintext review exceeded file budget");
            const descriptor = fs.openSync(fullPath, "r");
            try {
                let overlap = Buffer.alloc(0);
                for (;;) {
                    const read = fs.readSync(descriptor, chunk, 0,
                        chunk.length, null);
                    if (read === 0) break;
                    scannedBytes += read;
                    assert.ok(scannedBytes <= 512 * 1024 * 1024,
                        "Firefox plaintext review exceeded byte budget");
                    const combined = Buffer.concat([
                        overlap, chunk.subarray(0, read),
                    ]);
                    if (combined.includes(needle)) return true;
                    overlap = combined.subarray(Math.max(0,
                        combined.length - needle.length + 1));
                }
            } finally {
                fs.closeSync(descriptor);
            }
        }
        return false;
    };
    return inspect(profile);
}

test(
    "Firefox explicit Markdown download preserves session source bytes",
    { skip: !FIREFOX_AVAILABLE, timeout: FIREFOX_CASE_TIMEOUT_MS },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/source-markdown-");
        const requests = [];
        const server = createSessionFixtureServer(requests);
        const children = [];
        let bidi;
        try {
            server.listen(0, "127.0.0.1");
            await once(server, "listening");
            const address = server.address();
            assert.notEqual(address, null);
            assert.notEqual(typeof address, "string");
            bidi = await startBidiFirefox(profile, children);
            const tab = await bidi.command("browsingContext.create", {
                type: "tab",
            });
            await bidi.command("browsingContext.navigate", {
                context: tab.context,
                url: `http://127.0.0.1:${address.port}/` +
                    `#session=${REFRESH_SECRET}`,
                wait: "complete",
            });
            await waitForSessionStatus(bidi, tab.context, "Session ready");
            const baseline = [...requests];
            const outcome = await bidi.command("script.evaluate", {
                awaitPromise: true,
                expression: `(async () => {
                    const button = document.getElementById(
                        "source-markdown-export");
                    const source = document.getElementById("source-input");
                    const initial = button.disabled;
                    let captured = null;
                    let offered = null;
                    const create = URL.createObjectURL;
                    const click = HTMLAnchorElement.prototype.click;
                    try {
                        URL.createObjectURL = (blob) => {
                            captured = blob;
                            return create.call(URL, blob);
                        };
                        HTMLAnchorElement.prototype.click = function () {
                            offered = {
                                filename: this.download, href: this.href,
                            };
                        };
                        button.click();
                        return JSON.stringify({
                            initial,
                            filename: offered?.filename,
                            local: offered?.href.startsWith("blob:"),
                            bytes: captured?.size,
                            mediaType: captured?.type,
                            text: await captured?.text(),
                            status: document.getElementById(
                                "source-markdown-status").textContent,
                            source: source.value,
                        });
                    } finally {
                        URL.createObjectURL = create;
                        HTMLAnchorElement.prototype.click = click;
                    }
                })()`,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.equal(outcome.type, "success");
            assert.equal(outcome.result.type, "string");
            const data = JSON.parse(outcome.result.value);
            assert.equal(data.initial, false);
            assert.equal(data.filename, "atrament-source.md");
            assert.equal(data.local, true);
            assert.equal(data.mediaType, "text/markdown; charset=utf-8");
            assert.equal(data.text, REFRESH_DRAFT.source);
            assert.equal(data.source, REFRESH_DRAFT.source);
            assert.equal(data.bytes,
                new TextEncoder().encode(REFRESH_DRAFT.source).length);
            assert.equal(data.status, "Markdown download requested.");
            assert.deepEqual(requests, baseline,
                "user-requested Blob download issues no network request");
            // The live UI must not treat exact, independently authored
            // Unicode space glyphs as an empty source. Exercise the actual
            // input handler, enabled download control and browser Blob.
            const spacing = await bidi.command("script.evaluate", {
                awaitPromise: true,
                expression: `(async () => {
                    const source = document.getElementById("source-input");
                    const button = document.getElementById(
                        "source-markdown-export");
                    const status = document.getElementById(
                        "source-markdown-status");
                    const create = URL.createObjectURL;
                    const click = HTMLAnchorElement.prototype.click;
                    const results = [];
                    let captured = null;
                    let filename = null;
                    try {
                        URL.createObjectURL = (blob) => {
                            captured = blob;
                            return create.call(URL, blob);
                        };
                        HTMLAnchorElement.prototype.click = function () {
                            filename = this.download;
                        };
                        for (const value of ["\u00A0", "\u2003", "\u2009",
                            "\u202F", "\uFEFF\u00A0", "\uFEFF",
                            String.fromCharCode(32, 9, 10)]) {
                            source.value = value;
                            source.dispatchEvent(new Event("input"));
                            const enabled = !button.disabled;
                            captured = null;
                            filename = null;
                            button.click();
                            results.push({
                                value, enabled, filename,
                                text: captured ? await captured.text() : null,
                                bytes: captured?.size ?? null,
                                octets: captured ? Array.from(
                                    new Uint8Array(await captured.arrayBuffer())
                                ) : null,
                                status: status.textContent,
                            });
                        }
                    } finally {
                        URL.createObjectURL = create;
                        HTMLAnchorElement.prototype.click = click;
                    }
                    return JSON.stringify(results);
                })()`,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.equal(spacing.type, "success");
            assert.equal(spacing.result.type, "string");
            const variants = JSON.parse(spacing.result.value);
            for (const sample of variants.slice(0, 5)) {
                assert.equal(sample.enabled, true);
                assert.equal(sample.filename, "atrament-source.md");
                // Blob.text() consumes the leading UTF-8 BOM, but its
                // downloaded binary bytes must retain that exact prefix.
                assert.equal(sample.text,
                    sample.value.replace(/^\uFEFF/u, ""));
                assert.deepEqual(sample.octets,
                    [...Buffer.from(sample.value, "utf8")]);
                assert.equal(sample.bytes,
                    Buffer.byteLength(sample.value, "utf8"));
                assert.equal(sample.status, "Markdown download requested.");
            }
            for (const blank of variants.slice(5)) {
                assert.equal(blank.enabled, false);
                assert.equal(blank.filename, null);
                assert.equal(blank.text, null);
                assert.equal(blank.octets, null);
                assert.equal(blank.status, "Paste source text to download.");
            }
            // The exact browser Blob, including its BOM octets, must reopen
            // as authored Unicode ink in the independent native Rust CLI.
            const sourceFile = path.join(profile, "source-unicode.md");
            fs.writeFileSync(sourceFile, Buffer.from(variants[4].octets));
            assert.deepEqual(fs.readFileSync(sourceFile),
                Buffer.from("\uFEFF\u00A0", "utf8"));
            const build = spawnSync(PINNED_CARGO, [
                "build", "--quiet", "-p", "atrament_profile_editor_cli",
                "--bin", "atrament-profiles",
            ], {
                env: { ...process.env, CARGO_TARGET_DIR: TARGET_DIR },
                encoding: "utf8", timeout: 60_000,
            });
            assert.equal(build.status, 0, build.stderr);
            const cli = path.join(TARGET_DIR, "debug", "atrament-profiles");
            const ink = ["--glyph-stroke", "\u00A0", "500", "500",
                "2500", "3500", "500", "#123456"];
            const exportInk = (format, sourceFlag, text, output, replace) =>
                spawnSync(cli, [
                    "notebook", "--printed-sheet", "false", ...ink,
                    sourceFlag, text, "--export-paper", format,
                    ...(format === "png"
                        ? ["--png-page", "1", "--dpi", "72"] : []),
                    "--output", output, ...(replace ? ["--replace"] : []),
                ], { encoding: "utf8", timeout: 20_000 });
            for (const format of ["pdf", "png"]) {
                const fileOutput = path.join(profile, `unicode-file.${format}`);
                const inlineOutput = path.join(profile,
                    `unicode-inline.${format}`);
                const fromFile = exportInk(format, "--flow-markdown-file",
                    sourceFile, fileOutput, false);
                assert.equal(fromFile.status, 0, fromFile.stderr);
                const fromInline = exportInk(format, "--flow-markdown",
                    "\u00A0", inlineOutput, false);
                assert.equal(fromInline.status, 0, fromInline.stderr);
                const bytes = fs.readFileSync(fileOutput);
                assert.deepEqual(bytes, fs.readFileSync(inlineOutput),
                    `native ${format} ignores only the initial UTF-8 BOM`);
                if (format === "pdf") {
                    assert.match(bytes.toString("latin1"), /^%PDF-1\.4/);
                    assert.match(bytes.toString("latin1"),
                        /0\.071 0\.204 0\.337 RG/);
                } else {
                    assert.equal(bytes.subarray(0, 8).toString("hex"),
                        "89504e470d0a1a0a");
                    assert.equal(bytes[25], 6, "PNG must retain RGBA");
                    const segments = [];
                    for (let offset = 8; offset + 12 <= bytes.length;) {
                        const length = bytes.readUInt32BE(offset);
                        if (bytes.toString("ascii", offset + 4,
                            offset + 8) === "IDAT") {
                            segments.push(bytes.subarray(
                                offset + 8, offset + 8 + length));
                        }
                        offset += length + 12;
                    }
                    assert.ok(segments.length > 0);
                    const pixels = inflateSync(Buffer.concat(segments));
                    assert.ok(pixels.includes(Buffer.from([18, 52, 86, 255])),
                        "PNG must include exact authored Unicode ink color");
                }
            }
            const acceptedPdf = path.join(profile, "unicode-file.pdf");
            const originalPdf = fs.readFileSync(acceptedPdf);
            const unsupported = exportInk("pdf", "--flow-markdown",
                "\u2003", acceptedPdf, true);
            assert.notEqual(unsupported.status, 0);
            assert.match(unsupported.stderr, /missing authored glyph shapes/);
            assert.deepEqual(fs.readFileSync(acceptedPdf), originalPdf,
                "missing exact Unicode space cannot replace native PDF");
        } finally {
            bidi?.close();
            await closeFixtureServer(server);
            for (const child of children.reverse()) await stopChild(child);
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox draft edits and explicit End leave browser stores empty",
    { skip: !FIREFOX_AVAILABLE, timeout: FIREFOX_CASE_TIMEOUT_MS },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/session-browser-storage-");
        const requests = [];
        let markDraftWritten;
        const draftWritten = new Promise((resolve) => {
            markDraftWritten = resolve;
        });
        let markEnded;
        const ended = new Promise((resolve) => { markEnded = resolve; });
        const allRequests = [];
        const server = createSessionFixtureServer(
            requests,
            (request, response, field) => {
                assert.equal(field, "task");
                request.resume();
                request.once("end", () => {
                    response.writeHead(204, { "Cache-Control": "no-store" });
                    response.end();
                    markDraftWritten();
                });
            },
            () => markEnded(),
        );
        // Capture browser requests before the fixture router even for
        // unknown paths that the fixture rejects with 404.
        server.prependListener("request", (request) => {
            allRequests.push(`${request.method} ${request.url}`);
        });
        const children = [];
        let bidi;
        let foreignOriginControl;
        try {
            server.listen(0, "127.0.0.1");
            await once(server, "listening");
            const address = server.address();
            assert.notEqual(address, null);
            assert.notEqual(typeof address, "string");
            bidi = await startBidiFirefox(profile, children);
            const tab = await bidi.command("browsingContext.create", {
                type: "tab",
            });
            await bidi.command("session.subscribe", {
                events: ["network.beforeRequestSent"],
                contexts: [tab.context],
            });
            await bidi.command("browsingContext.navigate", {
                context: tab.context,
                url: `http://127.0.0.1:${address.port}/` +
                    `#session=${REFRESH_SECRET}`,
                wait: "complete",
            });
            await waitForSessionStatus(bidi, tab.context, "Session ready");
            const expected = {
                localStorage: 0,
                sessionStorage: 0,
                cookie: "",
                indexedDB: 0,
                cacheStorage: 0,
            };
            assert.deepEqual(await browserStorageInventory(bidi, tab.context),
                expected, "new Firefox profile starts without browser stores");
            const edited = await bidi.command("script.evaluate", {
                awaitPromise: false,
                expression: `(() => {
                    const task = document.getElementById("task-input");
                    task.value = "ephemeral synthetic privacy draft";
                    task.dispatchEvent(new Event("input", { bubbles: true }));
                })()`,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.equal(edited.type, "success");
            await observedWithin(draftWritten, "synthetic draft mutation");
            assert.ok(requests.includes("POST /api/session/task"));
            assert.deepEqual(await browserStorageInventory(bidi, tab.context),
                expected, "accepted edit must not persist into browser stores");
            const confirmation = await bidi.command("script.evaluate", {
                awaitPromise: false,
                expression: `(() => {
                    const end = document.getElementById("end-session");
                    end.click();
                    end.click();
                })()`,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.equal(confirmation.type, "success");
            await observedWithin(ended, "synthetic session end");
            const final = await waitForSessionStatus(
                bidi, tab.context, "Session ended");
            assert.deepEqual(final.task, { disabled: true, value: "" });
            assert.deepEqual(await browserStorageInventory(bidi, tab.context),
                expected, "ended session must not leave browser stores");
            const approvedRoutes = [
                "GET /",
                "GET /workspace.css",
                "GET /api/handwriting/dimensions",
                "GET /api/notebook/profile",
                "GET /api/paper/profile",
                "GET /generated/handwriting-profile.js",
                "GET /generated/glyph-strokes.js",
                "GET /generated/markdown-export.js",
                "GET /generated/main.js",
                "GET /generated/paper-profile.js",
                "GET /generated/session-diagnostic.js",
                "GET /generated/session-draft.js",
                "GET /generated/session-fragment.js",
                "GET /generated/session-handshake.js",
                "POST /api/handshake",
                "GET /api/session/task",
                "GET /api/session/source",
                "GET /api/session/candidate",
                "POST /api/session/task",
                "POST /api/session/end",
            ].sort();
            // A future accidental URL leak must not appear in test output.
            assert.equal(JSON.stringify(allRequests.sort())
                === JSON.stringify(approvedRoutes), true,
                "real browser must issue only the reviewed requests");
            const browserRequests = bidi.events.filter((event) =>
                event.type === "event"
                    && event.method === "network.beforeRequestSent");
            assert.ok(browserRequests.length > 0,
                "Firefox network observation must not silently be absent");
            // The loopback router cannot see requests to other hosts. BiDi
            // observes network attempts independently of that server.
            const localRequests = [];
            let dataIconEvents = 0;
            for (const event of browserRequests) {
                assert.equal(event.params.context, tab.context);
                assert.equal(typeof event.params.request?.url, "string");
                let url;
                try {
                    url = new URL(event.params.request.url);
                } catch {
                    assert.fail("Firefox reported a malformed request URL");
                }
                // Firefox also reports the document's inert data:, icon
                // as a load event even though it has no network origin.
                if (url.href === "data:,") {
                    dataIconEvents += 1;
                    continue;
                }
                assert.equal(
                    url.origin === `http://127.0.0.1:${address.port}`,
                    true,
                    "workspace made a request outside its loopback origin");
                assert.equal(url.search.length, 0,
                    "unexpected query in request");
                localRequests.push(
                    `${event.params.request.method} ${url.pathname}`,
                );
            }
            assert.ok(dataIconEvents <= 1,
                "only the reviewed inert icon may use a data URL");
            // BiDi may omit initial document and cached static-resource
            // events. Every observed request must still be router-admitted.
            assert.ok(localRequests.every(
                (route) => allRequests.includes(route),
            ), "BiDi network events must be admitted by the router");
            assert.ok(localRequests.includes("POST /api/session/task"),
                "BiDi must observe a real draft mutation");

            // Positive control: the same BiDi context must also report an
            // actual navigation to a different test-owned loopback origin.
            // Only run it after asserting the ended workspace's inventory.
            let controlVisits = 0;
            foreignOriginControl = http.createServer((request, response) => {
                if (request.url === "/network-observer-control") {
                    controlVisits += 1;
                }
                response.writeHead(200, {
                    "Content-Type": "text/html; charset=utf-8",
                });
                response.end("<!doctype html><title>Network control</title>");
            });
            foreignOriginControl.listen(0, "127.0.0.1");
            await once(foreignOriginControl, "listening");
            const controlAddress = foreignOriginControl.address();
            assert.notEqual(controlAddress, null);
            assert.notEqual(typeof controlAddress, "string");
            const controlUrl = `http://127.0.0.1:${controlAddress.port}`
                + "/network-observer-control";
            await bidi.command("browsingContext.navigate", {
                context: tab.context,
                url: controlUrl,
                wait: "complete",
            });
            assert.equal(controlVisits, 1,
                "test-owned second origin must receive its navigation");
            assert.equal(bidi.events.some((event) =>
                event.type === "event"
                    && event.method === "network.beforeRequestSent"
                    && event.params.context === tab.context
                    && event.params.request?.url === controlUrl), true,
                "BiDi must observe an off-origin request in the same tab");
        } finally {
            bidi?.close();
            if (foreignOriginControl !== undefined) {
                await closeFixtureServer(foreignOriginControl);
            }
            await closeFixtureServer(server);
            for (const child of children.reverse()) {
                await stopChild(child);
            }
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

// End's production response asks the browser to clear this origin's storage.
// Seed test-only storage first, and contrast the identical End response
// without the header: an empty-store assertion alone cannot prove the cause.
for (const clearStorage of [false, true]) {
    test(
        clearStorage
            ? "Firefox End storage header clears seeded origin stores"
            : "Firefox End without storage header preserves seeded stores",
        { skip: !FIREFOX_AVAILABLE, timeout: FIREFOX_CASE_TIMEOUT_MS },
        async () => {
            fs.mkdirSync(".temp", { recursive: true });
            const profile = fs.mkdtempSync(".temp/session-clear-storage-");
            const calls = [];
            let markEnded;
            const ended = new Promise((resolve) => { markEnded = resolve; });
            const server = createSessionFixtureServer(
                calls, null, null, null, null, null, null, null,
                null, null, null,
                (_request, response) => {
                    const headers = { "Cache-Control": "no-store" };
                    if (clearStorage) headers["Clear-Site-Data"] = '"storage"';
                    response.writeHead(204, headers);
                    response.end();
                    markEnded();
                    return true;
                },
            );
            const children = [];
            let bidi;
            try {
                server.listen(0, "127.0.0.1");
                await once(server, "listening");
                const address = server.address();
                assert.notEqual(address, null);
                assert.notEqual(typeof address, "string");
                bidi = await startBidiFirefox(profile, children);
                const tab = await bidi.command("browsingContext.create", {
                    type: "tab",
                });
                await bidi.command("browsingContext.navigate", {
                    context: tab.context,
                    url: `http://127.0.0.1:${address.port}/` +
                        `#session=${REFRESH_SECRET}`,
                    wait: "complete",
                });
                await waitForSessionStatus(bidi, tab.context, "Session ready");
                const seeded = await bidi.command("script.evaluate", {
                    awaitPromise: true,
                    expression: `(async () => {
                        localStorage.setItem("atrament-test", "probe");
                        sessionStorage.setItem("atrament-test", "probe");
                        const cache = await caches.open("atrament-test");
                        await cache.put("/test", new Response("probe"));
                        await new Promise((resolve, reject) => {
                            const request = indexedDB.open("atrament-test", 1);
                            request.onerror = () => reject(request.error);
                            request.onsuccess = () => {
                                request.result.close();
                                resolve();
                            };
                        });
                        return true;
                    })()`,
                    resultOwnership: "none",
                    target: { context: tab.context },
                });
                assert.equal(seeded.type, "success");
                assert.deepEqual(seeded.result,
                    { type: "boolean", value: true });
                assert.deepEqual(
                    await browserStorageInventory(bidi, tab.context), {
                        localStorage: 1,
                        sessionStorage: 1,
                        cookie: "",
                        indexedDB: 1,
                        cacheStorage: 1,
                    },
                    "fresh test profile must contain all seeded origin stores",
                );
                const confirmation = await bidi.command("script.evaluate", {
                    awaitPromise: false,
                    expression: `(() => {
                        const end = document.getElementById("end-session");
                        end.click();
                        end.click();
                    })()`,
                    resultOwnership: "none",
                    target: { context: tab.context },
                });
                assert.equal(confirmation.type, "success");
                await observedWithin(ended, "storage policy session End");
                await waitForSessionStatus(bidi, tab.context, "Session ended");
                const stores = await browserStorageInventory(bidi, tab.context);
                const count = clearStorage ? 0 : 1;
                assert.deepEqual(stores, {
                    localStorage: count,
                    sessionStorage: count,
                    cookie: "",
                    indexedDB: count,
                    cacheStorage: count,
                }, "End storage behavior must depend on response header");
                assert.ok(calls.includes("POST /api/session/end"));
                await bidi.command("browsingContext.close", {
                    context: tab.context,
                });
            } finally {
                bidi?.close();
                await closeFixtureServer(server);
                for (const child of children.reverse()) await stopChild(child);
                fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);
}

test(
    "Firefox End does not leave literal private drafts in its disk profile",
    { skip: !FIREFOX_AVAILABLE, timeout: FIREFOX_CASE_TIMEOUT_MS },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/session-profile-plaintext-");
        // Distinct synthetic content covers all private textarea surfaces.
        // No assertion or failure should print a marker's private content.
        const drafts = Object.fromEntries(
            ["task", "source", "candidate"].map((field) => [
                field, `atrament-${field}-end-private-` +
                    crypto.randomBytes(24).toString("hex"),
            ]),
        );
        const calls = [];
        const writtenFields = new Set();
        let markAllWritten;
        const allWritten = new Promise((resolve) => {
            markAllWritten = resolve;
        });
        let markEnded;
        const ended = new Promise((resolve) => { markEnded = resolve; });
        const server = createSessionFixtureServer(
            calls,
            (request, response, field) => {
                assert.ok(Object.hasOwn(drafts, field));
                request.resume();
                request.once("end", () => {
                    response.writeHead(204, { "Cache-Control": "no-store" });
                    response.end();
                    writtenFields.add(field);
                    if (writtenFields.size === 3) markAllWritten();
                });
            },
            () => markEnded(),
        );
        const children = [];
        let bidi;
        try {
            server.listen(0, "127.0.0.1");
            await once(server, "listening");
            const address = server.address();
            assert.notEqual(address, null);
            assert.notEqual(typeof address, "string");
            bidi = await startBidiFirefox(profile, children);
            const tab = await bidi.command("browsingContext.create", {
                type: "tab",
            });
            await bidi.command("browsingContext.navigate", {
                context: tab.context,
                url: `http://127.0.0.1:${address.port}/` +
                    `#session=${REFRESH_SECRET}`,
                wait: "complete",
            });
            await waitForSessionStatus(bidi, tab.context, "Session ready");
            const edit = await bidi.command("script.evaluate", {
                awaitPromise: false,
                expression: `(() => {
                    const drafts = ${JSON.stringify(drafts)};
                    for (const [field, value] of Object.entries(drafts)) {
                        const input = document.getElementById(field + "-input");
                        input.value = value;
                        input.dispatchEvent(new Event("input", {
                            bubbles: true
                        }));
                    }
                })()`,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.equal(edit.type, "success");
            await observedWithin(allWritten, "three private End draft writes");
            await waitForSessionStatus(bidi, tab.context, "Session ready");
            const resident = await bidi.command("script.evaluate", {
                awaitPromise: false,
                expression: `(() => {
                    const drafts = ${JSON.stringify(drafts)};
                    return Object.entries(drafts).every(([field, value]) =>
                        document.getElementById(field + "-input").value
                            === value);
                })()`,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.equal(resident.type, "success");
            assert.deepEqual(resident.result, { type: "boolean", value: true });
            const end = await bidi.command("script.evaluate", {
                awaitPromise: false,
                expression: `(() => {
                    const end = document.getElementById("end-session");
                    end.click();
                    end.click();
                })()`,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.equal(end.type, "success");
            await observedWithin(ended, "private synthetic session end");
            const final = await waitForSessionStatus(
                bidi, tab.context, "Session ended");
            for (const field of Object.keys(drafts)) {
                assert.deepEqual(final[field], { disabled: true, value: "" },
                    `${field} cleared and disabled after confirmed End`);
                assert.ok(calls.includes(`POST /api/session/${field}`));
            }
            await bidi.command("browsingContext.close", {
                context: tab.context,
            });
            bidi.close();
            bidi = null;
            for (const child of children.reverse()) await stopChild(child);
            children.length = 0;

            // Positive control: the scanner detects a marker straddling its
            // read boundary. Remove this test-owned seed before the real scan.
            const control = path.join(profile, "plaintext-control.bin");
            fs.writeFileSync(control, Buffer.concat([
                Buffer.alloc(64 * 1024 - 8),
                Buffer.from(Object.values(drafts).join("\n"), "utf8"),
            ]), { mode: 0o600 });
            for (const marker of Object.values(drafts)) {
                assert.equal(firefoxProfileContainsPlaintext(profile, marker),
                    true, "test-owned private marker must be detectable");
            }
            fs.rmSync(control);
            for (const [field, marker] of Object.entries(drafts)) {
                assert.equal(firefoxProfileContainsPlaintext(profile, marker),
                    false, `ended Firefox profile retained ${field} draft`);
            }
        } finally {
            bidi?.close();
            await closeFixtureServer(server);
            for (const child of children.reverse()) await stopChild(child);
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox forced death leaves no literal private draft in its profile",
    { skip: !FIREFOX_AVAILABLE, timeout: FIREFOX_CASE_TIMEOUT_MS },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/session-profile-hard-stop-");
        const drafts = Object.fromEntries(
            ["task", "source", "candidate"].map((field) => [
                field, `atrament-${field}-private-` +
                    crypto.randomBytes(24).toString("hex"),
            ]),
        );
        const writtenFields = new Set();
        let markAllWritten;
        const allWritten = new Promise((resolve) => {
            markAllWritten = resolve;
        });
        const calls = [];
        const server = createSessionFixtureServer(
            calls,
            (request, response, field) => {
                assert.ok(Object.hasOwn(drafts, field));
                request.resume();
                request.once("end", () => {
                    response.writeHead(204, { "Cache-Control": "no-store" });
                    response.end();
                    writtenFields.add(field);
                    if (writtenFields.size === 3) markAllWritten();
                });
            },
        );
        const children = [];
        let bidi;
        try {
            server.listen(0, "127.0.0.1");
            await once(server, "listening");
            const address = server.address();
            assert.notEqual(address, null);
            assert.notEqual(typeof address, "string");
            bidi = await startBidiFirefox(profile, children);
            const tab = await bidi.command("browsingContext.create", {
                type: "tab",
            });
            await bidi.command("browsingContext.navigate", {
                context: tab.context,
                url: `http://127.0.0.1:${address.port}/` +
                    `#session=${REFRESH_SECRET}`,
                wait: "complete",
            });
            await waitForSessionStatus(bidi, tab.context, "Session ready");
            const edited = await bidi.command("script.evaluate", {
                awaitPromise: false,
                expression: `(() => {
                    const drafts = ${JSON.stringify(drafts)};
                    for (const [field, value] of Object.entries(drafts)) {
                        const input = document.getElementById(field + "-input");
                        input.value = value;
                        input.dispatchEvent(new Event("input", {
                            bubbles: true
                        }));
                    }
                })()`,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.equal(edited.type, "success");
            await observedWithin(allWritten, "hard-stop three draft writes");
            await waitForSessionStatus(bidi, tab.context, "Session ready");
            const resident = await bidi.command("script.evaluate", {
                awaitPromise: false,
                expression: `(() => {
                    const drafts = ${JSON.stringify(drafts)};
                    return Object.entries(drafts).every(([field, value]) =>
                        document.getElementById(field + "-input").value
                            === value);
                })()`,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.equal(resident.type, "success");
            assert.deepEqual(resident.result, { type: "boolean", value: true });

            // No End, tab close, or pagehide: kill the disposable browser
            // directly to examine what an abrupt browser exit left on disk.
            const firefox = children[0];
            assert.ok(firefox.kill("SIGKILL"),
                "test-owned Firefox process must accept abrupt stop");
            await observedWithin(once(firefox, "exit"),
                "test-owned Firefox hard stop");
            bidi.close();
            bidi = null;
            const control = path.join(profile, "hard-stop-control.bin");
            // A positive control for each independently edited field proves
            // the bounded scanner would detect literal data if it survived.
            fs.writeFileSync(control,
                Object.values(drafts).join("\n"), { mode: 0o600 });
            for (const marker of Object.values(drafts)) {
                assert.equal(firefoxProfileContainsPlaintext(profile, marker),
                    true, "test-owned private marker must be detectable");
            }
            fs.rmSync(control);
            for (const [field, marker] of Object.entries(drafts)) {
                assert.equal(firefoxProfileContainsPlaintext(profile, marker),
                    false, `aborted Firefox profile retained ${field} draft`);
                assert.ok(calls.includes(`POST /api/session/${field}`),
                    `hard stop must follow acknowledged ${field} write`);
            }
            assert.ok(!calls.includes("POST /api/session/end"),
                "hard stop must not imply confirmed backend erasure");
        } finally {
            bidi?.close();
            await closeFixtureServer(server);
            for (const child of children.reverse()) await stopChild(child);
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

// Navigating away need not terminate the browser. A subsequent hard stop
// can still leave browser recovery data even after the pagehide handler has
// cleared script-visible state. Inspect only a fresh, test-owned profile.
test(
    "Firefox hard stop after navigation leaves no literal private draft",
    { skip: !FIREFOX_AVAILABLE, timeout: FIREFOX_CASE_TIMEOUT_MS },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/session-away-hard-stop-");
        const drafts = Object.fromEntries(
            ["task", "source", "candidate"].map((field) => [
                field, `atrament-${field}-away-private-` +
                    crypto.randomBytes(24).toString("hex"),
            ]),
        );
        const calls = [];
        const writtenFields = new Set();
        let markAllWritten;
        const allWritten = new Promise((resolve) => {
            markAllWritten = resolve;
        });
        const server = createSessionFixtureServer(
            calls,
            (request, response, field) => {
                assert.ok(Object.hasOwn(drafts, field));
                request.resume();
                request.once("end", () => {
                    response.writeHead(204, { "Cache-Control": "no-store" });
                    response.end();
                    writtenFields.add(field);
                    if (writtenFields.size === 3) markAllWritten();
                });
            },
        );
        const children = [];
        let bidi;
        try {
            server.listen(0, "127.0.0.1");
            await once(server, "listening");
            const address = server.address();
            assert.notEqual(address, null);
            assert.notEqual(typeof address, "string");
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
                bidi, tab.context, "Session ready",
            );
            assert.equal(ready.task.value, REFRESH_DRAFT.task);
            assert.equal(ready.hash, "");
            const edited = await bidi.command("script.evaluate", {
                awaitPromise: false,
                expression: `(() => {
                    const drafts = ${JSON.stringify(drafts)};
                    for (const [field, value] of Object.entries(drafts)) {
                        const input = document.getElementById(field + "-input");
                        input.value = value;
                        input.dispatchEvent(new Event("input", {
                            bubbles: true
                        }));
                    }
                })()`,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.equal(edited.type, "success");
            await observedWithin(allWritten, "accepted three departed drafts");
            await waitForSessionStatus(bidi, tab.context, "Session ready");
            const resident = await bidi.command("script.evaluate", {
                awaitPromise: false,
                expression: `(() => {
                    const drafts = ${JSON.stringify(drafts)};
                    return Object.entries(drafts).every(([field, value]) =>
                        document.getElementById(field + "-input").value
                            === value);
                })()`,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.deepEqual(resident.result, { type: "boolean", value: true });
            await bidi.command("browsingContext.navigate", {
                context: tab.context,
                url: `${origin}/missing`,
                wait: "complete",
            });
            const away = await bidi.command("script.evaluate", {
                awaitPromise: false,
                expression: `JSON.stringify({
                    path: location.pathname,
                    hasTask: document.querySelector("#task-input") !== null
                })`,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.equal(away.type, "success");
            assert.equal(away.result.type, "string");
            assert.deepEqual(JSON.parse(away.result.value), {
                path: "/missing", hasTask: false,
            });
            assert.deepEqual(calls.slice(0, 4), [
                "POST /api/handshake",
                "GET /api/session/task",
                "GET /api/session/source",
                "GET /api/session/candidate",
            ]);
            assert.deepEqual(calls.slice(4).sort(), [
                "POST /api/session/task",
                "POST /api/session/source",
                "POST /api/session/candidate",
            ].sort());
            // No End, tab close, or history return: terminate the actual
            // browser after the private page has left the active document.
            const firefox = children[0];
            const exited = once(firefox, "exit");
            assert.ok(firefox.kill("SIGKILL"),
                "test-owned Firefox must accept abrupt stop");
            await observedWithin(exited, "Firefox hard stop after pagehide");
            bidi.close();
            bidi = null;
            const control = path.join(profile, "away-control.bin");
            fs.writeFileSync(control,
                Object.values(drafts).join("\n"), { mode: 0o600 });
            for (const marker of Object.values(drafts)) {
                assert.equal(firefoxProfileContainsPlaintext(profile, marker),
                    true, "test-owned departed marker must be detectable");
            }
            fs.rmSync(control);
            for (const [field, marker] of Object.entries(drafts)) {
                assert.equal(firefoxProfileContainsPlaintext(profile, marker),
                    false, `Firefox recovery retained ${field} draft`);
            }
            assert.equal(calls.includes("POST /api/session/end"), false,
                "hard stop must not imply backend erasure");
        } finally {
            bidi?.close();
            await closeFixtureServer(server);
            for (const child of children.reverse()) await stopChild(child);
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox pagehide aborts a pending explicit session end request",
    { skip: !FIREFOX_AVAILABLE, timeout: FIREFOX_CASE_TIMEOUT_MS },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/session-end-pagehide-");
        const apiRequests = [];
        let markEndStarted;
        const endStarted = new Promise((resolve) => {
            markEndStarted = resolve;
        });
        let markEndClosed;
        const endClosed = new Promise((resolve) => {
            markEndClosed = resolve;
        });
        let pendingResponse = null;
        const server = createSessionFixtureServer(
            apiRequests, null, null, null, null, null, null, null, null,
            null, null,
            (_request, response) => {
                pendingResponse = response;
                response.once("close", markEndClosed);
                markEndStarted();
                return true;
            },
        );
        const children = [];
        let bidi;
        try {
            server.listen(0, "127.0.0.1");
            await once(server, "listening");
            const address = server.address();
            assert.notEqual(address, null);
            assert.notEqual(typeof address, "string");
            bidi = await startBidiFirefox(profile, children);
            const tab = await bidi.command("browsingContext.create", {
                type: "tab",
            });
            await bidi.command("browsingContext.navigate", {
                context: tab.context,
                url: `http://127.0.0.1:${address.port}/` +
                    `#session=${REFRESH_SECRET}`,
                wait: "complete",
            });
            await waitForSessionStatus(bidi, tab.context, "Session ready");
            const confirm = await bidi.command("script.evaluate", {
                awaitPromise: false,
                expression: `(() => {
                    const end = document.getElementById("end-session");
                    end.click();
                    end.click();
                })()`,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.equal(confirm.type, "success");
            await observedWithin(endStarted, "pending explicit end request");
            const before = await evaluateSessionDocument(bidi, tab.context);
            assert.equal(before.status, "Ending discards this notebook");
            assert.equal(before.task.value, REFRESH_DRAFT.task);
            // A synthetic pagehide exercises the browser's own lifetime
            // handler without actual navigation implicitly canceling fetch.
            const exit = await bidi.command("script.evaluate", {
                awaitPromise: false,
                expression: 'window.dispatchEvent(new Event("pagehide"))',
                resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.equal(exit.type, "success");
            await observedWithin(endClosed, "pagehide end cancellation");
            const after = await evaluateSessionDocument(bidi, tab.context);
            assert.equal(after.task.value, "");
            assert.equal(after.task.disabled, true);
            assert.notEqual(after.status, "Session ended");
            assert.deepEqual(apiRequests, [
                "POST /api/handshake",
                "GET /api/session/task",
                "GET /api/session/source",
                "GET /api/session/candidate",
                "POST /api/session/end",
            ]);
        } finally {
            if (pendingResponse !== null && !pendingResponse.destroyed) {
                pendingResponse.destroy();
            }
            bidi?.close();
            await closeFixtureServer(server);
            for (const child of children.reverse()) {
                await stopChild(child);
            }
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox stalled explicit end never claims erasure and permits retry",
    { skip: !FIREFOX_AVAILABLE, timeout: FIREFOX_CASE_TIMEOUT_MS },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/session-end-timeout-");
        const apiRequests = [];
        let endAttempts = 0;
        let signalClosed;
        const firstEndClosed = new Promise((resolve) => {
            signalClosed = resolve;
        });
        const server = createSessionFixtureServer(
            apiRequests, null, null, null, null, null, null, null, null,
            null, null,
            (_request, response) => {
                if (++endAttempts !== 1) {
                    return false;
                }
                // No response headers: the client must bound fetch itself,
                // not only the successful response-body decoder.
                response.once("close", signalClosed);
                return true;
            },
        );
        const children = [];
        let bidi;
        try {
            server.listen(0, "127.0.0.1");
            await once(server, "listening");
            const address = server.address();
            assert.notEqual(address, null);
            assert.notEqual(typeof address, "string");
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
            const initial = await waitForSessionStatus(
                bidi, tab.context, "Session ready",
            );
            assert.equal(initial.hash, "");
            const confirmEnd = async () => {
                const response = await bidi.command("script.evaluate", {
                    awaitPromise: false,
                    expression: `(() => {
                        const end = document.getElementById("end-session");
                        if (end.disabled) {
                            throw Error("session end unexpectedly disabled");
                        }
                        end.click();
                        end.click();
                    })()`,
                    resultOwnership: "none",
                    target: { context: tab.context },
                });
                assert.equal(response.type, "success");
            };
            await confirmEnd();
            const failed = await waitForSessionStatus(
                bidi, tab.context, "End session failed · retry", 400,
            );
            assert.equal(failed.hash, "");
            for (const field of ["task", "source", "candidate"]) {
                assert.deepEqual(failed[field], {
                    disabled: false, value: REFRESH_DRAFT[field],
                });
            }
            await observedWithin(firstEndClosed, "stalled session end abort");
            assert.equal(endAttempts, 1);
            await confirmEnd();
            const ended = await waitForSessionStatus(
                bidi, tab.context, "Session ended",
            );
            for (const field of ["task", "source", "candidate"]) {
                assert.deepEqual(ended[field], {
                    disabled: true, value: "",
                });
            }
            assert.equal(endAttempts, 2);
            assert.deepEqual(apiRequests, [
                "POST /api/handshake",
                "GET /api/session/task",
                "GET /api/session/source",
                "GET /api/session/candidate",
                "POST /api/session/end",
                "POST /api/session/end",
            ]);
        } finally {
            bidi?.close();
            await closeFixtureServer(server);
            for (const child of children.reverse()) {
                await stopChild(child);
            }
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox keeps end confirmation visible after a late draft rejection",
    { skip: !FIREFOX_AVAILABLE, timeout: FIREFOX_CASE_TIMEOUT_MS },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/session-end-armed-race-");
        const apiRequests = [];
        let markDraftStarted;
        const draftStarted = new Promise((resolve) => {
            markDraftStarted = resolve;
        });
        let rejectDraft;
        const server = createSessionFixtureServer(
            apiRequests,
            (request, response, field) => {
                assert.equal(field, "task");
                request.resume();
                request.once("end", () => {
                    rejectDraft = () => {
                        response.writeHead(503, {
                            "Cache-Control": "no-store",
                        });
                        response.end();
                    };
                    markDraftStarted();
                });
            },
        );
        const children = [];
        let bidi;
        try {
            server.listen(0, "127.0.0.1");
            await once(server, "listening");
            const address = server.address();
            assert.notEqual(address, null);
            assert.notEqual(typeof address, "string");
            bidi = await startBidiFirefox(profile, children);
            const tab = await bidi.command("browsingContext.create", {
                type: "tab",
            });
            await bidi.command("browsingContext.navigate", {
                context: tab.context,
                url: `http://127.0.0.1:${address.port}/` +
                    `#session=${REFRESH_SECRET}`,
                wait: "complete",
            });
            await waitForSessionStatus(bidi, tab.context, "Session ready");
            const edit = await bidi.command("script.evaluate", {
                awaitPromise: false,
                expression: `(() => {
                    const nativeFetch = window.fetch.bind(window);
                    globalThis.__draftRejectSeen = false;
                    window.fetch = (url, options) => nativeFetch(url, options)
                        .then((response) => {
                            if (url === "./api/session/task"
                                && options?.method === "POST") {
                                globalThis.__draftRejectSeen = true;
                            }
                            return response;
                        });
                    const task = document.getElementById("task-input");
                    task.value = "pending private draft";
                    task.dispatchEvent(new Event("input"));
                })()`,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.equal(edit.type, "success");
            await observedWithin(draftStarted, "pending private draft request");
            const arm = await bidi.command("script.evaluate", {
                awaitPromise: false,
                expression: 'document.getElementById("end-session").click()',
                resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.equal(arm.type, "success");
            rejectDraft();
            const settled = await bidi.command("script.evaluate", {
                awaitPromise: true,
                expression: `new Promise((resolve) => {
                    const deadline = Date.now() + 2000;
                    function inspect() {
                        if (globalThis.__draftRejectSeen) {
                            setTimeout(() => resolve(true), 100);
                        } else if (Date.now() >= deadline) {
                            resolve(false);
                        } else {
                            setTimeout(inspect, 20);
                        }
                    }
                    inspect();
                })`,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.deepEqual(settled.result, {
                type: "boolean", value: true,
            });
            const armed = await evaluateSessionDocument(bidi, tab.context);
            assert.equal(armed.status, "Ending discards this notebook");
            assert.equal(armed.task.value, "pending private draft");
            const cancel = await bidi.command("script.evaluate", {
                awaitPromise: false,
                expression: `(() => {
                    const end = document.getElementById("end-session");
                    end.dispatchEvent(new KeyboardEvent("keydown", {
                        key: "Escape", bubbles: true,
                    }));
                })()`,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.equal(cancel.type, "success");
            const disarmed = await evaluateSessionDocument(bidi, tab.context);
            assert.equal(disarmed.status, "Draft sync rejected · retry edit");
            assert.deepEqual(apiRequests, [
                "POST /api/handshake",
                "GET /api/session/task",
                "GET /api/session/source",
                "GET /api/session/candidate",
                "POST /api/session/task",
            ]);
        } finally {
            bidi?.close();
            await closeFixtureServer(server);
            for (const child of children.reverse()) {
                await stopChild(child);
            }
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox preserves failed end warning after a pending draft completes",
    { skip: !FIREFOX_AVAILABLE, timeout: FIREFOX_CASE_TIMEOUT_MS },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/session-end-draft-race-");
        const apiRequests = [];
        let markDraftStarted;
        const draftStarted = new Promise((resolve) => {
            markDraftStarted = resolve;
        });
        let finishDraft;
        const server = createSessionFixtureServer(
            apiRequests,
            (request, response, field) => {
                assert.equal(field, "task");
                request.resume();
                request.once("end", () => {
                    finishDraft = () => {
                        response.writeHead(204, {
                            "Cache-Control": "no-store",
                        });
                        response.end();
                    };
                    markDraftStarted();
                });
            },
            () => 503,
        );
        const children = [];
        let bidi;
        try {
            server.listen(0, "127.0.0.1");
            await once(server, "listening");
            const address = server.address();
            assert.notEqual(address, null);
            assert.notEqual(typeof address, "string");
            bidi = await startBidiFirefox(profile, children);
            const tab = await bidi.command("browsingContext.create", {
                type: "tab",
            });
            await bidi.command("browsingContext.navigate", {
                context: tab.context,
                url: `http://127.0.0.1:${address.port}/` +
                    `#session=${REFRESH_SECRET}`,
                wait: "complete",
            });
            await waitForSessionStatus(bidi, tab.context, "Session ready");
            const edit = await bidi.command("script.evaluate", {
                awaitPromise: false,
                expression: `(() => {
                    const nativeFetch = window.fetch.bind(window);
                    globalThis.__draftAckSeen = false;
                    window.fetch = (url, options) => nativeFetch(url, options)
                        .then((response) => {
                            if (url === "./api/session/task"
                                && options?.method === "POST") {
                                globalThis.__draftAckSeen = true;
                            }
                            return response;
                        });
                    const task = document.getElementById("task-input");
                    task.value = "pending private draft";
                    task.dispatchEvent(new Event("input"));
                })()`,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.equal(edit.type, "success");
            await observedWithin(draftStarted, "pending private draft request");
            const confirm = await bidi.command("script.evaluate", {
                awaitPromise: false,
                expression: `(() => {
                    const end = document.getElementById("end-session");
                    end.click();
                    end.click();
                })()`,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.equal(confirm.type, "success");
            const failed = await waitForSessionStatus(
                bidi, tab.context, "End session failed · retry",
            );
            assert.equal(failed.task.value, "pending private draft");
            assert.equal(failed.hash, "");
            finishDraft();
            // Observe the actual browser fetch acknowledgement, then allow
            // the adapter's awaited continuation to publish its status.
            const settled = await bidi.command("script.evaluate", {
                awaitPromise: true,
                expression: `new Promise((resolve) => {
                    const deadline = Date.now() + 2000;
                    function inspect() {
                        if (globalThis.__draftAckSeen) {
                            setTimeout(() => resolve(true), 100);
                        } else if (Date.now() >= deadline) {
                            resolve(false);
                        } else {
                            setTimeout(inspect, 20);
                        }
                    }
                    inspect();
                })`,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.deepEqual(settled.result, {
                type: "boolean", value: true,
            });
            const after = await evaluateSessionDocument(bidi, tab.context);
            assert.equal(after.status, "End session failed · retry");
            assert.deepEqual(after.task, {
                disabled: false, value: "pending private draft",
            });
            assert.deepEqual(apiRequests, [
                "POST /api/handshake",
                "GET /api/session/task",
                "GET /api/session/source",
                "GET /api/session/candidate",
                "POST /api/session/task",
                "POST /api/session/end",
            ]);
        } finally {
            bidi?.close();
            await closeFixtureServer(server);
            for (const child of children.reverse()) {
                await stopChild(child);
            }
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox end failures preserve retryability but revoke stale authority",
    { skip: !FIREFOX_AVAILABLE, timeout: FIREFOX_CASE_TIMEOUT_MS },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/session-end-rejections-");
        const children = [];
        let bidi;
        try {
            bidi = await startBidiFirefox(profile, children);
            for (const status of [503, 401]) {
                const apiRequests = [];
                const attempts = [];
                const server = createSessionFixtureServer(
                    apiRequests, null, () => {
                        attempts.push(status);
                        return status === 503 && attempts.length > 1
                            ? 204 : status;
                    },
                );
                try {
                    server.listen(0, "127.0.0.1");
                    await once(server, "listening");
                    const address = server.address();
                    assert.notEqual(typeof address, "string");
                    assert.notEqual(address, null);
                    const tab = await bidi.command(
                        "browsingContext.create", { type: "tab" },
                    );
                    await bidi.command("browsingContext.navigate", {
                        context: tab.context,
                        url: `http://127.0.0.1:${address.port}/` +
                            `#session=${REFRESH_SECRET}`,
                        wait: "complete",
                    });
                    await waitForSessionStatus(
                        bidi, tab.context, "Session ready",
                    );
                    for (let click = 0; click < 2; click += 1) {
                        await bidi.command("script.evaluate", {
                            awaitPromise: false,
                            expression:
                                'document.querySelector(' +
                                '"#end-session").click()',
                            resultOwnership: "none",
                            target: { context: tab.context },
                        });
                    }
                    const failed = await waitForSessionStatus(
                        bidi,
                        tab.context,
                        status === 401 ? "Authorization failed"
                            : "End session failed · retry",
                    );
                    assert.equal(attempts.length, 1);
                    assert.equal(
                        apiRequests.at(-1), "POST /api/session/end",
                    );
                    if (status === 401) {
                        for (const field of ["task", "source", "candidate"]) {
                            assert.deepEqual(failed[field], {
                                disabled: true, value: "",
                            });
                        }
                    } else {
                        assert.equal(failed.task.disabled, false);
                        assert.equal(failed.task.value, REFRESH_DRAFT.task);
                        for (let click = 0; click < 2; click += 1) {
                            await bidi.command("script.evaluate", {
                                awaitPromise: false,
                                expression:
                                    'document.querySelector(' +
                                    '"#end-session").click()',
                                resultOwnership: "none",
                                target: { context: tab.context },
                            });
                        }
                        const final = await waitForSessionStatus(
                            bidi, tab.context, "Session ended",
                        );
                        assert.deepEqual(final.task, {
                            disabled: true, value: "",
                        });
                        assert.equal(attempts.length, 2);
                    }
                    await bidi.command("browsingContext.close", {
                        context: tab.context,
                    });
                } finally {
                    await closeFixtureServer(server);
                }
            }
        } finally {
            bidi?.close();
            for (const child of children.reverse()) await stopChild(child);
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox glyph path controls reorder and reverse exact authored TSV",
    { skip: !FIREFOX_AVAILABLE, timeout: FIREFOX_CASE_TIMEOUT_MS },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/glyph-path-browser-");
        const requests = [];
        const server = createSessionFixtureServer(requests);
        const children = [];
        let bidi;
        try {
            server.listen(0, "127.0.0.1");
            await once(server, "listening");
            const address = server.address();
            assert.notEqual(address, null);
            assert.notEqual(typeof address, "string");
            bidi = await startBidiFirefox(profile, children);
            const tab = await bidi.command("browsingContext.create", {
                type: "tab",
            });
            await bidi.command("browsingContext.navigate", {
                context: tab.context,
                url: `http://127.0.0.1:${address.port}/`
                    + `#session=${REFRESH_SECRET}`,
                wait: "complete",
            });
            await waitForSessionStatus(bidi, tab.context, "Session ready");
            const baseline = [...requests];
            const response = await bidi.command("script.evaluate", {
                awaitPromise: true,
                expression: String.raw`(async () => {
                    const find = (id) => document.getElementById(id);
                    const nfd = "n\u0303";
                    const ready = !find("glyph-add").disabled;
                    const add = (glyph, coordinates) => {
                        find("glyph-character").value = glyph;
                        find("glyph-character").dispatchEvent(
                            new Event("input", { bubbles: true }));
                        for (const [axis, value] of Object.entries({
                            x1: coordinates[0], y1: coordinates[1],
                            x2: coordinates[2], y2: coordinates[3],
                            width: 250,
                        })) find("glyph-" + axis).value = String(value);
                        find("glyph-color").value = "#243647";
                        find("glyph-add").click();
                    };
                    add(nfd, [100, 200, 400, 600]);
                    add("ñ", [200, 300, 500, 700]);
                    add(nfd, [400, 600, 700, 900]);
                    add(nfd, [1200, 1500, 1500, 1800]);
                    find("glyph-selected").value = "2";
                    find("glyph-selected").dispatchEvent(
                        new Event("change", { bubbles: true }));
                    const before = find("glyph-paths").textContent;
                    const enabled = {
                        earlier: !find("glyph-path-earlier").disabled,
                        later: !find("glyph-path-later").disabled,
                        reverse: !find("glyph-reverse-path").disabled,
                    };
                    find("glyph-path-later").click();
                    const afterMove = {
                        selected: find("glyph-selected").value,
                        paths: find("glyph-paths").textContent,
                    };
                    find("glyph-reverse-path").click();
                    const afterReverse = {
                        selected: find("glyph-selected").value,
                        paths: find("glyph-paths").textContent,
                    };
                    let bytes = null;
                    let download = null;
                    const create = URL.createObjectURL;
                    const click = HTMLAnchorElement.prototype.click;
                    try {
                        URL.createObjectURL = (blob) => {
                            bytes = blob;
                            return create.call(URL, blob);
                        };
                        HTMLAnchorElement.prototype.click = function () {
                            download = this.download;
                        };
                        find("glyph-save").click();
                        return JSON.stringify({
                            ready, before, enabled, afterMove, afterReverse,
                            download, media: bytes?.type,
                            tsv: await bytes?.text(),
                        });
                    } finally {
                        URL.createObjectURL = create;
                        HTMLAnchorElement.prototype.click = click;
                    }
                })()`,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.equal(response.type, "success");
            assert.equal(response.result.type, "string");
            const actual = JSON.parse(response.result.value);
            assert.equal(actual.ready, true);
            assert.match(actual.before, new RegExp(
                "3 directed strokes, 2 candidate pen paths, "
                    + "1 apparent pen lift"));
            assert.deepEqual(actual.enabled,
                { earlier: false, later: true, reverse: true });
            assert.equal(actual.afterMove.selected, "3");
            assert.match(actual.afterMove.paths,
                /Path 1: strokes 1–1.*Path 2: strokes 2–3/);
            assert.equal(actual.afterReverse.selected, "2");
            assert.match(actual.afterReverse.paths,
                /Path 2: strokes 2–3, \(700, 900\) → \(100, 200\)/);
            assert.equal(actual.download, "my-glyph-strokes.tsv");
            assert.equal(actual.media, "text/tab-separated-values");
            const expected = [
                "atrament.glyph-strokes.v1",
                "n\u0303\t1200\t1500\t1500\t1800\t250\t#243647",
                "ñ\t200\t300\t500\t700\t250\t#243647",
                "n\u0303\t700\t900\t400\t600\t250\t#243647",
                "n\u0303\t400\t600\t100\t200\t250\t#243647",
                "",
            ].join("\n");
            assert.equal(actual.tsv, expected,
                "real browser exports intact Unicode rows and moved paths");
            assert.deepEqual(requests, baseline,
                "path edits and local TSV download never contact the backend");
        } finally {
            bidi?.close();
            await closeFixtureServer(server);
            for (const child of children.reverse()) await stopChild(child);
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox closed loop control changes pen start without network",
    { skip: !FIREFOX_AVAILABLE, timeout: FIREFOX_CASE_TIMEOUT_MS },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/glyph-loop-start-browser-");
        const requests = [];
        const server = createSessionFixtureServer(requests);
        const children = [];
        let bidi;
        try {
            server.listen(0, "127.0.0.1");
            await once(server, "listening");
            const address = server.address();
            assert.notEqual(address, null);
            assert.notEqual(typeof address, "string");
            bidi = await startBidiFirefox(profile, children);
            const tab = await bidi.command("browsingContext.create", {
                type: "tab",
            });
            await bidi.command("browsingContext.navigate", {
                context: tab.context,
                url: `http://127.0.0.1:${address.port}/`
                    + `#session=${REFRESH_SECRET}`,
                wait: "complete",
            });
            await waitForSessionStatus(bidi, tab.context, "Session ready");
            const baseline = [...requests];
            const response = await bidi.command("script.evaluate", {
                awaitPromise: true,
                expression: String.raw`(async () => {
                    const find = (id) => document.getElementById(id);
                    const nfd = "o\u0301";
                    const add = (glyph, coords, color = "#243647") => {
                        find("glyph-character").value = glyph;
                        find("glyph-character").dispatchEvent(
                            new Event("input", { bubbles: true }));
                        for (const [axis, value] of Object.entries({
                            x1: coords[0], y1: coords[1],
                            x2: coords[2], y2: coords[3],
                            width: 250,
                        })) find("glyph-" + axis).value = String(value);
                        find("glyph-color").value = color;
                        find("glyph-add").click();
                    };
                    const choose = (index) => {
                        find("glyph-selected").value = String(index);
                        find("glyph-selected").dispatchEvent(
                            new Event("change", { bubbles: true }));
                    };
                    add(nfd, [400, 500, 1400, 500]);
                    add("ó", [600, 700, 800, 900]);
                    add(nfd, [1400, 500, 1400, 1500]);
                    add("∞", [900, 1000, 1200, 1400]);
                    add(nfd, [1400, 1500, 400, 1500]);
                    add(nfd, [400, 1500, 400, 500]);
                    add(nfd, [1700, 2100, 2000, 2400], "#334455");
                    choose(0);
                    const initialDisabled = find("glyph-path-start").disabled;
                    choose(4);
                    const laterEnabled = !find("glyph-path-start").disabled;
                    const initialPaths = find("glyph-paths").textContent;
                    find("glyph-path-start").click();
                    const rotated = {
                        status: find("glyph-status").textContent,
                        selected: find("glyph-selected").value,
                        disabled: find("glyph-path-start").disabled,
                        paths: find("glyph-paths").textContent,
                    };
                    choose(6);
                    const accentDisabled = find("glyph-path-start").disabled;
                    find("glyph-character").value = "ó";
                    find("glyph-character").dispatchEvent(
                        new Event("input", { bubbles: true }));
                    choose(1);
                    const otherDisabled = find("glyph-path-start").disabled;
                    let bytes = null;
                    let download = null;
                    const create = URL.createObjectURL;
                    const click = HTMLAnchorElement.prototype.click;
                    try {
                        URL.createObjectURL = (blob) => {
                            bytes = blob;
                            return create.call(URL, blob);
                        };
                        HTMLAnchorElement.prototype.click = function () {
                            download = this.download;
                        };
                        find("glyph-save").click();
                        return JSON.stringify({
                            initialDisabled, laterEnabled, initialPaths,
                            rotated, accentDisabled, otherDisabled,
                            download, media: bytes?.type,
                            tsv: await bytes?.text(),
                        });
                    } finally {
                        URL.createObjectURL = create;
                        HTMLAnchorElement.prototype.click = click;
                    }
                })()`,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.equal(response.type, "success");
            assert.equal(response.result.type, "string");
            const actual = JSON.parse(response.result.value);
            assert.equal(actual.initialDisabled, true);
            assert.equal(actual.laterEnabled, true);
            assert.match(actual.initialPaths,
                /5 directed strokes, 2 candidate pen paths/);
            assert.match(actual.rotated.status,
                /Changed closed pen loop starting stroke; unsaved/);
            assert.equal(actual.rotated.selected, "0");
            assert.equal(actual.rotated.disabled, true);
            assert.equal(actual.accentDisabled, true);
            assert.equal(actual.otherDisabled, true);
            assert.match(actual.rotated.paths,
                /Path 1: strokes 1–4, \(1400, 1500\) → \(1400, 1500\)/);
            assert.match(actual.rotated.paths, /\[closed loop\]/);
            assert.equal(actual.download, "my-glyph-strokes.tsv");
            assert.equal(actual.media, "text/tab-separated-values");
            assert.equal(actual.tsv, [
                "atrament.glyph-strokes.v1",
                "o\u0301\t1400\t1500\t400\t1500\t250\t#243647",
                "ó\t600\t700\t800\t900\t250\t#243647",
                "o\u0301\t400\t1500\t400\t500\t250\t#243647",
                "∞\t900\t1000\t1200\t1400\t250\t#243647",
                "o\u0301\t400\t500\t1400\t500\t250\t#243647",
                "o\u0301\t1400\t500\t1400\t1500\t250\t#243647",
                "o\u0301\t1700\t2100\t2000\t2400\t250\t#334455",
                "",
            ].join("\n"));
            assert.deepEqual(requests, baseline,
                "closed-loop editing and TSV download stay browser-local");
        } finally {
            bidi?.close();
            await closeFixtureServer(server);
            for (const child of children.reverse()) await stopChild(child);
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox closes an authored open glyph path without backend requests",
    { skip: !FIREFOX_AVAILABLE, timeout: FIREFOX_CASE_TIMEOUT_MS },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/glyph-path-close-browser-");
        const requests = [];
        const server = createSessionFixtureServer(requests);
        const children = [];
        let bidi;
        try {
            server.listen(0, "127.0.0.1");
            await once(server, "listening");
            const address = server.address();
            assert.notEqual(address, null);
            assert.notEqual(typeof address, "string");
            bidi = await startBidiFirefox(profile, children);
            const tab = await bidi.command("browsingContext.create", {
                type: "tab",
            });
            await bidi.command("browsingContext.navigate", {
                context: tab.context,
                url: `http://127.0.0.1:${address.port}/`
                    + `#session=${REFRESH_SECRET}`,
                wait: "complete",
            });
            await waitForSessionStatus(bidi, tab.context, "Session ready");
            const baseline = [...requests];
            const response = await bidi.command("script.evaluate", {
                awaitPromise: true,
                expression: String.raw`(async () => {
                    const find = (id) => document.getElementById(id);
                    const nfd = "o\u0301";
                    const add = (glyph, coords, color = "#243647") => {
                        find("glyph-character").value = glyph;
                        find("glyph-character").dispatchEvent(
                            new Event("input", { bubbles: true }));
                        for (const [axis, value] of Object.entries({
                            x1: coords[0], y1: coords[1],
                            x2: coords[2], y2: coords[3], width: 250,
                        })) find("glyph-" + axis).value = String(value);
                        find("glyph-color").value = color;
                        find("glyph-add").click();
                    };
                    const choose = (index) => {
                        find("glyph-selected").value = String(index);
                        find("glyph-selected").dispatchEvent(
                            new Event("change", { bubbles: true }));
                    };
                    const initiallyDisabled = find("glyph-path-close").disabled;
                    add(nfd, [2000, 2000, 2200, 2300], "#334455");
                    add("ó", [600, 700, 800, 900]);
                    add(nfd, [400, 500, 1400, 500]);
                    add("∞", [900, 1000, 1200, 1400]);
                    add(nfd, [1400, 500, 1400, 1500]);
                    add(nfd, [1400, 1500, 600, 1500]);
                    choose(0);
                    const accentDisabled = find("glyph-path-close").disabled;
                    choose(4);
                    const canClose = !find("glyph-path-close").disabled;
                    const cannotRotateOpen = find("glyph-path-start").disabled;
                    const before = find("glyph-paths").textContent;
                    find("glyph-path-close").click();
                    const after = {
                        status: find("glyph-status").textContent,
                        selected: find("glyph-selected").value,
                        closedDisabled: find("glyph-path-close").disabled,
                        canRotate: !find("glyph-path-start").disabled,
                        paths: find("glyph-paths").textContent,
                    };
                    find("glyph-character").value = "ó";
                    find("glyph-character").dispatchEvent(
                        new Event("input", { bubbles: true }));
                    choose(1);
                    const otherDisabled = find("glyph-path-close").disabled;
                    let bytes = null;
                    let download = null;
                    const create = URL.createObjectURL;
                    const click = HTMLAnchorElement.prototype.click;
                    try {
                        URL.createObjectURL = (blob) => {
                            bytes = blob;
                            return create.call(URL, blob);
                        };
                        HTMLAnchorElement.prototype.click = function () {
                            download = this.download;
                        };
                        find("glyph-save").click();
                        return JSON.stringify({
                            initiallyDisabled, accentDisabled, canClose,
                            cannotRotateOpen, before, after, otherDisabled,
                            download, media: bytes?.type,
                            tsv: await bytes?.text(),
                        });
                    } finally {
                        URL.createObjectURL = create;
                        HTMLAnchorElement.prototype.click = click;
                    }
                })()`,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.equal(response.type, "success");
            assert.equal(response.result.type, "string");
            const actual = JSON.parse(response.result.value);
            assert.equal(actual.initiallyDisabled, true);
            assert.equal(actual.accentDisabled, true);
            assert.equal(actual.canClose, true);
            assert.equal(actual.cannotRotateOpen, true);
            assert.match(actual.before,
                /4 directed strokes, 2 candidate pen paths/);
            assert.match(actual.after.status,
                /Closed selected pen path with a short bridge; unsaved/);
            assert.equal(actual.after.selected, "6");
            assert.equal(actual.after.closedDisabled, true);
            assert.equal(actual.after.canRotate, true);
            assert.match(actual.after.paths,
                /5 directed strokes, 2 candidate pen paths/);
            assert.match(actual.after.paths,
                /Path 2: strokes 2–5, \(400, 500\) → \(400, 500\)/);
            assert.match(actual.after.paths, /\[closed loop\]/);
            assert.equal(actual.otherDisabled, true);
            assert.equal(actual.download, "my-glyph-strokes.tsv");
            assert.equal(actual.media, "text/tab-separated-values");
            assert.equal(actual.tsv, [
                "atrament.glyph-strokes.v1",
                "o\u0301\t2000\t2000\t2200\t2300\t250\t#334455",
                "ó\t600\t700\t800\t900\t250\t#243647",
                "o\u0301\t400\t500\t1400\t500\t250\t#243647",
                "∞\t900\t1000\t1200\t1400\t250\t#243647",
                "o\u0301\t1400\t500\t1400\t1500\t250\t#243647",
                "o\u0301\t1400\t1500\t600\t1500\t250\t#243647",
                "o\u0301\t600\t1500\t400\t500\t250\t#243647",
                "",
            ].join("\n"));
            assert.deepEqual(requests, baseline,
                "closing a glyph path and TSV download stay browser-local");
        } finally {
            bidi?.close();
            await closeFixtureServer(server);
            for (const child of children.reverse()) await stopChild(child);
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);


test(
    "Firefox italic a guide exports exact UTF-8 TSV without backend edits",
    { skip: !FIREFOX_AVAILABLE, timeout: FIREFOX_CASE_TIMEOUT_MS },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/italic-a-browser-");
        const requests = [];
        const server = createSessionFixtureServer(requests);
        const children = [];
        let bidi;
        try {
            server.listen(0, "127.0.0.1");
            await once(server, "listening");
            const address = server.address();
            assert.notEqual(address, null);
            assert.notEqual(typeof address, "string");
            bidi = await startBidiFirefox(profile, children);
            const tab = await bidi.command("browsingContext.create", {
                type: "tab",
            });
            await bidi.command("browsingContext.navigate", {
                context: tab.context,
                url: `http://127.0.0.1:${address.port}/`
                    + `#session=${REFRESH_SECRET}`,
                wait: "complete",
            });
            await waitForSessionStatus(bidi, tab.context, "Session ready");
            const baseline = [...requests];
            const response = await bidi.command("script.evaluate", {
                awaitPromise: true,
                expression: String.raw`(async () => {
                    const find = (id) => document.getElementById(id);
                    const change = (id, value) => {
                        const input = find(id);
                        input.value = value;
                        input.dispatchEvent(new Event("input", {
                            bubbles: true,
                        }));
                    };
                    const nfd = "a\u0301";
                    const initiallyDisabled =
                        find("glyph-italic-angle").disabled;
                    change("glyph-character", nfd);
                    for (const [axis, value] of Object.entries({
                        x1: 300, y1: 400, x2: 900, y2: 1200,
                        width: 250,
                    })) change("glyph-" + axis, String(value));
                    find("glyph-add").click();
                    change("glyph-character", "á");
                    change("glyph-form", "á-two-italic");
                    change("glyph-circle-radius", "700");
                    const italicEnabled =
                        !find("glyph-italic-angle").disabled;
                    const guide = {
                        circle: find("glyph-ideal-circle")
                            .getAttribute("visibility"),
                        arch: find("glyph-ideal-arch")
                            .getAttribute("visibility"),
                        accent: find("glyph-ideal-accent")
                            .getAttribute("visibility"),
                        shear: find("glyph-ideal-geometry")
                            .getAttribute("transform"),
                    };
                    find("glyph-construct").click();
                    const accepted = find("glyph-status").textContent;
                    const blobs = [];
                    const names = [];
                    const create = URL.createObjectURL;
                    const click = HTMLAnchorElement.prototype.click;
                    try {
                        URL.createObjectURL = (blob) => {
                            blobs.push(blob);
                            return create.call(URL, blob);
                        };
                        HTMLAnchorElement.prototype.click = function () {
                            names.push(this.download);
                        };
                        find("glyph-save").click();
                        change("glyph-italic-angle", "21");
                        const refusedGuide =
                            find("glyph-ideal-circle")
                                .getAttribute("visibility");
                        find("glyph-construct").click();
                        const refused = find("glyph-status").textContent;
                        find("glyph-save").click();
                        change("glyph-italic-angle", "0");
                        const zeroGuide = find("glyph-ideal-geometry")
                            .getAttribute("transform");
                        find("glyph-construct").click();
                        const zeroStatus =
                            find("glyph-status").textContent;
                        find("glyph-save").click();
                        change("glyph-form", "o");
                        const regular = {
                            disabled: find("glyph-italic-angle").disabled,
                            transform: find("glyph-ideal-geometry")
                                .getAttribute("transform"),
                            circle: find("glyph-ideal-circle")
                                .getAttribute("visibility"),
                        };
                        return JSON.stringify({
                            initiallyDisabled, italicEnabled, guide,
                            accepted, refusedGuide, refused,
                            zeroGuide, zeroStatus, regular,
                            names, media: blobs.map((blob) => blob.type),
                            files: await Promise.all(blobs.map((blob) =>
                                blob.text())),
                        });
                    } finally {
                        URL.createObjectURL = create;
                        HTMLAnchorElement.prototype.click = click;
                    }
                })()`,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.equal(response.type, "success");
            assert.equal(response.result.type, "string");
            const result = JSON.parse(response.result.value);
            assert.equal(result.initiallyDisabled, true);
            assert.equal(result.italicEnabled, true);
            assert.deepEqual([result.guide.circle, result.guide.arch,
                result.guide.accent], ["visible", "visible", "visible"]);
            assert.match(result.guide.shear,
                /^matrix\(1 0 -0\.2125.+ 1 .+ 0\)$/);
            assert.match(result.accepted,
                /Constructed 82 sampled segments for á/);
            assert.equal(result.refusedGuide, "hidden");
            assert.match(result.refused, /Italic angle must be 0..20/);
            assert.equal(result.zeroGuide, "matrix(1 0 0 1 0 0)");
            assert.match(result.zeroStatus,
                /Constructed 82 sampled segments for á/);
            assert.deepEqual(result.regular, {
                disabled: true, transform: "", circle: "visible",
            });
            assert.deepEqual(result.names,
                Array(3).fill("my-glyph-strokes.tsv"));
            assert.deepEqual(result.media,
                Array(3).fill("text/tab-separated-values"));
            const independent = [{
                grapheme: "a\u0301", x1: 300, y1: 400,
                x2: 900, y2: 1200, width: 250, color: "#243647",
            }];
            const geometry = [1550, 2450, 700, 250, "#243647"];
            const italic = geometricStarter("á", ...geometry,
                { dx: 0, lift: 0, italicDegrees: 12 },
                "two-storey-italic");
            const regular = geometricStarter("á", ...geometry,
                undefined, "two-storey");
            assert.equal(italic.length, 82);
            const accepted = encodeGlyphLibrary([...independent, ...italic]);
            const zero = encodeGlyphLibrary([...independent, ...regular]);
            assert.equal(result.files[0], accepted);
            assert.equal(result.files[1], accepted,
                "invalid slant cannot overwrite accepted Unicode ink");
            assert.equal(result.files[2], zero,
                "zero-degree italic retains exact regular geometry");
            assert.notEqual(accepted, zero);
            assert.deepEqual(requests, baseline,
                "italic geometry and TSV downloads remain browser-local");
        } finally {
            bidi?.close();
            await closeFixtureServer(server);
            for (const child of children.reverse()) await stopChild(child);
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox slants authored Unicode without changing sibling glyph ink",
    { skip: !FIREFOX_AVAILABLE, timeout: FIREFOX_CASE_TIMEOUT_MS },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/authored-slant-browser-");
        const requests = [];
        const server = createSessionFixtureServer(requests);
        const children = [];
        let bidi;
        try {
            server.listen(0, "127.0.0.1");
            await once(server, "listening");
            const address = server.address();
            assert.notEqual(address, null);
            assert.notEqual(typeof address, "string");
            bidi = await startBidiFirefox(profile, children);
            const tab = await bidi.command("browsingContext.create", {
                type: "tab",
            });
            await bidi.command("browsingContext.navigate", {
                context: tab.context,
                url: `http://127.0.0.1:${address.port}/`
                    + `#session=${REFRESH_SECRET}`,
                wait: "complete",
            });
            await waitForSessionStatus(bidi, tab.context, "Session ready");
            const baseline = [...requests];
            const response = await bidi.command("script.evaluate", {
                awaitPromise: true,
                expression: String.raw`(async () => {
                    const get = (id) => document.getElementById("glyph-" + id);
                    const change = (id, value) => {
                        get(id).value = String(value);
                        get(id).dispatchEvent(new Event("input", {
                            bubbles: true,
                        }));
                    };
                    const add = (glyph, coordinates) => {
                        change("character", glyph);
                        for (const [key, value] of Object.entries({
                            x1: coordinates[0], y1: coordinates[1],
                            x2: coordinates[2], y2: coordinates[3],
                            width: 250,
                        })) change(key, value);
                        get("add").click();
                    };
                    const nfd = "n\u0303";
                    const initiallyDisabled = get("slant-authored").disabled;
                    add(nfd, [450, 1000, 850, 1600]);
                    add("ñ", [300, 350, 500, 550]);
                    change("character", nfd);
                    const enabled = !get("slant-authored").disabled;
                    const preview = get("slant-preview");
                    const guideX1 = preview.querySelector("line")
                        ?.getAttribute("x1");
                    const guideCount = preview.childElementCount;
                    const guideStatus = get("slant-preview-status")
                        .textContent;
                    get("slant-authored").click();
                    const accepted = get("status").textContent;
                    const undoEnabled = !get("slant-undo").disabled;
                    const blobs = [];
                    const originalUrl = URL.createObjectURL;
                    const originalClick = HTMLAnchorElement.prototype.click;
                    try {
                        URL.createObjectURL = (blob) => {
                            blobs.push(blob);
                            return originalUrl.call(URL, blob);
                        };
                        HTMLAnchorElement.prototype.click = function () {};
                        get("save").click();
                        get("slant-undo").click();
                        const undone = get("status").textContent;
                        const redoEnabled = !get("slant-redo").disabled;
                        get("save").click();
                        get("slant-redo").click();
                        const redone = get("status").textContent;
                        get("save").click();
                        change("authored-angle", 21);
                        const invalidGuide = preview.childElementCount;
                        const invalidGuideStatus =
                            get("slant-preview-status").textContent;
                        get("slant-authored").click();
                        const refused = get("status").textContent;
                        get("save").click();
                        change("character", "á");
                        const missing = get("slant-authored").disabled;
                        return JSON.stringify({
                            initiallyDisabled, enabled, accepted, refused,
                            undoEnabled, undone, redoEnabled, redone,
                            guideX1, guideCount, guideStatus,
                            invalidGuide, invalidGuideStatus,
                            missing, blobs: await Promise.all(blobs.map((b) =>
                                b.text())),
                        });
                    } finally {
                        URL.createObjectURL = originalUrl;
                        HTMLAnchorElement.prototype.click = originalClick;
                    }
                })()`,
                resultOwnership: "none",
                target: { context: tab.context },
            });
            assert.equal(response.type, "success");
            assert.equal(response.result.type, "string");
            const actual = JSON.parse(response.result.value);
            assert.equal(actual.initiallyDisabled, true);
            assert.equal(actual.enabled, true);
            assert.equal(actual.missing, true);
            assert.equal(actual.guideCount, 1);
            assert.equal(actual.guideX1, "992");
            assert.match(actual.guideStatus, /preview only, draft unchanged/);
            assert.equal(actual.invalidGuide, 0);
            assert.match(actual.invalidGuideStatus, /Italic angle/);
            assert.equal(actual.undoEnabled, true);
            assert.equal(actual.redoEnabled, true);
            assert.match(actual.undone, /Undid last glyph slant; unsaved/);
            assert.match(actual.redone, /Redid last glyph slant; unsaved/);
            assert.match(actual.accepted, /Slanted all authored ñ ink/);
            assert.match(actual.refused, /Italic angle must be 0..20/);
            const nfd = "n\u0303";
            const source = [
                { grapheme: nfd, x1: 450, y1: 1000,
                    x2: 850, y2: 1600, width: 250,
                    color: "#243647" },
                { grapheme: "ñ", x1: 300, y1: 350,
                    x2: 500, y2: 550, width: 250,
                    color: "#243647" },
            ];
            const expected = encodeGlyphLibrary(
                italicizeGlyphStrokes(source, nfd, 12, 3550),
            );
            assert.deepEqual(actual.blobs, [expected,
                encodeGlyphLibrary(source), expected, expected],
                "Firefox undo/redo restores exact accepted TSV bytes");
            assert.deepEqual(requests, baseline,
                "authored italic transformation stays in the browser");
        } finally {
            bidi?.close();
            await closeFixtureServer(server);
            for (const child of children.reverse()) await stopChild(child);
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox height preview and exact Unicode TSV require explicit action",
    { skip: !FIREFOX_AVAILABLE, timeout: FIREFOX_CASE_TIMEOUT_MS },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/authored-height-browser-");
        const requests = [];
        const server = createSessionFixtureServer(requests);
        const children = [];
        let bidi;
        try {
            server.listen(0, "127.0.0.1");
            await once(server, "listening");
            const address = server.address();
            assert.notEqual(address, null);
            assert.notEqual(typeof address, "string");
            bidi = await startBidiFirefox(profile, children);
            const tab = await bidi.command("browsingContext.create", {
                type: "tab",
            });
            await bidi.command("browsingContext.navigate", {
                context: tab.context,
                url: `http://127.0.0.1:${address.port}/`
                    + `#session=${REFRESH_SECRET}`,
                wait: "complete",
            });
            await waitForSessionStatus(bidi, tab.context, "Session ready");
            const baseline = [...requests];
            const response = await bidi.command("script.evaluate", {
                awaitPromise: true,
                expression: String.raw`(async () => {
                    const get = (id) => document.getElementById("glyph-" + id);
                    const change = (id, value) => {
                        get(id).value = String(value);
                        get(id).dispatchEvent(new Event("input", {
                            bubbles: true,
                        }));
                    };
                    const add = (glyph, coords) => {
                        change("character", glyph);
                        for (const [key, value] of Object.entries({
                            x1: coords[0], y1: coords[1],
                            x2: coords[2], y2: coords[3], width: 250,
                        })) change(key, value);
                        get("add").click();
                    };
                    const nfd = "a\u0301";
                    const initiallyDisabled = get("scale-height").disabled;
                    add(nfd, [450, 1000, 850, 1600]);
                    add("á", [600, 1200, 900, 1550]);
                    add(nfd, [850, 1600, 1300, 2200]);
                    add("∞", [500, 900, 700, 1100]);
                    change("character", nfd);
                    change("height-baseline", 2250);
                    const enabled = !get("scale-height").disabled;
                    const guide = get("height-preview");
                    const guideCount = guide.childElementCount;
                    const guideY1 = guide.querySelector("line")
                        ?.getAttribute("y1");
                    const guideStatus = get("height-preview-status")
                        .textContent;
                    const blobs = [];
                    const names = [];
                    const create = URL.createObjectURL;
                    const click = HTMLAnchorElement.prototype.click;
                    try {
                        URL.createObjectURL = (blob) => {
                            blobs.push(blob);
                            return create.call(URL, blob);
                        };
                        HTMLAnchorElement.prototype.click = function () {
                            names.push(this.download);
                        };
                        get("save").click();
                        change("height-percent", 151);
                        const invalidGuide = guide.childElementCount;
                        const invalidStatus = get("height-preview-status")
                            .textContent;
                        get("scale-height").click();
                        const refused = get("status").textContent;
                        get("save").click();
                        change("height-percent", 110);
                        get("scale-height").click();
                        const accepted = get("status").textContent;
                        const afterPreview = guide.querySelector("line")
                            ?.getAttribute("y1");
                        get("save").click();
                        change("height-percent", 150);
                        change("height-baseline", 4500);
                        const clippedGuide = guide.childElementCount;
                        const clippedStatus = get("height-preview-status")
                            .textContent;
                        get("scale-height").click();
                        const clipped = get("status").textContent;
                        get("save").click();
                        change("character", "ñ");
                        const missingDisabled = get("scale-height").disabled;
                        return JSON.stringify({
                            initiallyDisabled, enabled, guideCount,
                            guideY1, guideStatus, invalidGuide, invalidStatus,
                            refused, accepted, afterPreview, clippedGuide,
                            clippedStatus, clipped, missingDisabled,
                            names, types: blobs.map((blob) => blob.type),
                            files: await Promise.all(blobs.map((blob) =>
                                blob.text())),
                        });
                    } finally {
                        URL.createObjectURL = create;
                        HTMLAnchorElement.prototype.click = click;
                    }
                })()`,
                resultOwnership: "none",
                target: { context: tab.context },
            }, FIREFOX_BIDI_READY_TIMEOUT_MS);
            assert.equal(response.type, "success");
            assert.equal(response.result.type, "string");
            const result = JSON.parse(response.result.value);
            assert.equal(result.initiallyDisabled, true);
            assert.equal(result.enabled, true);
            assert.equal(result.missingDisabled, true);
            assert.equal(result.guideCount, 2);
            assert.equal(result.guideY1, "875");
            assert.match(result.guideStatus,
                /preview only, draft unchanged/);
            assert.equal(result.invalidGuide, 0);
            assert.match(result.invalidStatus, /Height percent/);
            assert.match(result.refused, /Height percent/);
            assert.match(result.accepted,
                /Resized all authored á height; unsaved/);
            assert.equal(result.afterPreview, "738",
                "guide recomputes on the accepted unsaved draft");
            assert.equal(result.clippedGuide, 0);
            assert.match(result.clippedStatus, /Resized ink and pen clip/);
            assert.match(result.clipped, /Resized ink and pen clip/);
            assert.deepEqual(result.names,
                Array(4).fill("my-glyph-strokes.tsv"));
            assert.deepEqual(result.types,
                Array(4).fill("text/tab-separated-values"));
            const nfd = "a\u0301";
            const source = [
                { grapheme: nfd, x1: 450, y1: 1000,
                    x2: 850, y2: 1600, width: 250, color: "#243647" },
                { grapheme: "á", x1: 600, y1: 1200,
                    x2: 900, y2: 1550, width: 250, color: "#243647" },
                { grapheme: nfd, x1: 850, y1: 1600,
                    x2: 1300, y2: 2200, width: 250, color: "#243647" },
                { grapheme: "∞", x1: 500, y1: 900,
                    x2: 700, y2: 1100, width: 250, color: "#243647" },
            ];
            const original = encodeGlyphLibrary(source);
            const transformed = encodeGlyphLibrary(
                scaleGlyphHeight(source, nfd, 110, 2250));
            assert.deepEqual(result.files,
                [original, original, transformed, transformed],
                "invalid browser edits cannot overwrite accepted ink");
            assert.deepEqual(requests, baseline,
                "height editing, previews and downloads stay browser-local");
        } finally {
            bidi?.close();
            await closeFixtureServer(server);
            for (const child of children.reverse()) await stopChild(child);
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox width preview and exact Unicode TSV require explicit action",
    { skip: !FIREFOX_AVAILABLE, timeout: FIREFOX_CASE_TIMEOUT_MS },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/authored-width-browser-");
        const requests = [];
        const server = createSessionFixtureServer(requests);
        const children = [];
        let bidi;
        try {
            server.listen(0, "127.0.0.1");
            await once(server, "listening");
            const address = server.address();
            assert.notEqual(address, null);
            assert.notEqual(typeof address, "string");
            bidi = await startBidiFirefox(profile, children);
            const tab = await bidi.command("browsingContext.create", {
                type: "tab",
            });
            await bidi.command("browsingContext.navigate", {
                context: tab.context,
                url: `http://127.0.0.1:${address.port}/`
                    + `#session=${REFRESH_SECRET}`,
                wait: "complete",
            });
            await waitForSessionStatus(bidi, tab.context, "Session ready");
            const baseline = [...requests];
            const response = await bidi.command("script.evaluate", {
                awaitPromise: true,
                expression: String.raw`(async () => {
                    const get = (id) => document.getElementById("glyph-" + id);
                    const change = (id, value) => {
                        get(id).value = String(value);
                        get(id).dispatchEvent(new Event("input", {
                            bubbles: true,
                        }));
                    };
                    const add = (glyph, coords) => {
                        change("character", glyph);
                        for (const [key, value] of Object.entries({
                            x1: coords[0], y1: coords[1],
                            x2: coords[2], y2: coords[3], width: 250,
                        })) change(key, value);
                        get("add").click();
                    };
                    const nfd = "a\u0301";
                    const initiallyDisabled = get("scale-width").disabled;
                    add(nfd, [450, 1000, 850, 1600]);
                    add("á", [600, 1200, 900, 1550]);
                    add(nfd, [850, 1600, 1300, 2200]);
                    add("∞", [500, 900, 700, 1100]);
                    change("character", nfd);
                    change("width-anchor", 1750);
                    const enabled = !get("scale-width").disabled;
                    const guide = get("width-preview");
                    const guideCount = guide.childElementCount;
                    const guideX1 = guide.querySelector("line")
                        ?.getAttribute("x1");
                    const guideStatus = get("width-preview-status")
                        .textContent;
                    const blobs = [];
                    const names = [];
                    const create = URL.createObjectURL;
                    const click = HTMLAnchorElement.prototype.click;
                    try {
                        URL.createObjectURL = (blob) => {
                            blobs.push(blob);
                            return create.call(URL, blob);
                        };
                        HTMLAnchorElement.prototype.click = function () {
                            names.push(this.download);
                        };
                        get("save").click();
                        change("width-percent", 151);
                        const invalidGuide = guide.childElementCount;
                        const invalidStatus = get("width-preview-status")
                            .textContent;
                        get("scale-width").click();
                        const refused = get("status").textContent;
                        get("save").click();
                        change("width-percent", 110);
                        get("scale-width").click();
                        const accepted = get("status").textContent;
                        const afterPreview = guide.querySelector("line")
                            ?.getAttribute("x1");
                        get("save").click();
                        change("width-percent", 150);
                        change("width-anchor", 3500);
                        const clippedGuide = guide.childElementCount;
                        const clippedStatus = get("width-preview-status")
                            .textContent;
                        get("scale-width").click();
                        const clipped = get("status").textContent;
                        get("save").click();
                        change("character", "ñ");
                        const missingDisabled = get("scale-width").disabled;
                        return JSON.stringify({
                            initiallyDisabled, enabled, guideCount,
                            guideX1, guideStatus, invalidGuide, invalidStatus,
                            refused, accepted, afterPreview, clippedGuide,
                            clippedStatus, clipped, missingDisabled,
                            names, types: blobs.map((blob) => blob.type),
                            files: await Promise.all(blobs.map((blob) =>
                                blob.text())),
                        });
                    } finally {
                        URL.createObjectURL = create;
                        HTMLAnchorElement.prototype.click = click;
                    }
                })()`,
                resultOwnership: "none",
                target: { context: tab.context },
            }, FIREFOX_BIDI_READY_TIMEOUT_MS);
            assert.equal(response.type, "success");
            assert.equal(response.result.type, "string");
            const result = JSON.parse(response.result.value);
            assert.equal(result.initiallyDisabled, true);
            assert.equal(result.enabled, true);
            assert.equal(result.missingDisabled, true);
            assert.equal(result.guideCount, 2);
            assert.equal(result.guideX1, "320");
            assert.match(result.guideStatus,
                /preview only, draft unchanged/);
            assert.equal(result.invalidGuide, 0);
            assert.match(result.invalidStatus, /Width percent/);
            assert.match(result.refused, /Width percent/);
            assert.match(result.accepted,
                /Resized all authored á width; unsaved/);
            assert.equal(result.afterPreview, "177",
                "guide recomputes on the accepted unsaved draft");
            assert.equal(result.clippedGuide, 0);
            assert.match(result.clippedStatus, /Resized ink and pen clip/);
            assert.match(result.clipped, /Resized ink and pen clip/);
            assert.deepEqual(result.names,
                Array(4).fill("my-glyph-strokes.tsv"));
            assert.deepEqual(result.types,
                Array(4).fill("text/tab-separated-values"));
            const nfd = "a\u0301";
            const source = [
                { grapheme: nfd, x1: 450, y1: 1000,
                    x2: 850, y2: 1600, width: 250, color: "#243647" },
                { grapheme: "á", x1: 600, y1: 1200,
                    x2: 900, y2: 1550, width: 250, color: "#243647" },
                { grapheme: nfd, x1: 850, y1: 1600,
                    x2: 1300, y2: 2200, width: 250, color: "#243647" },
                { grapheme: "∞", x1: 500, y1: 900,
                    x2: 700, y2: 1100, width: 250, color: "#243647" },
            ];
            const original = encodeGlyphLibrary(source);
            const transformed = encodeGlyphLibrary(
                scaleGlyphWidth(source, nfd, 110, 1750));
            assert.deepEqual(result.files,
                [original, original, transformed, transformed],
                "invalid browser edits cannot overwrite accepted ink");
            assert.deepEqual(requests, baseline,
                "width editing, previews and downloads stay browser-local");
        } finally {
            bidi?.close();
            await closeFixtureServer(server);
            for (const child of children.reverse()) await stopChild(child);
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);


test(
    "Firefox ink thickness preview and explicit exact Unicode TSV",
    { skip: !FIREFOX_AVAILABLE, timeout: FIREFOX_CASE_TIMEOUT_MS },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/authored-ink-browser-");
        const requests = [];
        const server = createSessionFixtureServer(requests);
        const children = [];
        let bidi;
        try {
            server.listen(0, "127.0.0.1");
            await once(server, "listening");
            const address = server.address();
            assert.notEqual(address, null);
            assert.notEqual(typeof address, "string");
            bidi = await startBidiFirefox(profile, children);
            const tab = await bidi.command("browsingContext.create", {
                type: "tab",
            });
            await bidi.command("browsingContext.navigate", {
                context: tab.context,
                url: `http://127.0.0.1:${address.port}/`
                    + `#session=${REFRESH_SECRET}`,
                wait: "complete",
            });
            await waitForSessionStatus(bidi, tab.context, "Session ready");
            const baseline = [...requests];
            const response = await bidi.command("script.evaluate", {
                awaitPromise: true,
                expression: String.raw`(async () => {
                    const get = (id) => document.getElementById("glyph-" + id);
                    const change = (id, value) => {
                        get(id).value = String(value);
                        get(id).dispatchEvent(new Event("input", {
                            bubbles: true,
                        }));
                    };
                    const add = (glyph, coords, width) => {
                        change("character", glyph);
                        for (const [key, value] of Object.entries({
                            x1: coords[0], y1: coords[1],
                            x2: coords[2], y2: coords[3], width,
                        })) change(key, value);
                        get("add").click();
                    };
                    const nfd = "o\u0301";
                    const initiallyDisabled = get("scale-ink").disabled;
                    add(nfd, [175, 800, 850, 1200], 251);
                    add("ó", [600, 1200, 900, 1550], 250);
                    add(nfd, [850, 1200, 1100, 1700], 251);
                    add("∞", [500, 900, 700, 1100], 250);
                    change("character", nfd);
                    const enabled = !get("scale-ink").disabled;
                    const guide = get("ink-preview");
                    const guideCount = guide.childElementCount;
                    const guideWidth = guide.querySelector("line")
                        ?.getAttribute("stroke-width");
                    const guideStatus = get("ink-preview-status").textContent;
                    const blobs = [];
                    const names = [];
                    const create = URL.createObjectURL;
                    const click = HTMLAnchorElement.prototype.click;
                    try {
                        URL.createObjectURL = (blob) => {
                            blobs.push(blob);
                            return create.call(URL, blob);
                        };
                        HTMLAnchorElement.prototype.click = function () {
                            names.push(this.download);
                        };
                        get("save").click();
                        change("ink-percent", 151);
                        const invalidGuide = guide.childElementCount;
                        const invalidStatus = get("ink-preview-status")
                            .textContent;
                        get("scale-ink").click();
                        const refused = get("status").textContent;
                        get("save").click();
                        change("ink-percent", 110);
                        get("scale-ink").click();
                        const accepted = get("status").textContent;
                        const afterPreview = guide.querySelector("line")
                            ?.getAttribute("stroke-width");
                        get("save").click();
                        change("ink-percent", 100);
                        get("scale-ink").click();
                        const undoAfterIdentity =
                            !get("slant-undo").disabled;
                        change("ink-percent", 110);
                        const undoReady = !get("slant-undo").disabled;
                        const undoLabel = get("slant-undo").textContent;
                        get("slant-undo").click();
                        const undoStatus = get("status").textContent;
                        const undoneWidth = guide.querySelector("line")
                            ?.getAttribute("stroke-width");
                        get("save").click();
                        change("ink-percent", 100);
                        get("scale-ink").click();
                        const redoAfterIdentity =
                            !get("slant-redo").disabled;
                        change("ink-percent", 110);
                        const redoReady = !get("slant-redo").disabled;
                        const redoLabel = get("slant-redo").textContent;
                        get("slant-redo").click();
                        const redoStatus = get("status").textContent;
                        get("save").click();
                        change("ink-percent", 150);
                        const clippedGuide = guide.childElementCount;
                        const clippedStatus = get("ink-preview-status")
                            .textContent;
                        get("scale-ink").click();
                        const clipped = get("status").textContent;
                        get("save").click();
                        change("character", "ñ");
                        const missingDisabled = get("scale-ink").disabled;
                        const genericLabel = get("slant-undo").textContent;
                        return JSON.stringify({
                            initiallyDisabled, enabled, guideCount,
                            guideWidth, guideStatus, invalidGuide,
                            invalidStatus,
                            refused, accepted, afterPreview,
                            undoReady, undoAfterIdentity, undoLabel,
                            undoStatus, undoneWidth,
                            redoReady, redoAfterIdentity, redoLabel,
                            redoStatus, clippedGuide, genericLabel,
                            clippedStatus, clipped, missingDisabled,
                            names, types: blobs.map((blob) => blob.type),
                            files: await Promise.all(blobs.map((blob) =>
                                blob.text())),
                        });
                    } finally {
                        URL.createObjectURL = create;
                        HTMLAnchorElement.prototype.click = click;
                    }
                })()`,
                resultOwnership: "none",
                target: { context: tab.context },
            }, FIREFOX_BIDI_READY_TIMEOUT_MS);
            assert.equal(response.type, "success");
            assert.equal(response.result.type, "string");
            const result = JSON.parse(response.result.value);
            assert.equal(result.initiallyDisabled, true);
            assert.equal(result.enabled, true);
            assert.equal(result.missingDisabled, true);
            assert.equal(result.guideCount, 2);
            assert.equal(result.guideWidth, "276");
            assert.match(result.guideStatus,
                /preview only, draft unchanged/);
            assert.equal(result.invalidGuide, 0);
            assert.match(result.invalidStatus, /Ink percent/);
            assert.match(result.refused, /Ink percent/);
            assert.match(result.accepted,
                /Resized all authored ó pen thickness; unsaved/);
            assert.equal(result.afterPreview, "304",
                "read-only guide recomputes after a valid ink edit");
            assert.equal(result.undoReady, true);
            assert.equal(result.undoAfterIdentity, true);
            assert.equal(result.undoLabel, "Undo last glyph pen thickness");
            assert.equal(result.redoReady, true);
            assert.equal(result.redoAfterIdentity, true);
            assert.equal(result.redoLabel, "Redo last glyph pen thickness");
            assert.equal(result.genericLabel, "Undo last glyph transform");
            assert.match(result.undoStatus,
                /Undid last glyph pen thickness; unsaved/);
            assert.equal(result.undoneWidth, "276");
            assert.match(result.redoStatus,
                /Redid last glyph pen thickness; unsaved/);
            assert.equal(result.clippedGuide, 0);
            assert.match(result.clippedStatus, /Scaled ink and pen clip/);
            assert.match(result.clipped, /Scaled ink and pen clip/);
            assert.deepEqual(result.names,
                Array(6).fill("my-glyph-strokes.tsv"));
            assert.deepEqual(result.types,
                Array(6).fill("text/tab-separated-values"));
            const nfd = "o\u0301";
            const source = [
                { grapheme: nfd, x1: 175, y1: 800,
                    x2: 850, y2: 1200, width: 251, color: "#243647" },
                { grapheme: "ó", x1: 600, y1: 1200,
                    x2: 900, y2: 1550, width: 250, color: "#243647" },
                { grapheme: nfd, x1: 850, y1: 1200,
                    x2: 1100, y2: 1700, width: 251, color: "#243647" },
                { grapheme: "∞", x1: 500, y1: 900,
                    x2: 700, y2: 1100, width: 250, color: "#243647" },
            ];
            const original = encodeGlyphLibrary(source);
            const transformed = encodeGlyphLibrary(
                scaleGlyphInk(source, nfd, 110));
            assert.deepEqual(result.files,
                [original, original, transformed,
                    original, transformed, transformed],
                "refused browser edits cannot overwrite accepted ink");
            assert.deepEqual(requests, baseline,
                "ink edits and downloads make no extra backend requests");
        } finally {
            bidi?.close();
            await closeFixtureServer(server);
            for (const child of children.reverse()) await stopChild(child);
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);


test(
    "Firefox whole-glyph positioning previews and exports exact Unicode ink",
    { skip: !FIREFOX_AVAILABLE, timeout: FIREFOX_CASE_TIMEOUT_MS },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/authored-move-browser-");
        const requests = [];
        const server = createSessionFixtureServer(requests);
        const children = [];
        let bidi;
        try {
            server.listen(0, "127.0.0.1");
            await once(server, "listening");
            const address = server.address();
            assert.notEqual(address, null);
            assert.notEqual(typeof address, "string");
            bidi = await startBidiFirefox(profile, children);
            const tab = await bidi.command("browsingContext.create", {
                type: "tab",
            });
            await bidi.command("browsingContext.navigate", {
                context: tab.context,
                url: `http://127.0.0.1:${address.port}/`
                    + `#session=${REFRESH_SECRET}`,
                wait: "complete",
            });
            await waitForSessionStatus(bidi, tab.context, "Session ready");
            const baseline = [...requests];
            const response = await bidi.command("script.evaluate", {
                awaitPromise: true,
                expression: String.raw`(async () => {
                    const get = (id) => document.getElementById("glyph-" + id);
                    const change = (id, value) => {
                        get(id).value = String(value);
                        get(id).dispatchEvent(new Event("input", {
                            bubbles: true,
                        }));
                    };
                    const add = (glyph, coords, width) => {
                        change("character", glyph);
                        for (const [key, value] of Object.entries({
                            x1: coords[0], y1: coords[1],
                            x2: coords[2], y2: coords[3], width,
                        })) change(key, value);
                        get("add").click();
                    };
                    const nfd = "o\u0301";
                    const initiallyDisabled = get("move-all").disabled;
                    add(nfd, [450, 1000, 850, 1600], 251);
                    add("ó", [300, 1000, 500, 1500], 250);
                    add(nfd, [850, 1600, 1300, 2200], 251);
                    add("∞", [500, 900, 700, 1100], 250);
                    change("character", nfd);
                    const enabled = !get("move-all").disabled;
                    const guide = get("move-preview");
                    change("move-x", -200);
                    change("move-y", 175);
                    const guideCount = guide.childElementCount;
                    const guideX1 = guide.querySelector("line")
                        ?.getAttribute("x1");
                    const guideY1 = guide.querySelector("line")
                        ?.getAttribute("y1");
                    const guideStatus = get("move-preview-status")
                        .textContent;
                    const blobs = [];
                    const names = [];
                    const create = URL.createObjectURL;
                    const click = HTMLAnchorElement.prototype.click;
                    try {
                        URL.createObjectURL = (blob) => {
                            blobs.push(blob);
                            return create.call(URL, blob);
                        };
                        HTMLAnchorElement.prototype.click = function () {
                            names.push(this.download);
                        };
                        get("save").click();
                        change("move-x", 1.5);
                        const invalidGuide = guide.childElementCount;
                        const invalidStatus = get("move-preview-status")
                            .textContent;
                        get("move-all").click();
                        const refused = get("status").textContent;
                        get("save").click();
                        change("move-x", -200);
                        get("move-all").click();
                        const accepted = get("status").textContent;
                        const undoReady = !get("slant-undo").disabled;
                        const undoLabel = get("slant-undo").textContent;
                        get("save").click();
                        change("move-x", 0);
                        change("move-y", 0);
                        get("move-all").click();
                        const undoAfterIdentity =
                            !get("slant-undo").disabled;
                        get("slant-undo").click();
                        const undone = get("status").textContent;
                        const redoReady = !get("slant-redo").disabled;
                        const redoLabel = get("slant-redo").textContent;
                        get("save").click();
                        get("slant-redo").click();
                        const redone = get("status").textContent;
                        get("save").click();
                        change("move-x", -326);
                        const clippedGuide = guide.childElementCount;
                        const clippedStatus = get("move-preview-status")
                            .textContent;
                        get("move-all").click();
                        const clipped = get("status").textContent;
                        get("save").click();
                        change("character", "ñ");
                        const missingDisabled = get("move-all").disabled;
                        return JSON.stringify({
                            initiallyDisabled, enabled, guideCount,
                            guideX1, guideY1, guideStatus,
                            invalidGuide, invalidStatus, refused,
                            accepted, undoReady, undoLabel,
                            undoAfterIdentity, undone, redoReady,
                            redoLabel, redone, clippedGuide,
                            clippedStatus, clipped, missingDisabled,
                            names, types: blobs.map((blob) => blob.type),
                            files: await Promise.all(blobs.map((blob) =>
                                blob.text())),
                        });
                    } finally {
                        URL.createObjectURL = create;
                        HTMLAnchorElement.prototype.click = click;
                    }
                })()`,
                resultOwnership: "none",
                target: { context: tab.context },
            }, FIREFOX_BIDI_READY_TIMEOUT_MS);
            assert.equal(response.type, "success");
            assert.equal(response.result.type, "string");
            const result = JSON.parse(response.result.value);
            assert.equal(result.initiallyDisabled, true);
            assert.equal(result.enabled, true);
            assert.equal(result.missingDisabled, true);
            assert.equal(result.guideCount, 2);
            assert.equal(result.guideX1, "250");
            assert.equal(result.guideY1, "1175");
            assert.match(result.guideStatus,
                /preview only, draft unchanged/);
            assert.equal(result.invalidGuide, 0);
            assert.match(result.invalidStatus, /offsets require signed whole/);
            assert.match(result.refused, /offsets require signed whole/);
            assert.match(result.accepted,
                /Moved all authored ó strokes; unsaved/);
            assert.equal(result.undoReady, true);
            assert.equal(result.undoAfterIdentity, true);
            assert.equal(result.undoLabel, "Undo last glyph move");
            assert.equal(result.redoReady, true);
            assert.equal(result.redoLabel, "Redo last glyph move");
            assert.match(result.undone, /Undid last glyph move; unsaved/);
            assert.match(result.redone, /Redid last glyph move; unsaved/);
            assert.equal(result.clippedGuide, 0);
            assert.match(result.clippedStatus, /clips glyph cell/);
            assert.match(result.clipped, /clips glyph cell/);
            assert.deepEqual(result.names,
                Array(6).fill("my-glyph-strokes.tsv"));
            assert.deepEqual(result.types,
                Array(6).fill("text/tab-separated-values"));
            const nfd = "o\u0301";
            const original = [
                { grapheme: nfd, x1: 450, y1: 1000,
                    x2: 850, y2: 1600, width: 251, color: "#243647" },
                { grapheme: "ó", x1: 300, y1: 1000,
                    x2: 500, y2: 1500, width: 250, color: "#243647" },
                { grapheme: nfd, x1: 850, y1: 1600,
                    x2: 1300, y2: 2200, width: 251, color: "#243647" },
                { grapheme: "∞", x1: 500, y1: 900,
                    x2: 700, y2: 1100, width: 250, color: "#243647" },
            ];
            const initial = encodeGlyphLibrary(original);
            const moved = encodeGlyphLibrary(
                translateGlyphStrokes(original, nfd, -200, 175));
            assert.deepEqual(result.files,
                [initial, initial, moved, initial, moved, moved],
                "real browser downloads track exact accepted/undone ink");
            assert.deepEqual(requests, baseline,
                "browser-only positioning never calls backend again");
        } finally {
            bidi?.close();
            await closeFixtureServer(server);
            for (const child of children.reverse()) await stopChild(child);
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox whole-glyph rotation previews and recovers real SVG ink",
    { skip: !FIREFOX_AVAILABLE, timeout: FIREFOX_CASE_TIMEOUT_MS },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/authored-rotate-browser-");
        const requests = [];
        const server = createSessionFixtureServer(requests);
        const children = [];
        let bidi;
        try {
            server.listen(0, "127.0.0.1");
            await once(server, "listening");
            const address = server.address();
            assert.notEqual(address, null);
            assert.notEqual(typeof address, "string");
            bidi = await startBidiFirefox(profile, children);
            const tab = await bidi.command("browsingContext.create", {
                type: "tab",
            });
            await bidi.command("browsingContext.navigate", {
                context: tab.context,
                url: `http://127.0.0.1:${address.port}/`
                    + `#session=${REFRESH_SECRET}`,
                wait: "complete",
            });
            await waitForSessionStatus(bidi, tab.context, "Session ready");
            const baseline = [...requests];
            const response = await bidi.command("script.evaluate", {
                awaitPromise: false,
                expression: String.raw`(() => {
                    const get = (id) => document.getElementById("glyph-" + id);
                    const set = (id, value) => {
                        get(id).value = String(value);
                        get(id).dispatchEvent(new Event("input", {
                            bubbles: true,
                        }));
                    };
                    const initialDisabled = get("rotate-all").disabled;
                    set("character", "o\u0301");
                    for (const [key, value] of Object.entries({
                        x1: 450, y1: 1000, x2: 850, y2: 1600,
                        width: 251,
                    })) set(key, value);
                    get("add").click();
                    const enabled = !get("rotate-all").disabled;
                    set("rotate-angle", 10);
                    const guide = get("rotate-preview");
                    const proposedX = guide.querySelector("line")
                        ?.getAttribute("x1");
                    const proposedY = guide.querySelector("line")
                        ?.getAttribute("y1");
                    const guideCount = guide.childElementCount;
                    const guideStatus = get("rotate-preview-status")
                        .textContent;
                    set("rotate-angle", 21);
                    const invalidGuide = guide.childElementCount;
                    get("rotate-all").click();
                    const invalid = get("status").textContent;
                    set("rotate-angle", 20);
                    set("rotate-pivot-x", 0);
                    set("rotate-pivot-y", 0);
                    const clippedGuide = guide.childElementCount;
                    get("rotate-all").click();
                    const clipped = get("status").textContent;
                    set("rotate-pivot-x", 1750);
                    set("rotate-pivot-y", 2250);
                    set("rotate-angle", 10);
                    get("rotate-all").click();
                    const appliedX = get("preview").querySelector("line")
                        ?.getAttribute("x1");
                    const accepted = get("status").textContent;
                    const undoLabel = get("slant-undo").textContent;
                    get("slant-undo").click();
                    const undoneX = get("preview").querySelector("line")
                        ?.getAttribute("x1");
                    const redoLabel = get("slant-redo").textContent;
                    get("slant-redo").click();
                    const redoneX = get("preview").querySelector("line")
                        ?.getAttribute("x1");
                    set("character", "ñ");
                    const missingDisabled = get("rotate-all").disabled;
                    return JSON.stringify({ initialDisabled, enabled,
                        proposedX, proposedY, guideCount, guideStatus,
                        invalidGuide, invalid, clippedGuide, clipped,
                        appliedX, accepted, undoLabel, undoneX,
                        redoLabel, redoneX, missingDisabled });
                })()`,
                resultOwnership: "none",
                target: { context: tab.context },
            }, FIREFOX_BIDI_READY_TIMEOUT_MS);
            assert.equal(response.type, "success");
            assert.equal(response.result.type, "string");
            const result = JSON.parse(response.result.value);
            assert.equal(result.initialDisabled, true);
            assert.equal(result.enabled, true);
            assert.equal(result.guideCount, 1);
            assert.match(result.guideStatus, /preview only, draft unchanged/);
            assert.equal(result.invalidGuide, 0);
            assert.match(result.invalid, /Rotation requires/);
            assert.equal(result.clippedGuide, 0);
            assert.match(result.clipped, /clips glyph cell/);
            assert.match(result.accepted, /Rotated all authored/);
            assert.equal(result.undoLabel, "Undo last glyph rotation");
            assert.equal(result.redoLabel, "Redo last glyph rotation");
            assert.equal(result.missingDisabled, true);
            assert.equal(result.undoneX, "450");
            assert.equal(result.appliedX, result.redoneX);
            assert.equal(result.proposedX, result.appliedX);
            assert.notEqual(result.proposedX, result.undoneX);
            assert.deepEqual(requests, baseline,
                "local rotation and recovery do not call the Rust backend");
        } finally {
            bidi?.close();
            await closeFixtureServer(server);
            for (const child of children.reverse()) await stopChild(child);
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox downloads exact precedence ink for native PDF and PNG",
    { skip: !FIREFOX_AVAILABLE, timeout: 120_000 },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/precedence-firefox-");
        const requests = [];
        const server = createSessionFixtureServer(requests);
        const children = [];
        let bidi;
        try {
            server.listen(0, "127.0.0.1");
            await once(server, "listening");
            const address = server.address();
            assert.notEqual(address, null);
            assert.notEqual(typeof address, "string");
            bidi = await startBidiFirefox(profile, children);
            const tab = await bidi.command("browsingContext.create", {
                type: "tab",
            });
            await bidi.command("browsingContext.navigate", {
                context: tab.context,
                url: `http://127.0.0.1:${address.port}/`
                    + `#session=${REFRESH_SECRET}`,
                wait: "complete",
            });
            await waitForSessionStatus(bidi, tab.context, "Session ready");
            const baseline = [...requests];
            const response = await bidi.command("script.evaluate", {
                awaitPromise: true,
                expression: String.raw`(async () => {
                    const get = (id) => document.getElementById("glyph-" + id);
                    const change = (id, value) => {
                        get(id).value = value;
                        get(id).dispatchEvent(new Event("input", {
                            bubbles: true,
                        }));
                    };
                    const guides = [];
                    for (const glyph of ["≺", "≻", "≼", "≽"]) {
                        change("form", glyph);
                        change("character", glyph);
                        const guide = get("ideal-rounded");
                        const d = guide.getAttribute("d");
                        const visible = guide.getAttribute("visibility");
                        get("construct").click();
                        guides.push({ glyph, d, visible,
                            selected: get("form").value,
                            status: get("status").textContent });
                    }
                    const blobs = [];
                    const names = [];
                    const url = URL.createObjectURL;
                    const click = HTMLAnchorElement.prototype.click;
                    try {
                        URL.createObjectURL = (blob) => {
                            blobs.push(blob);
                            return url.call(URL, blob);
                        };
                        HTMLAnchorElement.prototype.click = function () {
                            names.push(this.download);
                        };
                        get("save").click();
                        return JSON.stringify({ guides, names,
                            files: await Promise.all(blobs.map((blob) =>
                                blob.text())) });
                    } finally {
                        URL.createObjectURL = url;
                        HTMLAnchorElement.prototype.click = click;
                    }
                })()`,
                resultOwnership: "none",
                target: { context: tab.context },
            }, FIREFOX_BIDI_READY_TIMEOUT_MS);
            assert.equal(response.type, "success", JSON.stringify(response));
            assert.equal(response.result.type, "string");
            const actual = JSON.parse(response.result.value);
            const symbols = ["≺", "≻", "≼", "≽"];
            for (const [index, glyph] of symbols.entries()) {
                const guide = actual.guides[index];
                assert.equal(guide.glyph, glyph);
                assert.equal(guide.selected, glyph);
                assert.equal(guide.visible, "visible");
                assert.equal((guide.d.match(/ Q /g) ?? []).length, 2);
                assert.equal((guide.d.match(/M /g) ?? []).length,
                    index < 2 ? 1 : 2);
                assert.match(guide.status, new RegExp(
                    `Constructed ${index < 2 ? 32 : 33} sampled segments`));
            }
            const ink = symbols.flatMap((glyph) => geometricStarter(
                glyph, 1550, 2450, 1100, 250, "#243647"));
            assert.equal(ink.length, 130);
            const expected = encodeGlyphLibrary(ink);
            assert.deepEqual(actual.names, ["my-glyph-strokes.tsv"]);
            assert.deepEqual(actual.files, [expected]);
            assert.deepEqual(requests, baseline,
                "manual selection and download do not contact backend");
            const library = path.join(profile, "relations.tsv");
            fs.writeFileSync(library, actual.files[0]);
            const build = spawnSync(PINNED_CARGO, [
                "build", "--offline", "--quiet",
                "-p", "atrament_profile_editor_cli",
                "--bin", "atrament-profiles",
            ], { timeout: 90_000, stdio: "pipe",
                env: { ...process.env, CARGO_TARGET_DIR: TARGET_DIR } });
            assert.equal(build.status, 0,
                "native CLI must build offline for real Firefox TSV");
            const cli = path.join(TARGET_DIR, "debug", "atrament-profiles");
            const invoke = (...flags) => spawnSync(cli, flags, {
                timeout: 20_000, maxBuffer: 16 * 1024 * 1024,
            });
            const readback = invoke("glyphs", "--profile", library);
            assert.equal(readback.status, 0, readback.stderr.toString());
            assert.equal(readback.stdout.toString(), expected);
            for (const format of ["pdf", "png"]) {
                const result = invoke("notebook", "--printed-sheet",
                    "true", "--glyph-file", library,
                    "--flow-text", `${symbols.join("")}\n${symbols.join("")}`,
                    "--export-paper", format,
                    ...(format === "png" ? ["--dpi", "72"] : []));
                assert.equal(result.status, 0, result.stderr.toString());
                if (format === "pdf") {
                    assert.match(result.stdout.toString("latin1"),
                        /^%PDF-1\.4/);
                    assert.equal((result.stdout.toString("latin1")
                        .match(/ m\n/g) ?? []).length, 260);
                } else {
                    assert.deepEqual(result.stdout.subarray(0, 8),
                        Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]));
                    assert.equal(result.stdout.readUInt32BE(16), 596);
                    assert.equal(result.stdout.readUInt32BE(20), 842);
                }
            }
        } finally {
            bidi?.close();
            await closeFixtureServer(server);
            for (const child of children.reverse()) await stopChild(child);
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox authors exact calculus and currency ink for native PDF/PNG",
    // A fresh checkout may need one bounded offline Rust compilation.
    { skip: !FIREFOX_AVAILABLE, timeout: 120_000 },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/calculus-firefox-");
        const requests = [];
        const server = createSessionFixtureServer(requests);
        const children = [];
        let bidi;
        try {
            server.listen(0, "127.0.0.1");
            await once(server, "listening");
            const address = server.address();
            assert.notEqual(address, null);
            assert.notEqual(typeof address, "string");
            bidi = await startBidiFirefox(profile, children);
            const tab = await bidi.command("browsingContext.create", {
                type: "tab",
            });
            await bidi.command("browsingContext.navigate", {
                context: tab.context,
                url: `http://127.0.0.1:${address.port}/`
                    + `#session=${REFRESH_SECRET}`,
                wait: "complete",
            });
            await waitForSessionStatus(bidi, tab.context, "Session ready");
            const baseline = [...requests];
            const response = await bidi.command("script.evaluate", {
                awaitPromise: true,
                expression: String.raw`(async () => {
                    const get = (id) => document.getElementById("glyph-" + id);
                    const change = (id, value) => {
                        get(id).value = String(value);
                        get(id).dispatchEvent(new Event("input", {
                            bubbles: true,
                        }));
                    };
                    const before = get("seed-math").disabled;
                    get("seed-math").click();
                    const filled = get("status").textContent;
                    get("seed-math").click();
                    const complete = get("status").textContent;
                    const guides = [];
                    for (const glyph of ["∑", "∫", "∂", "Δ", "≈", "→", "∝",
                        "<", ">", "←", "↔", "↑", "↓", "↕", "∧", "∨",
                        "∩", "∪", "∈", "∉", "⊂", "⊆", "⊃", "⊇",
                        "⊄", "⊈", "⊅", "⊉", "∋", "∌",
                        "∅", "∇", "≡", "≢", "∥", "⊥", "∼", "≅",
                        "∀", "∃", "∄", "∴", "∵", "⊤", "⊢", "⊨",
                        "¬", "⇒", "⇔", "∠", "∟", "⊬", "⊭",
                        "≪", "≫", "≮", "≯",
                        "≰", "≱", "↗", "↘", "↖", "↙", "⇐",
                        "↦", "↤", "∣", "∤", "′", "″", "‴", "⋅", "∗",
                        "∖", "∦", "⌊", "⌋", "⌈", "⌉"]) {
                        change("form", glyph);
                        change("character", glyph);
                        const guide = get("ideal-rounded");
                        const d = guide.getAttribute("d");
                        guides.push({ glyph, d,
                            selected: get("form").value,
                            visible: guide.getAttribute("visibility") });
                    }
                    const currency = [];
                    for (const glyph of ["$", "€"]) {
                        change("form", glyph);
                        change("character", glyph);
                        const rounded = get("ideal-rounded");
                        const arc = get("ideal-c-loop");
                        const guide = { glyph,
                            d: rounded.getAttribute("d"),
                            visible: rounded.getAttribute("visibility"),
                            arc: arc.getAttribute("d"),
                            arcVisible: arc.getAttribute("visibility") };
                        get("construct").click();
                        currency.push({ ...guide,
                            status: get("status").textContent });
                    }
                    const manualMarks = [];
                    for (const glyph of ["⟨", "⟩", "α", "β", "θ", "λ",
                        "μ", "γ", "δ", "σ", "ρ", "τ", "ε", "ω",
                        "κ", "χ", "φ", "ψ", "ζ", "η", "ξ", "ι",
                        "ο", "υ", "ν", "ς", "Γ", "Ω", "Π", "Σ",
                        "Λ", "Θ", "Ξ", "Φ", "Ψ", "Χ", "Β", "Υ",
                        "Α", "Ε", "Ι", "Ο", "Ζ", "Η",
                        "Κ", "Μ", "Ν", "Ρ", "Τ", "∏", "∐", "∮", "∬", "∭",
                        "ℕ", "ℤ", "ℚ", "ℝ", "ℂ", "∛", "∜", "⊕", "⊗", "⊙",
                        "⊖", "⊘", "⋂", "⋃", "⋀", "⋁", "⊓", "⊔",
                        "⊏", "⊐", "⊑", "⊒",
                        "⊞", "⊟", "⊠", "⊡", "⊲", "⊳", "⊴", "⊵",
                        "⋖", "⋗", "⋘", "⋙",
                        "≶", "≷", "⋚", "⋛"]) {
                        change("form", glyph);
                        change("character", glyph);
                        const guide = get("ideal-rounded");
                        const d = guide.getAttribute("d");
                        const visible = guide.getAttribute("visibility");
                        get("construct").click();
                        manualMarks.push({ glyph, d, visible,
                            selected: get("form").value,
                            status: get("status").textContent });
                    }
                    const names = [];
                    const blobs = [];
                    const create = URL.createObjectURL;
                    const click = HTMLAnchorElement.prototype.click;
                    try {
                        URL.createObjectURL = (blob) => {
                            blobs.push(blob);
                            return create.call(URL, blob);
                        };
                        HTMLAnchorElement.prototype.click = function () {
                            names.push(this.download);
                        };
                        get("save").click();
                        change("form", "∫");
                        change("character", "∫");
                        change("circle-x", 150);
                        const hidden = get("ideal-rounded")
                            .getAttribute("visibility");
                        get("construct").click();
                        const refused = get("status").textContent;
                        get("save").click();
                        return JSON.stringify({ before, filled, complete,
                            guides, currency, manualMarks, hidden, refused,
                            names,
                            media: blobs.map((blob) => blob.type),
                            files: await Promise.all(blobs.map((blob) =>
                                blob.text())) });
                    } finally {
                        URL.createObjectURL = create;
                        HTMLAnchorElement.prototype.click = click;
                    }
                })()`,
                resultOwnership: "none",
                target: { context: tab.context },
            }, FIREFOX_BIDI_READY_TIMEOUT_MS);
            assert.equal(response.type, "success", JSON.stringify(response));
            assert.equal(response.result.type, "string");
            const actual = JSON.parse(response.result.value);
            assert.equal(actual.before, false);
            assert.match(actual.filled, /Filled 93 missing math glyphs/);
            assert.match(actual.complete, /All 93 math glyphs/);
            for (const [index, count, lifts] of [
                [0, 0, 1], [1, 3, 1], [2, 4, 1], [3, 0, 1], [4, 4, 2],
                [5, 0, 2], [6, 4, 1],
                [7, 0, 1], [8, 0, 1], [9, 0, 2], [10, 0, 3],
                [11, 0, 2], [12, 0, 2], [13, 0, 3],
                [14, 0, 1], [15, 0, 1], [16, 2, 1], [17, 2, 1],
                [18, 2, 2], [19, 2, 3], [20, 4, 1], [21, 4, 2],
                [22, 4, 1], [23, 4, 2],
                [24, 4, 2], [25, 4, 3], [26, 4, 2], [27, 4, 3],
                [28, 2, 2], [29, 2, 3],
                [30, 4, 2], [31, 0, 1], [32, 0, 3], [33, 0, 4],
                [34, 0, 2], [35, 0, 2], [36, 2, 1], [37, 2, 3],
                [38, 0, 2], [39, 0, 4], [40, 0, 5], [41, 0, 3],
                [42, 0, 3], [43, 0, 2], [44, 0, 2], [45, 0, 3],
                [46, 0, 1], [47, 0, 3], [48, 0, 4],
                [49, 0, 1], [50, 0, 1], [51, 0, 3], [52, 0, 4],
                [53, 0, 2], [54, 0, 2], [55, 0, 2], [56, 0, 2],
                [57, 0, 3], [58, 0, 3], [59, 0, 2], [60, 0, 2],
                [61, 0, 2], [62, 0, 2], [63, 0, 3],
                [64, 0, 3], [65, 0, 3], [66, 0, 1], [67, 0, 2],
                [68, 0, 1], [69, 0, 2], [70, 0, 3], [71, 0, 1],
                [72, 0, 3], [73, 0, 1], [74, 0, 3],
                [75, 0, 1], [76, 0, 1], [77, 0, 1], [78, 0, 1],
            ]) {
                const guide = actual.guides[index];
                assert.equal(guide.selected, guide.glyph,
                    "real HTML option decodes the exact math grapheme");
                assert.equal(guide.visible, "visible");
                assert.equal((guide.d.match(/ Q /g) ?? []).length, count);
                assert.equal((guide.d.match(/M /g) ?? []).length, lifts);
            }
            for (const [index, glyph, count] of [
                [0, "$", 65], [1, "€", 38],
            ]) {
                const guide = actual.currency[index];
                assert.equal(guide.glyph, glyph);
                assert.equal(guide.visible, "visible");
                assert.equal((guide.d.match(/ Q /g) ?? []).length,
                    glyph === "$" ? 4 : 0);
                assert.equal((guide.d.match(/ L /g) ?? []).length,
                    glyph === "$" ? 1 : 2);
                assert.equal(guide.arcVisible,
                    glyph === "€" ? "visible" : "hidden");
                if (glyph === "€") assert.match(guide.arc, / A /);
                assert.match(guide.status,
                    new RegExp(`Constructed ${count} sampled segments`));
            }
            for (const guide of actual.manualMarks) {
                assert.ok([
                    "⟨", "⟩", "α", "β", "θ", "λ", "μ", "γ",
                    "δ", "σ", "ρ", "τ", "ε", "ω", "κ", "χ",
                    "φ", "ψ", "ζ", "η", "ξ", "ι",
                    "ο", "υ", "ν", "ς", "Γ", "Ω", "Π", "Σ",
                    "Λ", "Θ", "Ξ", "Φ", "Ψ", "Χ", "Β", "Υ",
                    "Α", "Ε", "Ι", "Ο", "Ζ", "Η",
                    "Κ", "Μ", "Ν", "Ρ", "Τ", "∏", "∐", "∮", "∬", "∭",
                    "ℕ", "ℤ", "ℚ", "ℝ", "ℂ", "∛", "∜", "⊕", "⊗", "⊙",
                    "⊖", "⊘", "⋂", "⋃", "⋀", "⋁", "⊓", "⊔",
                    "⊏", "⊐", "⊑", "⊒",
                    "⊞", "⊟", "⊠", "⊡",
                    "⊲", "⊳", "⊴", "⊵",
                    "⋖", "⋗", "⋘", "⋙",
                    "≶", "≷", "⋚", "⋛",
                ].includes(guide.glyph));
                assert.equal(guide.selected, guide.glyph);
                assert.equal(guide.visible, "visible");
                const bracket = ["⟨", "⟩"].includes(guide.glyph);
                const lambda = guide.glyph === "λ";
                const mu = guide.glyph === "μ";
                const gamma = guide.glyph === "γ";
                const tau = guide.glyph === "τ";
                const epsilon = guide.glyph === "ε";
                const kappa = guide.glyph === "κ";
                const chi = guide.glyph === "χ";
                const psi = guide.glyph === "ψ";
                const zeta = guide.glyph === "ζ";
                const eta = guide.glyph === "η";
                const xi = guide.glyph === "ξ";
                const iota = guide.glyph === "ι";
                const omicron = guide.glyph === "ο";
                const upsilon = guide.glyph === "υ";
                const nu = guide.glyph === "ν";
                const finalSigma = guide.glyph === "ς";
                const upperGamma = guide.glyph === "Γ";
                const upperOmega = guide.glyph === "Ω";
                const upperPi = guide.glyph === "Π";
                const upperSigma = guide.glyph === "Σ";
                const upperLambda = guide.glyph === "Λ";
                const upperXi = guide.glyph === "Ξ";
                const upperPsi = guide.glyph === "Ψ";
                const upperChi = guide.glyph === "Χ";
                const upperUpsilon = guide.glyph === "Υ";
                const upperAlpha = guide.glyph === "Α";
                const upperEpsilon = guide.glyph === "Ε";
                const upperIota = guide.glyph === "Ι";
                const upperOmicron = guide.glyph === "Ο";
                const upperZeta = guide.glyph === "Ζ";
                const upperEta = guide.glyph === "Η";
                const upperKappa = guide.glyph === "Κ";
                const upperMu = guide.glyph === "Μ";
                const upperNu = guide.glyph === "Ν";
                const upperRho = guide.glyph === "Ρ";
                const upperTau = guide.glyph === "Τ";
                const naryProduct = ["∏", "∐"].includes(guide.glyph);
                const straightSets = ["ℕ", "ℤ"].includes(guide.glyph);
                const setQ = guide.glyph === "ℚ";
                const setR = guide.glyph === "ℝ";
                const setC = guide.glyph === "ℂ";
                const indexCube = guide.glyph === "∛";
                const indexFourth = guide.glyph === "∜";
                const circledSingle = ["⊙", "⊖", "⊘"]
                    .includes(guide.glyph);
                const circled = circledSingle || ["⊕", "⊗"]
                    .includes(guide.glyph);
                const bigSet = ["⋂", "⋃"].includes(guide.glyph);
                const bigLogic = ["⋀", "⋁"].includes(guide.glyph);
                const squareLattice = ["⊓", "⊔"]
                    .includes(guide.glyph);
                const squareSubset = ["⊏", "⊐", "⊑", "⊒"]
                    .includes(guide.glyph);
                const squareEqual = ["⊑", "⊒"].includes(guide.glyph);
                const squareOperator = ["⊞", "⊟", "⊠", "⊡"]
                    .includes(guide.glyph);
                const squareDouble = ["⊞", "⊠"].includes(guide.glyph);
                const normalSubgroup = ["⊲", "⊳", "⊴", "⊵"]
                    .includes(guide.glyph);
                const normalEqual = ["⊴", "⊵"].includes(guide.glyph);
                const dottedComparison = ["⋖", "⋗"]
                    .includes(guide.glyph);
                const tripleComparison = ["⋘", "⋙"]
                    .includes(guide.glyph);
                const stackedComparison = ["≶", "≷", "⋚", "⋛"]
                    .includes(guide.glyph);
                const stackedEqual = ["⋚", "⋛"].includes(guide.glyph);
                const count = stackedComparison ? (stackedEqual ? 5 : 4)
                    : dottedComparison ? 3 : tripleComparison ? 6
                    : normalSubgroup ? (normalEqual ? 4 : 3)
                    : squareOperator ? (squareDouble ? 6 : 5)
                    : squareSubset ? (squareEqual ? 4 : 3)
                    : squareLattice ? 3 : bigLogic ? 2 : bigSet ? 34
                    : circled ? (circledSingle ? 65 : 66)
                    : indexCube ? 36 : indexFourth ? 7
                    : straightSets ? 4 : setQ ? 66
                    : setR ? 35 : setC ? 65
                    : guide.glyph === "∮" ? 112
                    : guide.glyph === "∬" ? 96
                    : guide.glyph === "∭" ? 144
                    : naryProduct ? 5 : upperIota ? 1
                    : bracket || lambda || upperGamma
                    || upperLambda || upperChi || upperTau ? 2
                    : tau || upperPi || upperXi || upperUpsilon
                        || upperAlpha || upperZeta || upperEta
                        || upperKappa || upperNu ? 3
                        : upperSigma || upperEpsilon || upperMu ? 4
                        : epsilon || kappa || psi || eta || upperPsi
                            || upperRho ? 33
                        : chi || iota || upsilon || nu ? 32
                            : zeta || finalSigma ? 48
                                : upperOmega ? 66
                                : xi || omicron || upperOmicron ? 64
                            : mu ? 34
                            : gamma ? 33 : 65;
                assert.equal((guide.d.match(/ Q /g) ?? []).length,
                    stackedComparison || dottedComparison || tripleComparison
                    || normalSubgroup || squareOperator || squareSubset
                    || squareLattice || bigLogic ? 0
                    : bigSet ? 2 : circled ? 4
                    : indexCube ? 2 : indexFourth ? 0
                    : straightSets ? 0 : setQ || setC ? 4 : setR ? 2
                    : guide.glyph === "∮" ? 7
                        : guide.glyph === "∬" ? 6
                            : guide.glyph === "∭" ? 9
                    : bracket || lambda || tau || upperGamma
                        || upperPi || upperSigma || upperLambda || upperXi
                        || upperChi || upperUpsilon || upperAlpha
                        || upperEpsilon || upperIota || upperZeta
                        || upperEta || upperKappa || upperMu
                        || upperNu || upperTau || naryProduct ? 0
                        : mu || gamma || epsilon || kappa || chi
                            || psi || eta || iota || upsilon || nu
                            || upperPsi || upperRho ? 2
                            : zeta || finalSigma ? 3 : 4);
                assert.equal((guide.d.match(/M /g) ?? []).length,
                    stackedComparison ? (stackedEqual ? 3 : 2)
                    : dottedComparison ? 2 : tripleComparison ? 3
                    : normalSubgroup ? (normalEqual ? 2 : 1)
                    : squareOperator ? (squareDouble ? 3 : 2)
                    : squareSubset ? (squareEqual ? 2 : 1)
                    : squareLattice || bigLogic || bigSet ? 1
                    : circled ? (circledSingle ? 2 : 3)
                    : indexCube ? 2 : indexFourth ? 3
                    : setQ ? 3 : straightSets || setR || setC ? 2
                    : guide.glyph === "∮" || guide.glyph === "∬" ? 2
                        : guide.glyph === "∭" ? 3
                    : bracket || mu || zeta || xi || iota
                        || upperGamma || upperPi || upperSigma || upperLambda
                        || omicron || upsilon || nu || finalSigma
                        || upperIota || upperOmicron || upperZeta
                        || upperMu || upperNu ? 1
                            : naryProduct ? 5
                            : upperOmega || upperXi || upperEta ? 3 : 2);
                assert.match(guide.status,
                    new RegExp(`Constructed ${count} sampled segments`));
            }
            assert.equal(actual.hidden, "hidden");
            assert.match(actual.refused, /fit inside glyph cell/);
            assert.deepEqual(actual.names,
                Array(2).fill("my-glyph-strokes.tsv"));
            assert.deepEqual(actual.media,
                Array(2).fill("text/tab-separated-values"));
            const symbols = ["+", "-", "−", "±", "∓", "=", "×", "÷",
                "√", "π", "∞", "≠", "<", ">", "≤", "≥", "∧", "∨",
                "∩", "∪", "∈", "∉", "∋", "∌", "⊂", "⊆",
                "⊃", "⊇", "⊄", "⊈", "⊅", "⊉", "∑", "∫",
                "∂", "Δ", "≈", "→", "←", "↔", "↑", "↓", "↕", "∝",
                "∅", "∇", "≡", "≢", "∥", "⊥", "∼", "≅",
                "∀", "∃", "∄", "∴", "∵", "⊤", "⊢", "⊨",
                "∠", "∟", "⊬", "⊭", "¬", "⇒", "⇔",
                "≪", "≫", "≮", "≯", "≰", "≱", "↗", "↘",
                "↖", "↙", "⇐", "↦", "↤", "∣", "∤",
                "′", "″", "‴", "⋅", "∗", "∖", "∦",
                "⌊", "⌋", "⌈", "⌉"];
            const ink = [
                ...symbols, "$", "€", "⟨", "⟩",
                "α", "β", "θ", "λ", "μ", "γ", "δ", "σ", "ρ", "τ",
                "ε", "ω", "κ", "χ", "φ", "ψ", "ζ", "η",
                "ξ", "ι", "ο", "υ", "ν", "ς", "Γ", "Ω", "Π", "Σ",
                "Λ", "Θ", "Ξ", "Φ", "Ψ", "Χ", "Β", "Υ",
                "Α", "Ε", "Ι", "Ο", "Ζ", "Η",
                "Κ", "Μ", "Ν", "Ρ", "Τ", "∏", "∐", "∮", "∬", "∭",
                "ℕ", "ℤ", "ℚ", "ℝ", "ℂ", "∛", "∜", "⊕", "⊗", "⊙",
                "⊖", "⊘", "⋂", "⋃", "⋀", "⋁", "⊓", "⊔",
                    "⊏", "⊐", "⊑", "⊒",
                    "⊞", "⊟", "⊠", "⊡",
                    "⊲", "⊳", "⊴", "⊵",
                    "⋖", "⋗", "⋘", "⋙",
                    "≶", "≷", "⋚", "⋛",
            ].flatMap((glyph) => geometricStarter(
                glyph, 1550, 2450, 1100, 250, "#243647"));
            assert.equal(ink.length, 4046);
            const expected = encodeGlyphLibrary(ink);
            assert.deepEqual(actual.files, [expected, expected],
                "real browser fill and refusal preserve exact pen order");
            assert.deepEqual(requests, baseline,
                "unsaved geometry and downloads never access backend");

            const library = path.join(profile, "math.tsv");
            fs.writeFileSync(library, actual.files[0]);
            const build = spawnSync(PINNED_CARGO, [
                "build", "--offline", "--quiet",
                "-p", "atrament_profile_editor_cli",
                "--bin", "atrament-profiles",
            ], {
                timeout: 90_000, stdio: "pipe",
                env: { ...process.env, CARGO_TARGET_DIR: TARGET_DIR },
            });
            assert.equal(build.status, 0,
                "native CLI must build offline for Firefox TSV replay");
            const cli = path.join(TARGET_DIR, "debug", "atrament-profiles");
            const invoke = (...flags) => spawnSync(cli, flags, {
                timeout: 20_000,
                maxBuffer: 16 * 1024 * 1024,
            });
            const reopen = invoke("glyphs", "--profile", library);
            assert.equal(reopen.status, 0, reopen.stderr.toString());
            assert.equal(reopen.stdout.toString(), expected,
                "Rust reopens the actual Firefox TSV without rewriting ink");
            for (const format of ["pdf", "png"]) {
                const rendered = invoke("notebook", "--printed-sheet",
                    "true", "--glyph-file", library, "--flow-text",
                    "∑∫∂Δ≈$€∓→∝<>←↔↑↓↕∧∨∩∪∈∉⊂⊆⊃⊇⊄⊈⊅⊉"
                        + "∋∌∅∇≡≢∥⊥∼≅∀∃∄∴∵⊤⊢⊨∠∟⊬⊭¬⇒⇔"
                        + "≪≫≮≯≰≱↗↘↖↙⇐↦↤∣∤′″‴⋅∗∖∦⌊⌋⌈⌉⟨⟩αβθλμγδσρτεωκχφψζηξι"
                        + "ουνςΓΩΠΣΛΘΞΦΨΧΒΥΑΕΙΟΖΗΚΜΝΡΤ∏∐∮∬∭ℕℤℚℝℂ"
                        + "∛∜⊕⊗⊙⊖⊘⋂⋃⋀⋁⊓⊔⊏⊐⊑⊒⊞⊟⊠⊡⊲⊳⊴⊵⋖⋗⋘⋙≶≷⋚⋛",
                    "--export-paper", format,
                    ...(format === "png" ? ["--dpi", "72"] : []));
                assert.equal(rendered.status, 0,
                    rendered.stderr.toString());
                assert.ok(rendered.stdout.length > 1000);
                if (format === "pdf") {
                    const vector = rendered.stdout.toString("latin1");
                    assert.match(vector, /^%PDF-1\.4/);
                    assert.equal((vector.match(/ m\n/g) ?? []).length, 3951,
                        "calculus, currency, arrows and comparisons ink");
                } else {
                    const bytes = rendered.stdout;
                    assert.deepEqual(bytes.subarray(0, 8),
                        Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]));
                    assert.equal(bytes.readUInt32BE(16), 596);
                    assert.equal(bytes.readUInt32BE(20), 842);
                    assert.equal(bytes[25], 6, "transparent RGBA page");
                    const chunks = [];
                    for (let offset = 8; offset + 12 <= bytes.length;) {
                        const length = bytes.readUInt32BE(offset);
                        if (bytes.toString("ascii", offset + 4,
                            offset + 8) === "IDAT") {
                            chunks.push(bytes.subarray(offset + 8,
                                offset + 8 + length));
                        }
                        offset += length + 12;
                    }
                    assert.ok(chunks.length);
                    assert.ok(inflateSync(Buffer.concat(chunks)).includes(
                        Buffer.from([36, 54, 71, 255])),
                    "native PNG contains real authored Unicode ink");
                }
            }
            assert.equal(fs.readFileSync(library, "utf8"), expected);
        } finally {
            bidi?.close();
            await closeFixtureServer(server);
            for (const child of children.reverse()) await stopChild(child);
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox authors Spanish accent starters for exact native PDF/PNG",
    { skip: !FIREFOX_AVAILABLE, timeout: 120_000 },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/accents-firefox-");
        const requests = [];
        const server = createSessionFixtureServer(requests);
        const children = [];
        let bidi;
        try {
            server.listen(0, "127.0.0.1");
            await once(server, "listening");
            const address = server.address();
            assert.notEqual(address, null);
            assert.notEqual(typeof address, "string");
            bidi = await startBidiFirefox(profile, children);
            const tab = await bidi.command("browsingContext.create", {
                type: "tab",
            });
            await bidi.command("browsingContext.navigate", {
                context: tab.context,
                url: `http://127.0.0.1:${address.port}/`
                    + `#session=${REFRESH_SECRET}`,
                wait: "complete",
            });
            await waitForSessionStatus(bidi, tab.context, "Session ready");
            const baseline = [...requests];
            const response = await bidi.command("script.evaluate", {
                awaitPromise: true,
                expression: String.raw`(async () => {
                    const get = (id) => document.getElementById("glyph-" + id);
                    const change = (id, value) => {
                        get(id).value = String(value);
                        get(id).dispatchEvent(new Event("input", {
                            bubbles: true,
                        }));
                    };
                    const guides = [];
                    for (const glyph of ["é", "ú", "ü", "ñ"]) {
                        change("form", glyph);
                        change("character", glyph);
                        const preview = (id) => ({
                            visible: get(id).getAttribute("visibility"),
                            d: get(id).getAttribute("d"),
                        });
                        const guide = {
                            glyph, round: preview("ideal-rounded"),
                            open: preview("ideal-c-loop"),
                            acute: get("ideal-accent")
                                .getAttribute("visibility"),
                            marks: preview("ideal-diacritic"),
                            bar: get("ideal-e-bar")
                                .getAttribute("visibility"),
                        };
                        get("construct").click();
                        guides.push({ ...guide,
                            status: get("status").textContent });
                    }
                    const names = [];
                    const blobs = [];
                    const create = URL.createObjectURL;
                    const click = HTMLAnchorElement.prototype.click;
                    try {
                        URL.createObjectURL = (blob) => {
                            blobs.push(blob);
                            return create.call(URL, blob);
                        };
                        HTMLAnchorElement.prototype.click = function () {
                            names.push(this.download);
                        };
                        get("save").click();
                        change("circle-y", 1350);
                        const hidden = get("ideal-rounded")
                            .getAttribute("visibility");
                        const hiddenAccent = get("ideal-diacritic")
                            .getAttribute("visibility");
                        get("construct").click();
                        const refused = get("status").textContent;
                        get("save").click();
                        return JSON.stringify({ guides, hidden, hiddenAccent,
                            refused, names, files: await Promise.all(
                                blobs.map((blob) => blob.text())) });
                    } finally {
                        URL.createObjectURL = create;
                        HTMLAnchorElement.prototype.click = click;
                    }
                })()`,
                resultOwnership: "none",
                target: { context: tab.context },
            }, FIREFOX_BIDI_READY_TIMEOUT_MS);
            assert.equal(response.type, "success");
            assert.equal(response.result.type, "string");
            const actual = JSON.parse(response.result.value);
            for (const [index, glyph, count] of [
                [0, "é", 38], [1, "ú", 35],
                [2, "ü", 36], [3, "ñ", 37],
            ]) {
                const guide = actual.guides[index];
                assert.equal(guide.glyph, glyph);
                assert.equal(guide.round.visible,
                    glyph === "é" ? "hidden" : "visible");
                assert.equal(guide.open.visible,
                    glyph === "é" ? "visible" : "hidden");
                assert.equal(guide.bar, glyph === "é" ? "visible" : "hidden");
                if (glyph === "é") assert.match(guide.open.d, / A /);
                else assert.equal((guide.round.d.match(/ Q /g) ?? []).length,
                    2, "Spanish rounded base retains ideal quadratic arcs");
                assert.equal(guide.acute,
                    ["é", "ú"].includes(glyph) ? "visible" : "hidden");
                assert.equal(guide.marks.visible,
                    ["ü", "ñ"].includes(glyph) ? "visible" : "hidden");
                assert.equal((guide.marks.d.match(/ L /g) ?? []).length,
                    glyph === "ü" ? 2 : glyph === "ñ" ? 3 : 0);
                assert.match(guide.status,
                    new RegExp(`Constructed ${count} sampled segments`));
            }
            assert.equal(actual.hidden, "hidden");
            assert.equal(actual.hiddenAccent, "hidden");
            assert.match(actual.refused, /Accent and pen/);
            assert.deepEqual(actual.names,
                Array(2).fill("my-glyph-strokes.tsv"));
            const glyphs = ["é", "ú", "ü", "ñ"];
            const rows = glyphs.flatMap((glyph) => geometricStarter(
                glyph, 1550, 2450, 1100, 250, "#243647"));
            assert.equal(rows.length, 146);
            const expected = encodeGlyphLibrary(rows);
            assert.deepEqual(actual.files, [expected, expected],
                "real Firefox edit refusal retains exact Unicode pen order");
            assert.deepEqual(requests, baseline,
                "browser drafting and downloads use no backend calls");

            const library = path.join(profile, "accented.tsv");
            fs.writeFileSync(library, actual.files[0]);
            const build = spawnSync(PINNED_CARGO, [
                "build", "--offline", "--quiet",
                "-p", "atrament_profile_editor_cli",
                "--bin", "atrament-profiles",
            ], { timeout: 90_000, stdio: "pipe",
                env: { ...process.env, CARGO_TARGET_DIR: TARGET_DIR } });
            assert.equal(build.status, 0,
                "native CLI must build offline for actual Firefox ink");
            const cli = path.join(TARGET_DIR, "debug", "atrament-profiles");
            const invoke = (...flags) => spawnSync(cli, flags, {
                timeout: 20_000, maxBuffer: 16 * 1024 * 1024,
            });
            const reopen = invoke("glyphs", "--profile", library);
            assert.equal(reopen.status, 0, reopen.stderr.toString());
            assert.equal(reopen.stdout.toString(), expected);
            const flags = ["notebook", "--printed-sheet", "true",
                "--glyph-file", library, "--flow-text", "éúüñ"];
            const output = path.join(profile, "accented.pdf");
            const pdf = invoke(...flags, "--export-paper", "pdf",
                "--output", output);
            assert.equal(pdf.status, 0, pdf.stderr.toString());
            const approved = fs.readFileSync(output);
            assert.match(approved.toString("latin1"), /^%PDF-1\.4/);
            assert.equal((approved.toString("latin1").match(/ m\n/g) ?? [])
                .length, 146, "one Firefox-authored row emits 146 ink lines");
            const png = invoke(...flags, "--export-paper", "png",
                "--dpi", "72");
            assert.equal(png.status, 0, png.stderr.toString());
            const bytes = png.stdout;
            assert.deepEqual(bytes.subarray(0, 8),
                Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]));
            assert.equal(bytes.readUInt32BE(16), 596);
            assert.equal(bytes.readUInt32BE(20), 842);
            assert.equal(bytes[25], 6, "native RGBA remains transparent");
            const chunks = [];
            for (let offset = 8; offset + 12 <= bytes.length;) {
                const length = bytes.readUInt32BE(offset);
                if (bytes.toString("ascii", offset + 4, offset + 8)
                    === "IDAT") {
                    chunks.push(bytes.subarray(offset + 8,
                        offset + 8 + length));
                }
                offset += length + 12;
            }
            assert.ok(inflateSync(Buffer.concat(chunks)).includes(
                Buffer.from([36, 54, 71, 255])),
            "real native PNG includes explicitly authored Spanish ink");
            const refused = invoke("notebook", "--printed-sheet", "true",
                "--glyph-file", library, "--flow-text", "éúüñß",
                "--export-paper", "pdf", "--output", output, "--replace");
            assert.notEqual(refused.status, 0);
            assert.match(refused.stderr.toString(), /missing authored glyph/i);
            assert.deepEqual(fs.readFileSync(output), approved,
                "native missing glyph must not overwrite reviewed PDF");
            assert.equal(fs.readFileSync(library, "utf8"), expected);
        } finally {
            bidi?.close();
            await closeFixtureServer(server);
            for (const child of children.reverse()) await stopChild(child);
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox selects exact NFD accent forms for Rust PDF and PNG ink",
    { skip: !FIREFOX_AVAILABLE, timeout: 120_000 },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/nfd-firefox-");
        const requests = [];
        const server = createSessionFixtureServer(requests);
        const children = [];
        let bidi;
        try {
            server.listen(0, "127.0.0.1");
            await once(server, "listening");
            const address = server.address();
            assert.notEqual(address, null);
            assert.notEqual(typeof address, "string");
            bidi = await startBidiFirefox(profile, children);
            const tab = await bidi.command("browsingContext.create", {
                type: "tab",
            });
            await bidi.command("browsingContext.navigate", {
                context: tab.context,
                url: `http://127.0.0.1:${address.port}/`
                    + `#session=${REFRESH_SECRET}`,
                wait: "complete",
            });
            await waitForSessionStatus(bidi, tab.context, "Session ready");
            const baseline = [...requests];
            const response = await bidi.command("script.evaluate", {
                awaitPromise: true,
                expression: String.raw`(async () => {
                    const get = (id) => document.getElementById("glyph-" + id);
                    const change = (id, value) => {
                        get(id).value = String(value);
                        get(id).dispatchEvent(new Event("input", {
                            bubbles: true,
                        }));
                    };
                    const nfc = ["á", "é", "í", "ó", "ú", "ñ", "ü",
                        "Á", "É", "Í", "Ó", "Ú", "Ñ", "Ü"];
                    const nfd = nfc.map((glyph) => glyph.normalize("NFD"));
                    const options = Array.from(get("form").options,
                        (option) => option.value);
                    const guides = [];
                    for (const glyph of nfd) {
                        change("form", glyph);
                        change("character", glyph);
                        const guide = {
                            glyph, selected: get("form").value,
                            present: options.includes(glyph),
                            circle: get("ideal-circle")
                                .getAttribute("visibility"),
                            open: get("ideal-c-loop")
                                .getAttribute("visibility"),
                            rounded: get("ideal-rounded")
                                .getAttribute("visibility"),
                            acute: get("ideal-accent")
                                .getAttribute("visibility"),
                            marks: get("ideal-diacritic")
                                .getAttribute("visibility"),
                        };
                        get("construct").click();
                        guides.push({ ...guide,
                            status: get("status").textContent });
                    }
                    const variants = [
                        [nfd[0] + "-italic", 1100, "italic"],
                        [nfd[0] + "-cursive", 1100, "cursive"],
                        [nfd[0] + "-two", 700, "two-storey"],
                        [nfd[0] + "-two-italic", 700,
                            "two-storey-italic"],
                        [nfd[3] + "-italic", 1100, "italic"],
                        [nfd[3] + "-cursive", 1100, "cursive"],
                    ];
                    const styleGuides = [];
                    for (const [form, radius, kind] of variants) {
                        change("circle-radius", radius);
                        change("form", form);
                        change("character", form.split("-")[0]);
                        styleGuides.push({
                            form, kind, present: options.includes(form),
                            selected: get("form").value,
                            arch: get("ideal-arch")
                                .getAttribute("visibility"),
                            entry: get("ideal-entry")
                                .getAttribute("visibility"),
                            italicDisabled: get("italic-angle").disabled,
                        });
                    }
                    change("circle-radius", 1100);
                    change("form", "Ñ");
                    change("character", "Ñ");
                    get("construct").click();
                    const composed = get("status").textContent;
                    const names = [];
                    const blobs = [];
                    const create = URL.createObjectURL;
                    const click = HTMLAnchorElement.prototype.click;
                    try {
                        URL.createObjectURL = (blob) => {
                            blobs.push(blob);
                            return create.call(URL, blob);
                        };
                        HTMLAnchorElement.prototype.click = function () {
                            names.push(this.download);
                        };
                        get("save").click();
                        change("form", nfd[12]);
                        change("character", nfd[12]);
                        change("circle-x", 200);
                        const hidden = get("ideal-diacritic")
                            .getAttribute("visibility");
                        get("construct").click();
                        const refused = get("status").textContent;
                        get("save").click();
                        return JSON.stringify({ nfd, guides, composed,
                            styleGuides, hidden, refused, names,
                            files: await Promise.all(blobs.map((blob) =>
                                blob.text())) });
                    } finally {
                        URL.createObjectURL = create;
                        HTMLAnchorElement.prototype.click = click;
                    }
                })()`,
                resultOwnership: "none",
                target: { context: tab.context },
            }, FIREFOX_BIDI_READY_TIMEOUT_MS);
            assert.equal(response.type, "success");
            assert.equal(response.result.type, "string");
            const actual = JSON.parse(response.result.value);
            const composed = ["á", "é", "í", "ó", "ú", "ñ", "ü",
                "Á", "É", "Í", "Ó", "Ú", "Ñ", "Ü"];
            const nfd = composed.map((glyph) => glyph.normalize("NFD"));
            assert.deepEqual(actual.nfd, nfd);
            for (const [index, glyph] of nfd.entries()) {
                const guide = actual.guides[index];
                assert.equal(guide.glyph, glyph);
                assert.equal(guide.selected, glyph,
                    "real HTML option retains exact decomposed scalar order");
                assert.equal(guide.present, true);
                assert.match(guide.status, /Constructed \d+ sampled segments/);
                const canonical = composed[index];
                assert.equal(guide.acute, ["á", "é", "í", "ó", "ú",
                    "Á", "É", "Í", "Ó", "Ú"].includes(canonical)
                    ? "visible" : "hidden");
                assert.equal(guide.marks, ["ñ", "ü", "Ñ", "Ü"]
                    .includes(canonical) ? "visible" : "hidden");
                if (canonical === "ó" || canonical === "Ó") {
                    assert.equal(guide.circle, "visible");
                }
                if (canonical === "é") {
                    assert.equal(guide.open, "visible");
                }
            }
            assert.equal(actual.styleGuides.length, 6);
            for (const guide of actual.styleGuides) {
                assert.equal(guide.present, true,
                    `${guide.form} is an actual HTML select option`);
                assert.equal(guide.selected, guide.form);
                assert.equal(guide.arch,
                    guide.kind.startsWith("two-storey")
                        ? "visible" : "hidden");
                assert.equal(guide.entry,
                    guide.kind === "cursive" ? "visible" : "hidden");
                assert.equal(guide.italicDisabled,
                    !guide.kind.endsWith("italic"));
            }
            assert.match(actual.composed, /Constructed \d+ sampled segments/);
            assert.equal(actual.hidden, "hidden");
            assert.match(actual.refused, /fit inside glyph cell/);
            assert.deepEqual(actual.names,
                ["my-glyph-strokes.tsv", "my-glyph-strokes.tsv"]);
            const glyphs = [...nfd, "Ñ"];
            const rows = glyphs.flatMap((glyph) => geometricStarter(
                glyph, 1550, 2450, 1100, 250, "#243647"));
            const expected = encodeGlyphLibrary(rows);
            assert.deepEqual(actual.files, [expected, expected],
                "real Firefox refusal cannot alter saved NFD and NFC ink");
            assert.deepEqual(requests, baseline,
                "local glyph editing and downloads create no backend calls");

            const library = path.join(profile, "exact-nfd.tsv");
            fs.writeFileSync(library, actual.files[0]);
            const build = spawnSync(PINNED_CARGO, [
                "build", "--offline", "--quiet",
                "-p", "atrament_profile_editor_cli",
                "--bin", "atrament-profiles",
            ], { timeout: 90_000, stdio: "pipe",
                env: { ...process.env, CARGO_TARGET_DIR: TARGET_DIR } });
            assert.equal(build.status, 0,
                "native Rust CLI must build offline for browser NFD ink");
            const cli = path.join(TARGET_DIR, "debug", "atrament-profiles");
            const invoke = (...flags) => spawnSync(cli, flags, {
                timeout: 20_000, maxBuffer: 16 * 1024 * 1024,
            });
            const reopened = invoke("glyphs", "--profile", library);
            assert.equal(reopened.status, 0, reopened.stderr.toString());
            assert.equal(reopened.stdout.toString(), expected);
            const audit = invoke("glyphs", "--profile", library,
                "--audit-text", glyphs.join(""));
            assert.equal(audit.status, 0, audit.stderr.toString());
            assert.match(audit.stdout.toString(), /Missing authored glyphs: 0/);
            assert.match(audit.stdout.toString(),
                /Distinct visible graphemes: 15/);
            const flags = ["notebook", "--printed-sheet", "true",
                "--glyph-file", library,
                "--flow-text", nfd.join("") + "\nÑ"];
            const output = path.join(profile, "exact-nfd.pdf");
            const pdf = invoke(...flags, "--export-paper", "pdf",
                "--output", output);
            assert.equal(pdf.status, 0, pdf.stderr.toString());
            const approved = fs.readFileSync(output);
            assert.match(approved.toString("latin1"), /^%PDF-1\.4/);
            assert.equal((approved.toString("latin1").match(/ m\n/g) ?? [])
                .length, rows.length,
                "real Firefox NFD and NFC ink stays physical and exact");
            const png = invoke(...flags, "--export-paper", "png",
                "--dpi", "72");
            assert.equal(png.status, 0, png.stderr.toString());
            assert.deepEqual(png.stdout.subarray(0, 8),
                Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]));
            assert.equal(png.stdout.readUInt32BE(16), 596);
            assert.equal(png.stdout.readUInt32BE(20), 842);
            assert.equal(png.stdout[25], 6,
                "printed page PNG has transparent RGBA channels");
            const chunks = [];
            for (let offset = 8; offset + 12 <= png.stdout.length;) {
                const length = png.stdout.readUInt32BE(offset);
                if (png.stdout.toString("ascii", offset + 4, offset + 8)
                    === "IDAT") {
                    chunks.push(png.stdout.subarray(offset + 8,
                        offset + 8 + length));
                }
                offset += length + 12;
            }
            const pixels = inflateSync(Buffer.concat(chunks));
            assert.equal(pixels[4], 0, "page corner is transparent");
            assert.ok(pixels.includes(Buffer.from([36, 54, 71, 255])),
                "native PNG contains browser-authored colored pigment");
            const refused = invoke("notebook", "--printed-sheet", "true",
                "--glyph-file", library, "--flow-text", "A\u0300",
                "--export-paper", "pdf", "--output", output, "--replace");
            assert.notEqual(refused.status, 0);
            assert.match(refused.stderr.toString(), /missing authored glyph/i);
            assert.deepEqual(fs.readFileSync(output), approved);
            assert.equal(fs.readFileSync(library, "utf8"), expected);
        } finally {
            bidi?.close();
            await closeFixtureServer(server);
            for (const child of children.reverse()) await stopChild(child);
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

// No DOM focus, click, value mutation or synthetic events author ink here:
// WebDriver's trusted Tab/Shift+Tab/Ctrl+A/Enter input owns the complete path.
// The download spy observes the application's own anchor click, but does not
// trigger it, write outside the test profile or alter the generated bytes.
test(
    "Firefox trusted keyboard fills and downloads exact bilingual glyph ink",
    { skip: !FIREFOX_AVAILABLE, timeout: 120_000 },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/glyph-keyboard-journey-");
        const requests = [];
        const server = createSessionFixtureServer(requests);
        const children = [];
        let bidi;
        try {
            server.listen(0, "127.0.0.1");
            await once(server, "listening");
            const address = server.address();
            assert.notEqual(address, null);
            assert.notEqual(typeof address, "string");
            bidi = await startBidiFirefox(profile, children);
            const tab = await bidi.command("browsingContext.create", {
                type: "tab",
            });
            const context = tab.context;
            await bidi.command("browsingContext.navigate", {
                context,
                url: `http://127.0.0.1:${address.port}/`
                    + `#session=${REFRESH_SECRET}`,
                wait: "complete",
            });
            await waitForSessionStatus(bidi, context, "Session ready");
            const baselineRequests = [...requests];
            const state = async () => {
                const reply = await bidi.command("script.evaluate", {
                    awaitPromise: false,
                    expression: `JSON.stringify((() => {
                        const get = (id) => document.getElementById(
                            "glyph-" + id);
                        return {
                            active: document.activeElement?.id,
                            coverage: get("baseline-coverage").textContent,
                            center: get("circle-x").value,
                            form: get("form").value,
                            seedDisabled: get("seed-baseline").disabled,
                            status: get("status").textContent
                        };
                    })())`,
                    resultOwnership: "none",
                    target: { context },
                });
                assert.equal(reply.type, "success");
                assert.equal(reply.result.type, "string");
                return JSON.parse(reply.result.value);
            };
            assert.match((await state()).coverage, /0\/114 exact glyphs/);
            assert.equal((await state()).seedDisabled, false);

            await tabToControl(bidi, context, "glyph-focus-missing");
            await performTrustedKey(bidi, context, "\uE007");
            assert.equal((await state()).form, "ü");
            assert.match((await state()).status, /draft unchanged/);

            await tabToControl(bidi, context, "glyph-circle-x", true);
            await replaceWithTrustedKeys(bidi, context, "200");
            assert.equal((await state()).center, "200");
            await tabToControl(bidi, context, "glyph-seed-baseline");
            await performTrustedKey(bidi, context, "\uE007");
            const rejected = await state();
            assert.match(rejected.status, /fit inside glyph cell/);
            assert.match(rejected.coverage, /0\/114 exact glyphs/);
            assert.equal(rejected.active, "glyph-seed-baseline");

            await tabToControl(bidi, context, "glyph-circle-x", true);
            await replaceWithTrustedKeys(bidi, context, "1550");
            await tabToControl(bidi, context, "glyph-seed-baseline");
            await performTrustedKey(bidi, context, "\uE007");
            const accepted = await state();
            assert.match(accepted.status, /Filled 114 missing bilingual/);
            assert.match(accepted.coverage, /114\/114 exact glyphs; complete/);
            await performTrustedKey(bidi, context, "\uE007");
            assert.match((await state()).status,
                /All 114 bilingual text glyphs already have ink/);

            const installed = await bidi.command("script.evaluate", {
                awaitPromise: false,
                expression: `(() => {
                    window.__keyboardGlyphBlobs = [];
                    window.__keyboardGlyphNames = [];
                    const makeUrl = URL.createObjectURL;
                    URL.createObjectURL = function (blob) {
                        window.__keyboardGlyphBlobs.push(blob);
                        return makeUrl.call(URL, blob);
                    };
                    HTMLAnchorElement.prototype.click = function () {
                        window.__keyboardGlyphNames.push(this.download);
                    };
                    return true;
                })()`,
                resultOwnership: "none",
                target: { context },
            });
            assert.equal(installed.type, "success");
            await tabToControl(bidi, context, "glyph-save");
            await performTrustedKey(bidi, context, "\uE007");
            const observed = await bidi.command("script.evaluate", {
                awaitPromise: true,
                expression: `Promise.all(
                    window.__keyboardGlyphBlobs.map((blob) => blob.text())
                ).then((files) => JSON.stringify({
                    files, names: window.__keyboardGlyphNames
                }))`,
                resultOwnership: "none",
                target: { context },
            });
            assert.equal(observed.type, "success");
            assert.equal(observed.result.type, "string");
            const download = JSON.parse(observed.result.value);
            const rows = fillMissingBilingualBaseline([], 1550, 2450,
                1100, 250, "#243647", { dx: 0, lift: 0 });
            assert.deepEqual(download, {
                files: [encodeGlyphLibrary(rows)],
                names: ["my-glyph-strokes.tsv"],
            }, "trusted Enter downloads exact complete glyph geometry");
            assert.deepEqual(requests, baselineRequests,
                "keyboard-only edits and download never contact the server");
        } finally {
            bidi?.close();
            await closeFixtureServer(server);
            for (const child of children.reverse()) await stopChild(child);
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

// Real Firefox exercises the new button rather than a surrogate encoder.
// The browser only emits the TSV after explicit Download; Rust independently
// reopens those exact bytes and renders physical PDF/transparent PNG ink.
test(
    "Firefox one-click 114 bilingual glyphs download native PDF/PNG ink",
    { skip: !FIREFOX_AVAILABLE, timeout: 120_000 },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/baseline-firefox-");
        const requests = [];
        const server = createSessionFixtureServer(requests);
        const children = [];
        let bidi;
        try {
            server.listen(0, "127.0.0.1");
            await once(server, "listening");
            const address = server.address();
            assert.notEqual(address, null);
            assert.notEqual(typeof address, "string");
            bidi = await startBidiFirefox(profile, children);
            const tab = await bidi.command("browsingContext.create", {
                type: "tab",
            });
            await bidi.command("browsingContext.navigate", {
                context: tab.context,
                url: `http://127.0.0.1:${address.port}/`
                    + `#session=${REFRESH_SECRET}`,
                wait: "complete",
            });
            await waitForSessionStatus(bidi, tab.context, "Session ready");
            const baseline = [...requests];
            const response = await bidi.command("script.evaluate", {
                awaitPromise: true,
                expression: String.raw`(async () => {
                    const get = (name) =>
                        document.getElementById("glyph-" + name);
                    const names = [];
                    const blobs = [];
                    const makeURL = URL.createObjectURL;
                    const anchorClick = HTMLAnchorElement.prototype.click;
                    try {
                        URL.createObjectURL = (blob) => {
                            blobs.push(blob);
                            return makeURL.call(URL, blob);
                        };
                        HTMLAnchorElement.prototype.click = function () {
                            names.push(this.download);
                        };
                        const enabled = !get("seed-baseline").disabled;
                        const coverageBefore =
                            get("baseline-coverage").textContent;
                        get("character").value = "ü";
                        get("character").dispatchEvent(new Event("input"));
                        get("focus-missing").click();
                        const selectedMissing = get("character").value;
                        const selectedForm = get("form").value;
                        const inspectStatus = get("status").textContent;
                        get("circle-x").value = "200";
                        get("seed-baseline").click();
                        const refused = get("status").textContent;
                        const coverageRefused =
                            get("baseline-coverage").textContent;
                        const before = blobs.length;
                        get("circle-x").value = "1550";
                        get("seed-baseline").click();
                        const filled = get("status").textContent;
                        const coverageFilled =
                            get("baseline-coverage").textContent;
                        const unsaved = blobs.length;
                        get("seed-baseline").click();
                        const already = get("status").textContent;
                        const coverageAlready =
                            get("baseline-coverage").textContent;
                        get("save").click();
                        return JSON.stringify({ enabled, refused, before,
                            filled, unsaved, already, names,
                            coverageBefore, coverageRefused, coverageFilled,
                            coverageAlready, selectedMissing, selectedForm,
                            inspectStatus, files: await Promise.all(
                                blobs.map((blob) => blob.text())) });
                    } finally {
                        URL.createObjectURL = makeURL;
                        HTMLAnchorElement.prototype.click = anchorClick;
                    }
                })()`,
                resultOwnership: "none",
                target: { context: tab.context },
            }, FIREFOX_BIDI_READY_TIMEOUT_MS);
            assert.equal(response.type, "success");
            assert.equal(response.result.type, "string");
            const actual = JSON.parse(response.result.value);
            assert.equal(actual.enabled, true,
                "button is available only after admitted session");
            assert.match(actual.refused, /fit inside glyph cell/);
            assert.match(actual.coverageBefore, /0\/114 exact glyphs/);
            assert.equal(actual.selectedMissing, "a\u0301");
            assert.equal(actual.selectedForm, actual.selectedMissing);
            assert.match(actual.inspectStatus, /á \(NFD\); draft unchanged/);
            assert.equal(actual.coverageRefused, actual.coverageBefore,
                "refused geometry cannot alter the live coverage audit");
            assert.equal(actual.before, 0,
                "refused candidate cannot trigger a download");
            assert.match(actual.filled,
                /Filled 114 missing bilingual baseline glyphs/);
            assert.equal(actual.unsaved, 0, "fill is a local unsaved draft");
            assert.match(actual.already,
                /All 114 bilingual text glyphs already have ink/);
            assert.match(actual.coverageFilled,
                /114\/114 exact glyphs; complete/);
            assert.equal(actual.coverageAlready, actual.coverageFilled);
            assert.deepEqual(actual.names, ["my-glyph-strokes.tsv"]);
            const expectedRows = fillMissingBilingualBaseline([], 1550,
                2450, 1100, 250, "#243647", { dx: 0, lift: 0 });
            const exactTsv = encodeGlyphLibrary(expectedRows);
            assert.deepEqual(actual.files, [exactTsv],
                "real Firefox button emits exact 2728-line baseline ink");
            assert.deepEqual(requests, baseline,
                "all browser glyph generation stays local");

            const library = path.join(profile, "bilingual-browser.tsv");
            fs.writeFileSync(library, actual.files[0]);
            const build = spawnSync(PINNED_CARGO, [
                "build", "--offline", "--quiet",
                "-p", "atrament_profile_editor_cli",
                "--bin", "atrament-profiles",
            ], { timeout: 90_000, stdio: "pipe",
                env: { ...process.env, CARGO_TARGET_DIR: TARGET_DIR } });
            assert.equal(build.status, 0,
                "native CLI builds offline for real Firefox baseline ink");
            const cli = path.join(TARGET_DIR, "debug", "atrament-profiles");
            const invoke = (...flags) => spawnSync(cli, flags, {
                timeout: 20_000, maxBuffer: 16 * 1024 * 1024,
            });
            const reopened = invoke("glyphs", "--profile", library);
            assert.equal(reopened.status, 0, reopened.stderr.toString());
            assert.equal(reopened.stdout.toString(), exactTsv);
            const audit = invoke("glyphs", "--profile", library,
                "--audit-baseline");
            assert.equal(audit.status, 0, audit.stderr.toString());
            assert.match(audit.stdout.toString(),
                /Distinct visible graphemes: 114/);
            assert.match(audit.stdout.toString(), /Missing authored glyphs: 0/);
            const inkText = "¿Qué? o\u0301 Ñ¡";
            const counts = new Map();
            for (const line of expectedRows) counts.set(line.grapheme,
                (counts.get(line.grapheme) ?? 0) + 1);
            const expectedInk = [...new Intl.Segmenter("es", {
                granularity: "grapheme",
            }).segment(inkText)].reduce((count, item) =>
                count + (counts.get(item.segment) ?? 0), 0);
            const flags = ["notebook", "--printed-sheet", "true",
                "--glyph-file", library, "--flow-text", inkText];
            const pdfPath = path.join(profile, "bilingual.pdf");
            const pdf = invoke(...flags, "--export-paper", "pdf",
                "--output", pdfPath);
            assert.equal(pdf.status, 0, pdf.stderr.toString());
            const accepted = fs.readFileSync(pdfPath);
            assert.match(accepted.toString("latin1"), /^%PDF-1\.4/);
            assert.equal((accepted.toString("latin1").match(/ m\n/g) ?? [])
                .length, expectedInk,
            "the native page draws exactly the selected bilingual pen ink");
            const pngPath = path.join(profile, "bilingual.png");
            const png = invoke(...flags, "--export-paper", "png",
                "--dpi", "72", "--output", pngPath);
            assert.equal(png.status, 0, png.stderr.toString());
            const rgba = fs.readFileSync(pngPath);
            assert.deepEqual(rgba.subarray(0, 8),
                Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]));
            assert.equal(rgba.readUInt32BE(16), 596);
            assert.equal(rgba.readUInt32BE(20), 842);
            assert.equal(rgba[25], 6, "actual transparent RGBA ink");
            const unknown = invoke("notebook", "--printed-sheet", "true",
                "--glyph-file", library, "--flow-text", "☃",
                "--export-paper", "pdf", "--output", pdfPath,
                "--replace");
            assert.notEqual(unknown.status, 0);
            assert.match(unknown.stderr.toString(), /missing authored glyph/i);
            assert.deepEqual(fs.readFileSync(pdfPath), accepted,
                "missing nonbaseline shape cannot replace an accepted PDF");
            assert.equal(fs.readFileSync(library, "utf8"), exactTsv);
        } finally {
            bidi?.close();
            await closeFixtureServer(server);
            for (const child of children.reverse()) await stopChild(child);
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);

test(
    "Firefox cursive c/e/i/l/t/n/u/m/h/r and accents export native ink",
    { skip: !FIREFOX_AVAILABLE, timeout: 120_000 },
    async () => {
        fs.mkdirSync(".temp", { recursive: true });
        const profile = fs.mkdtempSync(".temp/cursive-e-firefox-");
        const requests = [];
        const server = createSessionFixtureServer(requests);
        const children = [];
        let bidi;
        try {
            server.listen(0, "127.0.0.1");
            await once(server, "listening");
            const address = server.address();
            assert.notEqual(address, null);
            assert.notEqual(typeof address, "string");
            bidi = await startBidiFirefox(profile, children);
            const tab = await bidi.command("browsingContext.create", {
                type: "tab",
            });
            await bidi.command("browsingContext.navigate", {
                context: tab.context,
                url: `http://127.0.0.1:${address.port}/`
                    + `#session=${REFRESH_SECRET}`,
                wait: "complete",
            });
            await waitForSessionStatus(bidi, tab.context, "Session ready");
            const baseline = [...requests];
            const response = await bidi.command("script.evaluate", {
                awaitPromise: true,
                expression: String.raw`(async () => {
                    const get = (id) => document.getElementById("glyph-" + id);
                    const change = (id, value) => {
                        get(id).value = value;
                        get(id).dispatchEvent(new Event("input", {
                            bubbles: true,
                        }));
                    };
                    change("circle-radius", "700");
                    const constructed = [];
                    const guides = [];
                    for (const [form, exact, radius] of [
                        ["c-cursive", "c", 700], ["e-cursive", "e", 700],
                        ["é-cursive", "é", 700],
                        ["e\u0301-cursive", "e\u0301", 700],
                        ["i-cursive", "i", 1100],
                        ["í-cursive", "í", 1100],
                        ["i\u0301-cursive", "i\u0301", 1100],
                        ["l-cursive", "l", 1100],
                        ["n-cursive", "n", 1100],
                        ["ñ-cursive", "ñ", 1100],
                        ["n\u0303-cursive", "n\u0303", 1100],
                        ["u-cursive", "u", 1100],
                        ["ú-cursive", "ú", 1100],
                        ["ü-cursive", "ü", 1100],
                        ["u\u0301-cursive", "u\u0301", 1100],
                        ["u\u0308-cursive", "u\u0308", 1100],
                        ["m-cursive", "m", 1100],
                        ["h-cursive", "h", 1100],
                        ["t-cursive", "t", 1100],
                        ["r-cursive", "r", 1100],
                    ]) {
                        change("circle-radius", String(radius));
                        change("form", form);
                        change("character", exact);
                        guides.push({
                            arc: get("ideal-c-loop").getAttribute("d"),
                            arcVisible: get("ideal-c-loop")
                                .getAttribute("visibility"),
                            approach: get("ideal-approach")
                                .getAttribute("visibility"),
                            bar: get("ideal-e-bar")
                                .getAttribute("visibility"),
                            acute: get("ideal-accent")
                                .getAttribute("visibility"),
                            stem: get("ideal-i-stem")
                                .getAttribute("visibility"),
                            dot: get("ideal-i-dot")
                                .getAttribute("visibility"),
                            rounded: get("ideal-rounded")
                                .getAttribute("visibility"),
                            roundedD: get("ideal-rounded")
                                .getAttribute("d"),
                            leftTop: get("ideal-rounded-left")
                                .getAttribute("y2"),
                            tilde: get("ideal-diacritic")
                                .getAttribute("visibility"),
                            entry: get("ideal-entry")
                                .getAttribute("visibility"),
                        });
                        get("construct").click();
                        constructed.push(get("status").textContent);
                    }
                    const blobs = [];
                    const create = URL.createObjectURL;
                    const click = HTMLAnchorElement.prototype.click;
                    try {
                        URL.createObjectURL = (blob) => {
                            blobs.push(blob);
                            return create.call(URL, blob);
                        };
                        HTMLAnchorElement.prototype.click = function () {};
                        get("save").click();
                        change("form", "i\u0301-cursive");
                        change("character", "i\u0301");
                        change("circle-radius", "700");
                        const clipped = get("ideal-i-stem")
                            .getAttribute("visibility");
                        get("construct").click();
                        const refused = get("status").textContent;
                        get("save").click();
                        return JSON.stringify({
                            constructed, guides, clipped, refused,
                            files: await Promise.all(blobs.map((blob) =>
                                blob.text())),
                        });
                    } finally {
                        URL.createObjectURL = create;
                        HTMLAnchorElement.prototype.click = click;
                    }
                })()`,
                resultOwnership: "none",
                target: { context: tab.context },
            }, FIREFOX_BIDI_READY_TIMEOUT_MS);
            assert.equal(response.type, "success");
            assert.equal(response.result.type, "string");
            const actual = JSON.parse(response.result.value);
            for (const guide of actual.guides.slice(0, 4)) {
                assert.equal(guide.arcVisible, "visible");
                assert.equal(guide.bar, "hidden");
                assert.match(guide.arc, / A 700 700 0 1 0 /);
            }
            for (const guide of actual.guides.slice(4, 8)) {
                assert.equal(guide.arcVisible, "hidden");
                assert.equal(guide.stem, "visible");
                assert.equal(guide.entry, "visible");
                assert.equal(guide.approach, "hidden");
            }
            assert.deepEqual(actual.guides.map((guide) => guide.approach),
                ["hidden", "visible", "visible", "visible",
                    "hidden", "hidden", "hidden", "hidden",
                    "visible", "visible", "visible",
                    "hidden", "hidden", "hidden", "hidden", "hidden",
                    "visible", "visible", "hidden", "visible"]);
            assert.deepEqual(actual.guides.map((guide) => guide.acute),
                ["hidden", "hidden", "visible", "visible",
                    "hidden", "visible", "visible", "hidden",
                    "hidden", "hidden", "hidden",
                    "hidden", "visible", "hidden", "visible", "hidden",
                    "hidden", "hidden", "hidden", "hidden"]);
            assert.deepEqual(actual.guides.slice(4, 8).map((g) => g.dot),
                ["visible", "hidden", "hidden", "hidden"]);
            assert.match(actual.constructed[0], /38 sampled segments/);
            assert.match(actual.constructed[1], /39 sampled segments/);
            assert.match(actual.constructed[2], /40 sampled segments/);
            assert.match(actual.constructed[3], /40 sampled segments/);
            for (const status of actual.constructed.slice(4, 7)) {
                assert.match(status, /4 sampled segments/);
            }
            assert.match(actual.constructed[7], /3 sampled segments/);
            assert.deepEqual(actual.guides.slice(8, 11).map((g) => g.rounded),
                ["visible", "visible", "visible"]);
            assert.deepEqual(actual.guides.slice(8, 11).map((g) => g.tilde),
                ["hidden", "visible", "visible"]);
            assert.deepEqual(actual.constructed.slice(8).map((s) =>
                Number(s.match(/(\d+) sampled segments/)?.[1])),
            [37, 40, 40, 36, 37, 38, 37, 38, 71, 37, 4, 36]);
            assert.deepEqual(actual.guides.slice(11).map((g) => g.rounded),
                ["visible", "visible", "visible", "visible", "visible",
                    "visible", "visible", "hidden", "visible"]);
            assert.deepEqual(actual.guides.slice(11).map((g) => g.tilde),
                ["hidden", "hidden", "visible", "hidden", "visible",
                    "hidden", "hidden", "hidden", "hidden"]);
            assert.match(actual.guides.at(-4).roundedD,
                /M 1550 2450 L 1550 3550 L 1550 2450$/);
            assert.equal(actual.guides.at(-3).leftTop, "800");
            assert.equal(actual.guides.at(-2).stem, "visible");
            assert.equal(actual.guides.at(-2).bar, "visible");
            assert.equal(actual.guides.at(-2).entry, "visible");
            assert.equal(actual.guides.at(-1).rounded, "visible");
            assert.equal(actual.guides.at(-1).approach, "visible");
            assert.equal(actual.guides.at(-1).stem, "hidden");
            assert.equal(actual.clipped, "hidden");
            assert.match(actual.refused,
                /Dotted i requires visible pen-clearance/);
            const glyphs = ["c", "e", "é", "e\u0301",
                "i", "í", "i\u0301", "l", "n", "ñ", "n\u0303",
                "u", "ú", "ü", "u\u0301", "u\u0308", "m", "h", "t", "r"];
            const expectedInk = glyphs.flatMap(
                (glyph, index) => geometricStarter(glyph, 1550, 2450,
                    index < 4 ? 700 : 1100, 250, "#243647",
                    undefined, "cursive"));
            const expected = encodeGlyphLibrary(expectedInk);
            assert.deepEqual(actual.files, [expected, expected]);
            assert.deepEqual(requests, baseline,
                "glyph edits and deliberate downloads stay browser-local");
            const library = path.join(profile, "cursive.tsv");
            fs.writeFileSync(library, actual.files[0]);
            const build = spawnSync(PINNED_CARGO, [
                "build", "--offline", "--quiet",
                "-p", "atrament_profile_editor_cli",
                "--bin", "atrament-profiles",
            ], {
                timeout: 90_000, stdio: "pipe",
                env: { ...process.env, CARGO_TARGET_DIR: TARGET_DIR },
            });
            assert.equal(build.status, 0,
                "offline Rust CLI builds before real Firefox TSV replay");
            const cli = path.join(TARGET_DIR, "debug", "atrament-profiles");
            const invoke = (...args) => spawnSync(cli, args, {
                timeout: 20_000, maxBuffer: 16 * 1024 * 1024,
            });
            const opened = invoke("glyphs", "--profile", library);
            assert.equal(opened.status, 0, opened.stderr.toString());
            assert.equal(opened.stdout.toString(), expected);
            const text = glyphs.join("");
            for (const format of ["pdf", "png"]) {
                const result = invoke("notebook", "--printed-sheet",
                    "true", "--glyph-file", library,
                    "--flow-text", `${text}\n${text}`,
                    "--export-paper", format,
                    ...(format === "png" ? ["--dpi", "72"] : []));
                assert.equal(result.status, 0, result.stderr.toString());
                if (format === "pdf") {
                    assert.match(result.stdout.toString("latin1"),
                        /^%PDF-1\.4/);
                    assert.equal((result.stdout.toString("latin1")
                        .match(/ m\n/g) ?? []).length,
                        expectedInk.length * 2);
                } else {
                    assert.deepEqual(result.stdout.subarray(0, 8),
                        Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]));
                    assert.equal(result.stdout[25], 6,
                        "rendered page is transparent RGBA PNG");
                }
            }
            assert.equal(fs.readFileSync(library, "utf8"), expected);
        } finally {
            bidi?.close();
            await closeFixtureServer(server);
            for (const child of children.reverse()) await stopChild(child);
            fs.rmSync(profile, { recursive: true, force: true });
        }
    },
);
