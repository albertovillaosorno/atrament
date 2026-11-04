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
//   - Read-only coverage of executable Rust regression test linkage.
// - Must-Not:
//   - Modify manifests, invent tests, or silently ignore unregistered roots.
// - Allows:
//   - Inputs: Repository Rust test roots, Cargo targets and cfg(test) modules.
//   - Outputs: Structural assertions over unique and resolvable test wiring.
//   - Side effects: Read-only repository file access.
// - Split-When:
//   - Cargo workspace metadata parsing requires a distinct native harness.
// - Merge-When:
//   - A verified repository test owner already proves complete target linkage.
// - Summary:
//   - Covers explicit integration tests and source-linked unit test modules.
// - Description:
//   - Refuses orphan roots without misclassifying compiled Rust unit modules.
// - Usage:
//   - Run through the repository frontend test script.
// - Defaults:
//   - No implicitly discovered test file counts as executable coverage.
//

//! Every backend regression root must be reachable through a Cargo test target.

import assert from "node:assert/strict";
import { existsSync, readFileSync, readdirSync } from "node:fs";
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
        if (item.name === "lib.rs") return [path];
        // Non-lib unit modules can be linked directly from a binary target.
        // Count files containing executable #[test] cases as required roots,
        // without mistaking supporting Rust fixtures for test entrypoints.
        if (item.name.endsWith(".rs")
            && /^#\[test\]$/mu.test(readFileSync(path, "utf8"))) {
            return [path];
        }
        return [];
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

// Cargo also runs #[cfg(test)] modules in its library and binary targets.
// Their #[path] can point outside src; the module is still an executable test.
// Only an explicit target path plus a directly adjacent cfg/test path directive
// counts. A stray string or an arbitrary file reference must not count.
function sourceTargetPaths(manifest, content) {
    const paths = [];
    let section = "";
    for (const line of content.split(/\r?\n/)) {
        const heading = line.match(
            /^(\[lib\]|\[\[bin\]\]|\[[^\]]+\]|\[\[[^\]]+\]\])$/,
        );
        if (heading !== null) {
            section = heading[1];
        }
        const path = line.match(/^path = "([^"\n]+)"$/);
        if (path !== null && (section === "[lib]" || section === "[[bin]]")) {
            paths.push(resolve(dirname(manifest), path[1]));
        }
    }
    return paths;
}

