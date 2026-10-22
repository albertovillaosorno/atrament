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
//   - Structural regression evidence for linked typed TODO records.
// - Must-Not:
//   - Edit task records or infer unspecified dependencies.
// - Allows:
//   - Inputs: Repository-authored TODO and per-task Markdown files.
//   - Outputs: Deterministic assertions over record identity and integrity.
//   - Side effects: Read-only repository file access.
// - Split-When:
//   - Task lifecycles gain independent index or migration validation.
// - Merge-When:
//   - Another repository test already verifies identical record invariants.
// - Summary:
//   - Keeps the TODO index short, linked, and consistent with typed records.
// - Description:
//   - Ensures all pending tasks have one resolvable canonical record.
// - Usage:
//   - Run through the repository frontend test script.
// - Defaults:
//   - Dependencies remain unspecified until their owners review them.
//
import assert from "node:assert/strict";
import { existsSync, readFileSync, readdirSync } from "node:fs";
import { dirname, relative, resolve } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../../../..");
const todo = readFileSync(resolve(root, "TODO.md"), "utf8");
const open = resolve(root, "docs/todo/open");

function discoverRecords(dir) {
    return readdirSync(dir, { withFileTypes: true }).flatMap((item) => {
        const path = resolve(dir, item.name);
        if (item.isDirectory()) {
            return discoverRecords(path);
        }
        assert.ok(item.isFile(), `unexpected task entry: ${path}`);
        assert.match(item.name, /\.mdc$/, `non-record in ${dir}`);
        return [relative(root, path).replaceAll("\\", "/")];
    });
}

function indexEntries() {
    const entries = [];
    let horizon = null;
    let title = null;
    let paragraph = [];
    let number = 0;
    for (const line of todo.split("\n")) {
        const section = /^## (P\d+) — /.exec(line);
        if (section) {
            assert.equal(title, null, "task missing its direct record link");
            horizon = section[1];
            number = 0;
        }
        const heading = /^### TODO - (.+)$/.exec(line);
        if (heading) {
            assert.equal(title, null, "task missing its direct record link");
            assert.ok(horizon, "task must belong to a horizon");
            title = heading[1];
            paragraph = [];
            number += 1;
        } else if (title !== null) {
            const link =
                /^\[Record\]\((docs\/todo\/open\/[^)]+\.mdc)\)$/.exec(line);
            if (link) {
                const summary = paragraph.join(" ").trim();
                assert.ok(summary, `${title}: synthesis paragraph missing`);
                entries.push({
                    horizon, title, summary, path: link[1], number,
                });
                title = null;
            } else if (line.trim()) {
                paragraph.push(line.trim());
            }
        }
    }
    assert.equal(title, null, "last task missing its direct record link");
    return entries;
}

const entries = indexEntries();

test("every task links one typed record with matching identity", () => {
    assert.ok(entries.length > 0, "TODO must contain open records");
    const seen = new Set();
    for (const entry of entries) {
        assert.ok(!seen.has(entry.path), `duplicate record: ${entry.path}`);
        seen.add(entry.path);
        const target = resolve(root, entry.path);
        assert.ok(target.startsWith(open + "/"), "record escapes open root");
        assert.ok(existsSync(target), `missing record: ${entry.path}`);
        const content = readFileSync(target, "utf8");
        const suffix = String(entry.number).padStart(3, "0");
        const id = `ATR-${entry.horizon}-${suffix}`;
        assert.ok(
            content.startsWith("---\n"),
            `${id}: YAML frontmatter missing`,
        );
        assert.match(content, /^record_type: task$/m);
        assert.match(content, new RegExp(`^id: "${id}"$`, "m"));
        assert.ok(content.includes(`title: ${JSON.stringify(entry.title)}`));
        assert.match(content, /^status: open$/m);
        assert.match(content, new RegExp(`^priority: ${entry.horizon}$`, "m"));
        assert.match(content, /^lane: [a-z-]+$/m);
        assert.match(content, /^dependencies: \[\]$/m);
        assert.match(content, /^dependencies_specified: false$/m);
        assert.match(content, /^acceptance_criteria:\n  - >-$/m);
        assert.match(content, /^## Scope and existing evidence$/m);
    }
});

test("index contains no orphan or duplicated records", () => {
    const discovered = discoverRecords(open);
    const linked = entries.map((item) => item.path).sort();
    assert.deepEqual(discovered.sort(), linked);
    assert.ok(!todo.includes("## Scope and existing evidence"));
});

test("README does not advertise the obsolete project pause", () => {
    const readme = readFileSync(resolve(root, "README.md"), "utf8");
    assert.ok(!/\bpaused\b/i.test(readme));
});
