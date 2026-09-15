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
//     requires review when direct or resolved registry dependencies change.
// - Usage:
//   - Execute through the repository frontend test script.
// - Defaults:
//   - Direct declarations are getrandom and unicode-segmentation; the resolved
//     registry set also includes their pinned transitive packages.
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


function usesStandardLibraryNetworking(source) {
    if (/\bstd\s*::\s*net\b/u.test(source)) {
        return true;
    }
    const groupedImports = source.matchAll(
        /\buse\s+std\s*::\s*\{([\s\S]*?)\}\s*;/gu,
    );
    for (const importMatch of groupedImports) {
        if (/\bnet\b\s*(?:::|,|$|\bas\b)/u.test(importMatch[1])) {
            return true;
        }
    }
    return false;
}

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

test("standard-library network detector covers equivalent imports", () => {
    for (const source of [
        "use std::net::TcpListener;",
        "use std :: net :: TcpStream;",
        "use std::{io, net::TcpListener};",
        "use std::{io::{Read, Write}, net::{TcpListener, TcpStream}};",
        "use std::{fmt, net as network};",
    ]) {
        assert.equal(usesStandardLibraryNetworking(source), true, source);
    }
    for (const source of [
        "use std::{fmt, io};",
        "use crate::net::LoopbackOnly;",
        "let internet = false;",
    ]) {
        assert.equal(usesStandardLibraryNetworking(source), false, source);
    }
});

test(
    "backend standard-library networking stays in loopback runtime",
    async () => {
    const networkFiles = [];
    for (const sourcePath of await rustSources(BACKEND_ROOT)) {
        const source = await readFile(sourcePath, "utf8");
        if (usesStandardLibraryNetworking(source)) {
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
        [
            "metadata",
            "--format-version",
            "1",
            "--locked",
            "--offline",
        ],
        {
            cwd: REPOSITORY_ROOT,
            encoding: "utf8",
        },
    );
    assert.equal(result.status, 0, result.stderr);
    const metadata = JSON.parse(result.stdout);
    const workspaceIds = new Set(metadata.workspace_members);
    const registryDependencies = [];
    const resolvedRegistryPackages = [];
    for (const packageValue of metadata.packages) {
        if (packageValue.source !== null) {
            resolvedRegistryPackages.push([
                packageValue.name,
                packageValue.version,
                packageValue.source,
            ]);
        }
        if (!workspaceIds.has(packageValue.id)) {
            continue;
        }
        for (const dependency of packageValue.dependencies) {
            if (dependency.source !== null) {
                registryDependencies.push({
                    defaultFeatures: dependency.uses_default_features,
                    features: dependency.features,
                    kind: dependency.kind,
                    name: dependency.name,
                    optional: dependency.optional,
                    owner: packageValue.name,
                    rename: dependency.rename,
                    requirement: dependency.req,
                    source: dependency.source,
                    target: dependency.target,
                });
            }
        }
    }
    const comparePackages = (left, right) =>
        left.join("\0").localeCompare(right.join("\0"));
    registryDependencies.sort((left, right) =>
        `${left.owner}\0${left.name}`.localeCompare(
            `${right.owner}\0${right.name}`,
        ),
    );
    resolvedRegistryPackages.sort(comparePackages);
    assert.deepEqual(registryDependencies, [
        {
            defaultFeatures: true,
            features: [],
            kind: null,
            name: "getrandom",
            optional: false,
            owner: "atrament_session_secret",
            rename: null,
            requirement: "=0.4.3",
            source: "registry+https://github.com/rust-lang/crates.io-index",
            target: null,
        },
        {
            defaultFeatures: true,
            features: [],
            kind: null,
            name: "unicode-segmentation",
            optional: false,
            owner: "atrament_unicode_grapheme_segmentation",
            rename: null,
            requirement: "=1.13.3",
            source: "registry+https://github.com/rust-lang/crates.io-index",
            target: null,
        },
    ]);
    assert.deepEqual(resolvedRegistryPackages, [
        [
            "cfg-if",
            "1.0.4",
            "registry+https://github.com/rust-lang/crates.io-index",
        ],
        [
            "getrandom",
            "0.4.3",
            "registry+https://github.com/rust-lang/crates.io-index",
        ],
        [
            "libc",
            "0.2.189",
            "registry+https://github.com/rust-lang/crates.io-index",
        ],
        [
            "r-efi",
            "6.0.0",
            "registry+https://github.com/rust-lang/crates.io-index",
        ],
        [
            "unicode-segmentation",
            "1.13.3",
            "registry+https://github.com/rust-lang/crates.io-index",
        ],
    ]);
});