// Follow only explicitly imported Rust modules from Cargo entrypoints.
// Some library test roots live in application modules rather than the
// first composition file, so immediate-source inspection loses coverage.
function linkedUnitModules(manifest, content, sources) {
    const modules = [];
    const crateRoot = dirname(manifest);
    const ancestry = new Set();
    const modulePattern = new RegExp(
        String.raw`(?:^([ \t]*#\[cfg\(test\)\][ \t]*\r?\n))?` +
        String.raw`^[ \t]*#\[path[ \t]*=[ \t]*"([^"\n]+)"\][ \t]*\r?\n` +
        String.raw`^[ \t]*(?:pub(?:\([^)]+\))?[ \t]+)?` +
        String.raw`mod[ \t]+[a-zA-Z_]\w*[ \t]*;`, "gm",
    );
    function visit(sourcePath) {
        assert.ok(!ancestry.has(sourcePath),
            `explicit Rust module path cycle: ${sourcePath}`);
        ancestry.add(sourcePath);
        const source = sources?.get(sourcePath)
            ?? readFileSync(sourcePath, "utf8");
        for (const match of source.matchAll(modulePattern)) {
            // A non-test conditional cannot prove the module participates
            // in this test build; it is not an executable coverage edge.
            const before = source.slice(0, match.index).trimEnd();
            const prior = before.split(/\r?\n/).at(-1) ?? "";
            if (match[1] === undefined && /^#\[cfg\(/u.test(prior)) {
                continue;
            }
            const child = resolve(dirname(sourcePath), match[2]);
            if (match[1] !== undefined) {
                modules.push(child);
            } else {
                assert.ok(child.startsWith(crateRoot + sep),
                    `source module escapes owning crate: ${child}`);
                visit(child);
            }
        }
        // Cargo also compiles conventional `mod flow;` modules without an
        // explicit #[path], including their own cfg(test) test roots.
        const conventional = new RegExp(
            String.raw`^[ \t]*(?:pub(?:\([^)]+\))?[ \t]+)?` +
            String.raw`mod[ \t]+([a-zA-Z_]\w*)[ \t]*;`, "gm",
        );
        for (const match of source.matchAll(conventional)) {
            const before = source.slice(0, match.index).trimEnd();
            const prior = before.split(/\r?\n/).at(-1) ?? "";
            if (/^#\[(?:path|cfg)\b/u.test(prior)) continue;
            const base = resolve(dirname(sourcePath), match[1]);
            const candidates = [base + ".rs", resolve(base, "mod.rs")];
            for (const child of candidates) {
                if (sources?.has(child) || existsSync(child)) {
                    assert.ok(child.startsWith(crateRoot + sep),
                        `source module escapes owning crate: ${child}`);
                    visit(child);
                    break;
                }
            }
        }
        ancestry.delete(sourcePath);
    }
    for (const sourcePath of sourceTargetPaths(manifest, content)) {
        visit(sourcePath);
    }
    return modules;
}

function checkTestTargetGraph(manifests, testRoots, sources) {
    const declaredByFile = new Map();
    const declaredNames = new Set();
    for (const [manifest, content] of manifests) {
        const crate = dirname(manifest).split(sep).at(-1);
        const targets = declaredTargets(manifest, content).map((target) => ({
            name: target.name,
            path: target.path,
        }));
        for (const path of linkedUnitModules(manifest, content, sources)) {
            targets.push({ name: null, path });
        }
        for (const target of targets) {
            assert.ok(
                target.path.startsWith(testsRoot + sep),
                `test target escapes backend tests: ${target.path}`,
            );
            assert.ok(!declaredByFile.has(target.path),
                `duplicate Cargo target: ${target.path}`);
            if (target.name !== null) {
                assert.ok(!declaredNames.has(target.name),
                    `duplicate Cargo test name: ${target.name}`);
                declaredNames.add(target.name);
            }
            const suffix = relative(testsRoot, target.path).split(sep);
            assert.equal(suffix.length, 3);
            assert.equal(suffix[0], crate, "test owned by wrong Cargo crate");
            if (target.name === null) {
                assert.match(suffix[2], /^[a-z0-9_-]+\.rs$/u,
                    "linked unit target must be a Rust source");
            } else {
                assert.equal(suffix[2], "lib.rs");
                const prefix = `${crate}_`.replaceAll("-", "_");
                assert.ok(target.name.startsWith(prefix),
                    "test target identity must name its owning crate");
            }
            declaredByFile.set(target.path, manifest);
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

test("nested explicit modules expose only reachable cfg(test) roots", () => {
    const manifest = resolve(packagesRoot, "example", "Cargo.toml");
    const entry = resolve(dirname(manifest), "composition/lib.rs");
    const nested = resolve(dirname(manifest), "application/lib.rs");
    const path = resolve(testsRoot, "example/composition/lib.rs");
    const content = '[lib]\npath = "composition/lib.rs"\n';
    const link = '#[path = "../application/lib.rs"]\n' +
        'pub mod application;\n';
    const nestedTest = '#[cfg(test)]\n' +
        '#[path = "../../../../tests/backend/example/composition/lib.rs"]\n' +
        'mod tests;\n';
    const sources = new Map([[entry, link], [nested, nestedTest]]);
    checkTestTargetGraph([[manifest, content]], [path], sources);

    // A reference in prose or an unrelated source cannot authorize tests.
    assert.throws(() => checkTestTargetGraph(
        [[manifest, content]], [path],
        new Map([[entry, '// ' + link], [nested, nestedTest]])),
    /not wired/);
    assert.throws(() => checkTestTargetGraph(
        [[manifest, content]], [path],
        new Map([[entry, link], [nested,
            nestedTest.replace("cfg(test)", "cfg(debug_assertions)")]])),
    /not wired/);
    assert.throws(() => checkTestTargetGraph(
        [[manifest, content]], [path],
        new Map([[entry, link], [nested, nestedTest + nestedTest]])),
    /duplicate/);
    assert.throws(() => checkTestTargetGraph(
        [[manifest, content]], [path],
        new Map([[entry, link], [nested,
            '#[path = "../composition/lib.rs"]\nmod parent;\n']])),
    /cycle/);

    // A conventional module remains compiled without a #[path] attribute.
    const flow = resolve(dirname(manifest), "composition/flow.rs");
    const flowTest = resolve(testsRoot, "example/composition/flow.rs");
    const flowSource = '#[cfg(test)]\n' +
        '#[path = "../../../../tests/backend/example/composition/flow.rs"]\n' +
        'mod tests;\n';
    checkTestTargetGraph([[manifest, content]], [flowTest],
        new Map([[entry, "mod flow;\n"], [flow, flowSource]]));
    assert.throws(() => checkTestTargetGraph(
        [[manifest, content]], [flowTest],
        new Map([[entry, "// mod flow;\n"], [flow, flowSource]])),
    /not wired/);
});

test("unit modules count only when linked from a compiled Cargo target", () => {
    const manifest = resolve(packagesRoot, "example", "Cargo.toml");
    const source = resolve(dirname(manifest), "composition/main.rs");
    const path = resolve(testsRoot, "example/composition/lib.rs");
    const content = '[[bin]]\nname = "example"\npath = "composition/main.rs"\n';
    const linked = '#[cfg(test)]\n' +
        '#[path = "../../../../tests/backend/example/composition/lib.rs"]\n' +
        'mod tests;\n';
    checkTestTargetGraph([[manifest, content]], [path],
        new Map([[source, linked]]));
    const unchecked = linked.replace("cfg(test)", "cfg(debug_assertions)");
    assert.throws(() => checkTestTargetGraph([[manifest, content]], [path],
        new Map([[source, unchecked]])), /not wired/);
    assert.throws(() => checkTestTargetGraph([[manifest, content]], [path],
        new Map([[source, linked + linked]])), /duplicate/);

    // A binary's directly linked unit module need not be named lib.rs.
    const printerSource = resolve(dirname(manifest), "composition/print.rs");
    const printerPath = resolve(testsRoot, "example/composition/print.rs");
    const printer = '#[cfg(test)]\n' +
        '#[path = "../../../../tests/backend/example/composition/print.rs"]\n' +
        'mod tests;\n';
    const printerTarget = '[[bin]]\nname = "example_print"\n' +
        'path = "composition/print.rs"\n';
    checkTestTargetGraph([[manifest, printerTarget]], [printerPath],
        new Map([[printerSource, printer]]));
    assert.throws(() => checkTestTargetGraph(
        [[manifest, printerTarget]], [printerPath],
        new Map([[printerSource, printer.replace("cfg(test)",
            "cfg(debug_assertions)")]])), /not wired/);
});
