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
//   - Source-level regression evidence for backend network, process-launch,
//     and filesystem-mutation authority.
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
//   - Confines network, process and direct filesystem mutation APIs to their
//     admitted adapters and registry dependency changes to review.
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

const BROWSER_LAUNCH = path.join(
    BACKEND_ROOT,
    "browser-launch",
    "adapter-outbound",
    "lib.rs",
);

const FILESYSTEM_COMMIT = path.join(
    BACKEND_ROOT,
    "profile-file-commit",
    "adapter-outbound",
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

// A child process can bypass std::net through an external program. Require
// explicit review for all standard-library process authority outside the
// platform browser-launch adapter, including grouped imports and constructors.
function usesStandardLibraryProcessAuthority(source) {
    if (/\bstd\s*::\s*process\b/u.test(source)
        || /\bCommand\s*::\s*new\s*\(/u.test(source)) {
        return true;
    }
    const groupedImports = source.matchAll(
        /\buse\s+std\s*::\s*\{([\s\S]*?)\}\s*;/gu,
    );
    for (const importMatch of groupedImports) {
        if (/\bprocess\b\s*(?:::|,|$|\bas\b)/u.test(importMatch[1])) {
            return true;
        }
    }
    return false;
}

// Short-lived writes can vanish before a process descriptor snapshot. This
// conservative source guard covers the ordinary direct Rust filesystem write
// constructors and this workspace's Linux profile-commit mutation operations.
// It does not claim to analyze arbitrary FFI or every indirect file effect.
const MUTATING_FS_FUNCTIONS = new Set([
    "copy", "create_dir", "create_dir_all", "hard_link", "remove_dir",
    "remove_dir_all", "remove_file", "rename", "set_permissions", "symlink",
    "write",
]);

function usesDirectFilesystemMutationAuthority(source) {
    // Also recognize imports such as `use std::fs::write` rather than only
    // call sites, so an alias cannot evade the source review boundary.
    const paths = source.matchAll(/\bfs\s*::\s*([a-z_]+)\b/gu);
    for (const match of paths) {
        if (MUTATING_FS_FUNCTIONS.has(match[1])) {
            return true;
        }
    }
    const grouped = source.matchAll(/\bfs\s*::\s*\{([^}]*)\}/gu);
    for (const match of grouped) {
        if (match[1].split(",").some((field) =>
            MUTATING_FS_FUNCTIONS.has(field.trim().split(/\s+/u)[0]))) {
            return true;
        }
    }
    const unixMutator =
        /\b(?:linkat|unlinkat|renameat|renameat2|mkdirat|mknodat)\s*\(/u;
    return /\bFile\s*::\s*(?:create|create_new|options)\s*\(/u.test(source)
        || /\bOpenOptions\b/u.test(source)
        || /\bO_(?:TMPFILE|CREAT|TRUNC)\b/u.test(source)
        || unixMutator.test(source);
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

test("standard-library process detector covers equivalent imports", () => {
    for (const source of [
        "use std::process::Command;",
        "use std :: process :: Command;",
        "use std::{io, process::Command};",
        "use std::{io::{Read, Write}, process::{Child, Command}};",
        "use std::{fmt, process as operating_system_process};",
        "std::process::Command::new(\"curl\");",
        "Command::new(\"curl\");",
    ]) {
        assert.equal(usesStandardLibraryProcessAuthority(source), true,
            source);
    }
    for (const source of [
        "use std::{fmt, io};",
        "use crate::process::Policy;",
        "let process = false;",
    ]) {
        assert.equal(usesStandardLibraryProcessAuthority(source), false,
            source);
    }
});

test("backend process authority stays in browser launch", async () => {
    const spawningFiles = [];
    for (const sourcePath of await rustSources(BACKEND_ROOT)) {
        const source = await readFile(sourcePath, "utf8");
        if (usesStandardLibraryProcessAuthority(source)) {
            spawningFiles.push(sourcePath);
        }
    }
    assert.deepEqual(spawningFiles, [BROWSER_LAUNCH]);
});

test("direct filesystem mutation detector covers write authority", () => {
    for (const source of [
        'std::fs::write("state", b"private");',
        'fs :: remove_file ("state");',
        'std::fs::create_dir_all("state");',
        'File::create("state");',
        'File::create_new("state");',
        'use std::fs::write as hidden_write;',
        'use std::{fs::{read, write as hidden_write}};',
        'use std::os::unix::fs::symlink;',
        'File::options().write(true);',
        'use std::fs::OpenOptions;',
        'OFlag::O_TMPFILE | OFlag::O_RDWR',
        'linkat(&file, "", &parent, "out", flags);',
        'renameat2(&parent, "old", &parent, "new", flags);',
        'unlinkat(Some(&parent), "old", flags);',
    ]) {
        assert.equal(usesDirectFilesystemMutationAuthority(source), true,
            source);
    }
    for (const source of [
        'use std::fs::{File, Metadata};',
        'File::open("state");',
        'fs::read("state");',
        'std::fs::metadata("state");',
        'OFlag::O_RDONLY | OFlag::O_CLOEXEC',
    ]) {
        assert.equal(usesDirectFilesystemMutationAuthority(source), false,
            source);
    }
});

test("direct filesystem mutation stays in explicit profile commit",
    async () => {
    const mutationFiles = [];
    for (const sourcePath of await rustSources(BACKEND_ROOT)) {
        const source = await readFile(sourcePath, "utf8");
        if (usesDirectFilesystemMutationAuthority(source)) {
            mutationFiles.push(sourcePath);
        }
    }
    assert.deepEqual(mutationFiles, [FILESYSTEM_COMMIT]);
});

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
            owner: "atrament_profile_file_commit",
            rename: null,
            requirement: "=0.4.3",
            source: "registry+https://github.com/rust-lang/crates.io-index",
            target: 'cfg(target_os = "linux")',
        },
        {
            defaultFeatures: false,
            features: ["fs"],
            kind: null,
            name: "nix",
            optional: false,
            owner: "atrament_profile_file_commit",
            rename: null,
            requirement: "=0.31.3",
            source: "registry+https://github.com/rust-lang/crates.io-index",
            target: 'cfg(target_os = "linux")',
        },
        {
            defaultFeatures: false,
            features: ["fs"],
            kind: null,
            name: "nix",
            optional: false,
            owner: "atrament_profile_selected_file_open",
            rename: null,
            requirement: "=0.31.3",
            source: "registry+https://github.com/rust-lang/crates.io-index",
            target: 'cfg(target_os = "linux")',
        },
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
            "bitflags",
            "2.13.2",
            "registry+https://github.com/rust-lang/crates.io-index",
        ],
        [
            "cfg_aliases",
            "0.2.2",
            "registry+https://github.com/rust-lang/crates.io-index",
        ],
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
            "nix",
            "0.31.3",
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
