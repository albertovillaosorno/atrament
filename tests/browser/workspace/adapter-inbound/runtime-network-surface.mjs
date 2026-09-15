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
//   - Source-level regression evidence for the production backend network
//     dependency surface.
// - Must-Not:
//   - Claim packet-level isolation, open sockets, or exercise external network
//     access.
// - Allows:
//   - Inputs: Tracked backend Rust sources and Cargo workspace metadata.
//   - Outputs: Assertions that network primitives and external dependencies
//     stay within the explicitly reviewed first-release surface.
//   - Side effects: Reads repository files and runs local Cargo metadata only.
// - Split-When:
//   - A deliberate non-loopback or separately admitted network adapter exists.
// - Merge-When:
//   - A broader offline-execution fixture subsumes this source-level guard.
// - Summary:
//   - Prevents silent expansion of Atrament's backend network surface.
// - Description:
//   - Keeps standard-library networking confined to the loopback runtime and
//     requires review when registry dependencies change.
// - Usage:
//   - Execute through the repository frontend test script.
// - Defaults:
//   - Only getrandom and unicode-segmentation are reviewed registry
//     dependencies.
//
import assert from "node:assert/strict";
import { readdir, readFile } from "node:fs/promises";
import path from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";
import { fileURLToPath } from "node:url";

const REPOSITORY_ROOT = fileURLToPath(
    new URL("../../../../", import.meta.url),
);
const BACKEND_ROOT = path.join(REPOSITORY_ROOT, "src", "backend");
const LOOPBACK_RUNTIME = path.join(
    BACKEND_ROOT,
    "session-runtime",
    "adapter-inbound",
    "lib.rs",
);

async function rustSources(directory) {
    const sources = [];
    for (const entry of await readdir(directory, { withFileTypes: true })) {
        const child = path.join(directory, entry.name);
        if (entry.isDirectory()) {
            sources.push(...await rustSources(child));
        } else if (entry.isFile() && entry.name.endsWith(".rs")) {
            sources.push(child);
        }
    }
    return sources;
}

test(
    "backend standard-library networking stays in loopback runtime",
    async () => {
    const networkFiles = [];
    for (const sourcePath of await rustSources(BACKEND_ROOT)) {
        const source = await readFile(sourcePath, "utf8");
        if (/\bstd::net(?:::|\s*::|\s*\{)/u.test(source)) {
            networkFiles.push(sourcePath);
        }
    }
    assert.deepEqual(networkFiles, [LOOPBACK_RUNTIME]);
    },
);

test("backend registry dependencies remain explicitly reviewed", () => {
    const cargo = path.join(
        REPOSITORY_ROOT,
        ".dependencies",
        "rust",
        "1.97.1",
        "bin",
        "cargo",
    );
    const result = spawnSync(
        cargo,
        ["metadata", "--format-version", "1", "--no-deps"],
        {
            cwd: REPOSITORY_ROOT,
            encoding: "utf8",
        },
    );
    assert.equal(result.status, 0, result.stderr);
    const metadata = JSON.parse(result.stdout);
    const registryDependencies = [];
    for (const packageValue of metadata.packages) {
        for (const dependency of packageValue.dependencies) {
            if (dependency.source !== null) {
                registryDependencies.push([
                    packageValue.name,
                    dependency.name,
                    dependency.req,
                    dependency.source,
                ]);
            }
        }
    }
    registryDependencies.sort((left, right) =>
        left.join("\0").localeCompare(right.join("\0")),
    );
    assert.deepEqual(registryDependencies, [
        [
            "atrament_session_secret",
            "getrandom",
            "=0.4.3",
            "registry+https://github.com/rust-lang/crates.io-index",
        ],
        [
            "atrament_unicode_grapheme_segmentation",
            "unicode-segmentation",
            "=1.13.3",
            "registry+https://github.com/rust-lang/crates.io-index",
        ],
    ]);
});
