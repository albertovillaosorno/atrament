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
const completed = resolve(root, "docs/todo/completed");

function discoverRecords(dir, ignoreGitkeep = false) {
    return readdirSync(dir, { withFileTypes: true }).flatMap((item) => {
        if (ignoreGitkeep && item.name === ".gitkeep") {
            return [];
        }
        const path = resolve(dir, item.name);
        if (item.isDirectory()) {
            return discoverRecords(path, ignoreGitkeep);
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
    for (const line of todo.split("\n")) {
        const section = /^## (P[1-4]) — /.exec(line);
        if (section) {
            assert.equal(title, null, "task missing its direct record link");
            horizon = section[1];
        }
        const heading = /^### TODO - (.+)$/.exec(line);
        if (heading) {
            assert.equal(title, null, "task missing its direct record link");
            assert.ok(horizon, "task must belong to a horizon");
            title = heading[1];
            paragraph = [];
        } else if (title !== null) {
            const link =
                /^\[(.+?)\]\((docs\/todo\/open\/[^)]+\.mdc)\)$/
                    .exec(line);
            if (link) {
                const summary = paragraph.join(" ").trim();
                assert.ok(summary, `${title}: synthesis paragraph missing`);
                assert.equal(link[1], link[2],
                    `${title}: link label must be its exact record path`);
                const basename = /\/(\d{3})-[^/]+\.mdc$/.exec(link[1]);
                assert.ok(basename, `${title}: record filename lacks ID`);
                const content = readFileSync(resolve(root, link[1]), "utf8");
                const id = /^id: "(ATR-P[1-9]-\d{3})"$/m
                    .exec(content)?.[1];
                assert.ok(id, `${title}: stable task identity missing`);
                assert.equal(id.slice(-3), basename[1],
                    `${title}: filename must keep original identity`);
                entries.push({
                    horizon, title, summary, path: link[1], id,
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

function dependenciesFromRecord(content, taskId) {
    const field = /^dependencies: (\[.*\])$/m.exec(content);
    assert.ok(field, `${taskId}: typed prerequisites missing`);
    const dependencies = JSON.parse(field[1]);
    assert.ok(Array.isArray(dependencies), `${taskId}: array required`);
    const specified = /^dependencies_specified: (true|false)$/m.exec(content);
    assert.ok(specified, `${taskId}: dependency status missing`);
    const reviewed = specified[1] === "true";
    const rationale = /^dependency_rationale: >-\n  (.+)$/m.exec(content);
    if (reviewed) {
        assert.ok(rationale, `${taskId}: dependency rationale missing`);
    } else {
        assert.deepEqual(dependencies, [], `${taskId}: unreviewed edges`);
        assert.equal(rationale, null, `${taskId}: unreviewed rationale`);
    }
    assert.equal(new Set(dependencies).size, dependencies.length);
    for (const dependency of dependencies) {
        assert.match(dependency, /^ATR-P[1-9]-[0-9]{3}$/);
    }
    return dependencies;
}

function checkDependencyGraph(graph) {
    const active = new Set();
    const complete = new Set();
    function visit(taskId) {
        if (active.has(taskId)) {
            throw new Error(`dependency cycle through ${taskId}`);
        }
        if (complete.has(taskId)) {
            return;
        }
        active.add(taskId);
        for (const dependency of graph.get(taskId) ?? []) {
            if (!graph.has(dependency)) {
                throw new Error(`missing prerequisite: ${dependency}`);
            }
            visit(dependency);
        }
        active.delete(taskId);
        complete.add(taskId);
    }
    for (const taskId of graph.keys()) {
        visit(taskId);
    }
}


test("every task links one typed record with matching identity", () => {
    // A fully accepted finite backlog may have no open records. Require
    // a nonempty total inventory, not a permanent unfinished obligation.
    assert.ok(entries.length + discoverRecords(completed, true).length > 0,
        "typed task inventory must not vanish");
    const seen = new Set();
    for (const entry of entries) {
        assert.ok(!seen.has(entry.id), `duplicate task ID: ${entry.id}`);
        seen.add(entry.id);
        const target = resolve(root, entry.path);
        assert.ok(target.startsWith(open + "/"), "record escapes open root");
        assert.ok(existsSync(target), `missing record: ${entry.path}`);
        const content = readFileSync(target, "utf8");
        const id = entry.id;
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
        dependenciesFromRecord(content, id);
        assert.match(content, /^acceptance_criteria:\n  - >-$/m);
        assert.match(content, /^## Scope and existing evidence$/m);
        const forbidden =
            /^(?:status|priority|lane|dependencies|acceptance_criteria):/;
        assert.ok(!forbidden.test(entry.summary),
            `${id}: field labels belong in typed record`);
    }
});

test("reviewed TODO prerequisites exist and form an acyclic graph", () => {
    const graph = new Map();
    let reviewed = 0;
    const paths = [
        ...entries.map((entry) => entry.path),
        ...discoverRecords(completed, true),
    ];
    for (const path of paths) {
        const content = readFileSync(resolve(root, path), "utf8");
        const taskId = /^id: "(ATR-P[1-9]-\d{3})"$/m.exec(content)?.[1];
        assert.ok(taskId, `${path}: task identity missing`);
        assert.ok(!graph.has(taskId), `${taskId}: duplicate task identity`);
        graph.set(taskId, dependenciesFromRecord(content, taskId));
        if (/^dependencies_specified: true$/m.test(content)) {
            reviewed += 1;
        }
    }
    assert.ok(reviewed > 0, "reviewed dependency authority missing");
    checkDependencyGraph(graph);
});

test("dependency validation fails closed on orphan and cyclic edges", () => {
    const unknown = new Map([["ATR-P1-001", ["ATR-P1-999"]]]);
    assert.throws(() => checkDependencyGraph(unknown), /missing prerequisite/);
    const cyclic = new Map([
        ["ATR-P1-001", ["ATR-P1-002"]],
        ["ATR-P1-002", ["ATR-P1-001"]],
    ]);
    assert.throws(() => checkDependencyGraph(cyclic), /dependency cycle/);
});

test("completed tasks preserve immutable identities outside open index", () => {
    const indexed = new Set(entries.map((entry) => entry.id));
    const paths = discoverRecords(completed, true);
    assert.ok(paths.length > 0, "completed record inventory is empty");
    for (const path of paths) {
        const content = readFileSync(resolve(root, path), "utf8");
        const horizon = /^priority: (P[1-9])$/m.exec(content)?.[1];
        const ordinal = /\/(\d{3})-[^/]+\.mdc$/.exec(path)?.[1];
        assert.ok(horizon && ordinal, `${path}: canonical identity missing`);
        const id = `ATR-${horizon}-${ordinal}`;
        assert.match(content, new RegExp(`^id: "${id}"$`, "m"));
        assert.match(content, /^status: completed$/m);
        assert.ok(!indexed.has(id), `${id}: already open`);
        assert.match(content, /^## Acceptance decision$/m);
    }
});

test("index contains no orphan or duplicated records", () => {
    const discovered = discoverRecords(open, true);
    const linked = entries.map((item) => item.path).sort();
    assert.deepEqual(discovered.sort(), linked);
    assert.ok(!todo.includes("## Scope and existing evidence"));
});

test("README does not advertise the obsolete project pause", () => {
    const readme = readFileSync(resolve(root, "README.md"), "utf8");
    assert.ok(!/\bpaused\b/i.test(readme));
});

test("every accepted design has one implementation follow-up", () => {
    const completedPaths = discoverRecords(completed, true);
    const archived = new Map();
    for (const path of completedPaths) {
        const content = readFileSync(resolve(root, path), "utf8");
        if (!/^migrated_from: TODO\.md$/m.test(content)) {
            continue;
        }
        const id = /^id: "(ATR-P[1-9]-\d{3})"$/m.exec(content)?.[1];
        assert.ok(id, `${path}: original design identity missing`);
        archived.set(id, path);
    }
    assert.ok(archived.size > 0, "original design inventory missing");

    const mapped = new Map();
    const allPaths = [...discoverRecords(open, true), ...completedPaths];
    for (const path of allPaths) {
        const content = readFileSync(resolve(root, path), "utf8");
        if (!/^migrated_from: accepted-theoretical-designs$/m.test(content)) {
            continue;
        }
        assert.match(content, /^priority: P[1-4]$/m);
        const sourceBlock = /^source_designs:\n((?:  - "ATR-P[1-9]-\d{3}"\n)+)/m
            .exec(content)?.[1];
        assert.ok(sourceBlock, `${path}: missing stable source design IDs`);
        const sources = [...sourceBlock.matchAll(
            /^  - "(ATR-P[1-9]-\d{3})"$/gm,
        )].map((match) => match[1]);
        assert.ok(sources.length > 0, `${path}: empty source designs`);
        for (const id of sources) {
            assert.ok(archived.has(id), `${path}: unknown design ${id}`);
            assert.ok(!mapped.has(id),
                `${id}: already mapped by ${mapped.get(id)}`);
            mapped.set(id, path);
        }
    }
    assert.deepEqual([...mapped.keys()].sort(), [...archived.keys()].sort(),
        "all accepted designs must remain tracked for implementation");
});

test("one ordered P1-P4 list keeps stable task identities", () => {
    const sections = [...todo.matchAll(/^## (P\d+) — /gm)]
        .map((match) => match[1]);
    assert.deepEqual(sections, ["P1", "P2", "P3", "P4"]);
    assert.ok(entries.length > 0, "unfinished task list cannot be empty");
    let previous = 1;
    for (const entry of entries) {
        const rank = Number(entry.horizon.slice(1));
        assert.ok(rank >= previous, "priority must not decrease");
        previous = rank;
        assert.match(entry.id, /^ATR-P[1-9]-\d{3}$/);
    }
    assert.doesNotMatch(todo, /\b(?:NOW|MVP|after-mvp)\b/i);
});
