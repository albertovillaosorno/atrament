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
//   - Read-only coverage of external Rust integration test target linkage.
// - Must-Not:
//   - Modify manifests, invent tests, or silently ignore unregistered roots.
// - Allows:
//   - Inputs: Repository Rust test roots and their Cargo package manifests.
//   - Outputs: Structural assertions over unique and resolvable test targets.
//   - Side effects: Read-only repository file access.
// - Split-When:
//   - Cargo workspace metadata parsing requires a distinct native harness.
// - Merge-When:
//   - A verified repository test owner already proves complete target linkage.
// - Summary:
//   - Keeps every external Rust regression in the executable Cargo test graph.
// - Description:
//   - Compares one explicit test declaration against every backend test root.
// - Usage:
//   - Run through the repository frontend test script.
// - Defaults:
//   - No implicitly discovered test file counts as executable coverage.
//

//! Every backend regression root must be reachable through a Cargo test target.

import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import { dirname, relative, resolve, sep } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../../../..");
const testsRoot = resolve(root, "tests/backend");
const packagesRoot = resolve(root, "src/backend");

function findRustTestRoots(parent) {
    return readdirSync(parent, { withFileTypes: true }).flatMap((item) => {
        const path = resolve(parent, item.name);
        if (item.isDirectory()) {
            return findRustTestRoots(path);
        }
        assert.ok(item.isFile(), `unexpected test root entry: ${path}`);
        return item.name === "lib.rs" ? [path] : [];
    });
}

function declaredTargets(manifest, content) {
    const targetPattern =
        /\[\[test\]\]\s*\nname = "([a-z0-9_]+)"\s*\npath = "([^"\n]+)"/g;
    const entries = [...content.matchAll(targetPattern)].map((match) => ({
        name: match[1],
        path: resolve(dirname(manifest), match[2]),
    }));
    const declarations = content.match(/^\[\[test\]\]$/gm) ?? [];
    assert.equal(
        entries.length,
        declarations.length,
        `unrecognized Cargo test declaration: ${manifest}`,
    );
    return entries;
}

function checkTestTargetGraph(manifests, testRoots) {
    const declaredByFile = new Map();
    const declaredNames = new Set();
    for (const [manifest, content] of manifests) {
        const crate = dirname(manifest).split(sep).at(-1);
        for (const target of declaredTargets(manifest, content)) {
            assert.ok(
                target.path.startsWith(testsRoot + sep),
                `test target escapes backend tests: ${target.path}`,
            );
            assert.ok(!declaredByFile.has(target.path),
                `duplicate Cargo target: ${target.path}`);
            assert.ok(!declaredNames.has(target.name),
                `duplicate Cargo test name: ${target.name}`);
            const suffix = relative(testsRoot, target.path).split(sep);
            assert.equal(suffix.length, 3);
            assert.equal(suffix[0], crate, "test owned by wrong Cargo crate");
            assert.equal(suffix[2], "lib.rs");
            const prefix = `${crate}_`.replaceAll("-", "_");
            assert.ok(target.name.startsWith(prefix),
                "test target identity must name its owning crate");
            declaredByFile.set(target.path, manifest);
            declaredNames.add(target.name);
        }
    }
    const discovered = new Set(testRoots);
    for (const path of discovered) {
        assert.ok(declaredByFile.has(path), `Rust tests not wired: ${path}`);
    }
    for (const path of declaredByFile.keys()) {
        assert.ok(discovered.has(path), `orphan Cargo test target: ${path}`);
    }
}

test("all backend test roots have unique executable Cargo test targets", () => {
    const packages = readdirSync(packagesRoot, { withFileTypes: true });
    const manifests = packages.filter((p) => p.isDirectory()).map((p) => {
        const manifest = resolve(packagesRoot, p.name, "Cargo.toml");
        return [manifest, readFileSync(manifest, "utf8")];
    });
    const testRoots = findRustTestRoots(testsRoot);
    assert.ok(testRoots.length > 0, "missing backend regression evidence");
    checkTestTargetGraph(manifests, testRoots);
});

test("target coverage rejects duplicate and unrelated files", () => {
    const manifest = resolve(packagesRoot, "example", "Cargo.toml");
    const path = resolve(testsRoot, "example/domain/lib.rs");
    const content =
        '[[test]]\nname = "example_domain"\n' +
        'path = "../../../tests/backend/example/domain/lib.rs"\n';
    assert.throws(() => checkTestTargetGraph([[manifest, content]], [
        path, resolve(testsRoot, "example/other/lib.rs"),
    ]), /not wired/);
    assert.throws(() => checkTestTargetGraph([
        [manifest, content + content],
    ], [path]), /duplicate/);
});
