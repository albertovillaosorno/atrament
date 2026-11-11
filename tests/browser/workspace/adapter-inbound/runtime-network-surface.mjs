// Copyright:
//   - Copyright © 2026 Alberto Villa Osorno.
// SPDX-License-Identifier:
//   - MIT
// Confidential:
//   - true
// License-File:
//   - LICENSE
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
import { realpathSync } from "node:fs";
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

// Paper rendering remains in application; only composition owns argv,
// stdout and caller-selected file effects. Inventory both audited roots.
const PAPER_APPLICATION = path.join(
    BACKEND_ROOT, "notebook-paper-cli", "application", "lib.rs",
);
const PAPER_CRATE_ROOT = path.join(
    BACKEND_ROOT, "notebook-paper-cli", "composition", "lib.rs",
);
const PAPER_CLI = path.join(
    BACKEND_ROOT, "notebook-paper-cli", "composition", "cli.rs",
);
const PROFILE_FLOW = path.join(
    BACKEND_ROOT, "profile-editor-cli", "composition", "flow.rs",
);
// This separate opt-in CLI is the only non-browser process-launch authority.
const PRINT_CLI = path.join(
    BACKEND_ROOT, "notebook-paper-cli", "composition", "print.rs",
);
const PROFILE_EDITOR_CLI = path.join(
    BACKEND_ROOT, "profile-editor-cli", "composition", "main.rs",
);
const NOTEBOOK_PROFILE = path.join(
    BACKEND_ROOT, "notebook-profile", "composition", "lib.rs",
);

const BROWSER_LAUNCH = path.join(
    BACKEND_ROOT,
    "browser-launch",
    "adapter-outbound",
    "lib.rs",
);

const RUNTIME_BOOTSTRAP = path.join(
    BACKEND_ROOT,
    "runtime-bootstrap",
    "composition",
    "main.rs",
);

const FILESYSTEM_COMMIT = path.join(
    BACKEND_ROOT,
    "profile-file-commit",
    "adapter-outbound",
    "lib.rs",
);


// Comments are lexical whitespace in Rust, including between a module name
// and `::`. Ignore them without damaging ordinary or raw string literals;
// Rust block comments nest, so a non-greedy regex is insufficient. This is a
// conservative source review, not a replacement for a Rust syntax parser.
function replaceRustCommentsWithWhitespace(source) {
    let normalized = "";
    let cursor = 0;
    while (cursor < source.length) {
        const remaining = source.slice(cursor);
        const raw = /^(?:b?r)(#+)?"/u.exec(remaining);
        if (raw !== null) {
            const delimiter = `"${raw[1] ?? ""}`;
            const end = source.indexOf(delimiter, cursor + raw[0].length);
            const next = end < 0 ? source.length : end + delimiter.length;
            normalized += source.slice(cursor, next);
            cursor = next;
            continue;
        }
        if (source[cursor] === '"') {
            const start = cursor++;
            while (cursor < source.length) {
                if (source[cursor] === "\\") {
                    cursor += 2;
                } else if (source[cursor++] === '"') {
                    break;
                }
            }
            normalized += source.slice(start, cursor);
            continue;
        }
        if (remaining.startsWith("//")) {
            const end = source.indexOf("\n", cursor + 2);
            cursor = end < 0 ? source.length : end;
            normalized += " ";
            continue;
        }
        if (remaining.startsWith("/*")) {
            let depth = 1;
            const start = cursor;
            cursor += 2;
            while (cursor < source.length && depth > 0) {
                if (source.startsWith("/*", cursor)) {
                    depth += 1;
                    cursor += 2;
                } else if (source.startsWith("*/", cursor)) {
                    depth -= 1;
                    cursor += 2;
                } else {
                    cursor += 1;
                }
            }
            const removed = source.slice(start, cursor);
            const newlines = (removed.match(/\n/gu) ?? []).length;
            normalized += ` ${"\n".repeat(newlines)}`;
            continue;
        }
        normalized += source[cursor];
        cursor += 1;
    }
    return normalized;
}

// Rust also accepts `r#` on ordinary identifiers, not just keywords.
// Strip that prefix after comment normalization, preserving raw strings.
function normalizeRustRawIdentifiers(source) {
    return replaceRustCommentsWithWhitespace(source)
        .replace(/\br#(?=[A-Za-z_])/gu, "");
}

test("Rust source scan treats nested comments as lexical whitespace", () => {
    const source = [
        "use std/* outer /* inner */ rest */::net::TcpListener;",
        "use std// line comment\n::process::Command;",
        'let url = "http://127.0.0.1/* not a comment */";',
        'let raw = r###"// not a comment /* either */"###;',
        'let bytes = br#"/* not a comment */"#;',
        "/* use std::fs::write; */",
        "// std::fs::create_dir_all",
    ].join("\n");
    const normalized = normalizeRustRawIdentifiers(source);
    assert.match(normalized, /std\s+::net::TcpListener/u);
    assert.match(normalized, /std\s+::process::Command/u);
    assert.equal(normalized.includes("/* not a comment */"), true);
    assert.equal(normalized.includes('r###"// not a comment'), true);
    assert.equal(normalized.includes('br#"/* not a comment'), true);
    assert.equal(normalized.includes("fs::write"), false);
    assert.equal(normalized.includes("fs::create_dir_all"), false);
});

function usesStandardLibraryNetworking(source) {
    source = normalizeRustRawIdentifiers(source);
    if (/\bstd\s*::\s*net\b/u.test(source)) {
        return true;
    }
    const groupedImports = source.matchAll(
        /\buse\s+(?:::\s*)?std\s*::\s*\{([\s\S]*?)\}\s*;/gu,
    );
    for (const importMatch of groupedImports) {
        if (/\bnet\b\s*(?:::|,|$|\bas\b)/u.test(importMatch[1])) {
            return true;
        }
    }
    return false;
}

// Unix-domain sockets can transfer private state to unreviewed local IPC
// endpoints without using std::net. Keep this platform path explicit.
function usesUnixDomainNetworking(source) {
    source = normalizeRustRawIdentifiers(source);
    if (/\bstd\s*::\s*os\s*::\s*unix\s*::\s*net\b/u.test(source)
        || /\bUnix(?:Stream|Listener|Datagram)\s*::/u.test(source)) {
        return true;
    }
    // Grouped Rust imports may place `os`, `unix`, and `net` in nested
    // braces; normalize only std-rooted use declarations for review.
    for (const match of source.matchAll(/\buse\s+(?:::\s*)?std\s*::[^;]+;/gu)) {
        const qualified = match[0].replace(/[{}\s]/gu, "")
            .replace(/:{3,}/gu, "::");
        if (qualified.includes("std::os::unix::net")) {
            return true;
        }
    }
    return false;
}

// Renaming the std or core root can hide native operations and compiler
// macros from the narrow source detectors. Review any such root alias.
function renamesStandardLibraryRoot(source) {
    source = normalizeRustRawIdentifiers(source);
    const directAlias = /\buse\s+(?:::\s*)?(?:std|core)\s+as\s+\S+/u;
    const externAlias = /\bextern\s+crate\s+(?:std|core)\s+as\s+\S+/u;
    if (directAlias.test(source) || externAlias.test(source)) {
        return true;
    }
    // A root-level `self as` inside a grouped import also renames std.
    // Ignore nested `io::{self as alias}`: that does not rename std itself.
    for (const match of source.matchAll(
        /\buse\s+(?:::\s*)?(?:std|core)\s*::\s*\{/gu,
    )) {
        let depth = 1;
        let item = "";
        for (let index = match.index + match[0].length;
            index < source.length && depth > 0; index += 1) {
            const character = source[index];
            if (character === "{") {
                depth += 1;
            } else if (character === "}") {
                depth -= 1;
            }
            if ((depth === 1 && character === ",") || depth === 0) {
                if (/^\s*self\s+as\s+\S+\s*$/u.test(item)) {
                    return true;
                }
                item = "";
            } else if (depth === 1) {
                item += character;
            }
        }
    }
    return false;
}

// A generic std::io::Write import can mutate an already-open File. Review
// new stream-writing sites regardless of whether their actual receiver is a
// file, a socket, or an in-memory buffer.
function usesStandardLibraryStreamWriting(source) {
    source = normalizeRustRawIdentifiers(source);
    // Writing and flushing a caller-supplied File do not require a new open.
    // std::io::copy delegates writes without importing the Write trait.
    const writeMethod = String.raw`(?:write|write_all|write_vectored|` +
        String.raw`write_fmt|flush)`;
    const writing = writeMethod + String.raw`\s*\(`;
    // Rust permits method items (and fully-qualified trait items) without
    // a local call; the value may be invoked later under a different name.
    const traitItem = new RegExp(
        String.raw`\bWrite\s*::\s*` + writeMethod + String.raw`\b`,
        "u",
    );
    const qualifiedItem = new RegExp(
        String.raw`<[^;{}]*\bas\s+(?:::)?std\s*::\s*io\s*::` +
            String.raw`\s*Write\s*>\s*::\s*` + writeMethod +
            String.raw`\b`, "u",
    );
    if (new RegExp(String.raw`\.\s*` + writing, "u").test(source)
        || traitItem.test(source) || qualifiedItem.test(source)
        || /\bio\s*::\s*(?:copy\b|\{[^}]*\bcopy\b)/u.test(source)) {
        return true;
    }
    for (const match of source.matchAll(
        /\buse\s+(?:::\s*)?std\s*::[^;]+;/gu,
    )) {
        const imported = match[0];
        if (/\bio\s*::\s*Write\b/u.test(imported)
            || /\bio\s*::\s*\{[^}]*\bWrite\b/u.test(imported)
            || /\bio\s+as\b/u.test(imported)
            || /\bio\s*::\s*\{[^}]*\bself\s+as\b/u
                .test(imported)) {
            return true;
        }
    }
    return false;
}

// A child process can bypass std::net through an external program, while a
// background Rust worker can outlive the observed single-thread snapshots.
// Keep both execution authorities within the reviewed browser launcher.
function usesStandardLibraryProcessAuthority(source) {
    source = normalizeRustRawIdentifiers(source);
    // Command construction and execution methods can be captured as items,
    // then invoked later without retaining the name of the original method.
    const commandItem = new RegExp(
        String.raw`\bCommand\s*::\s*(?:new|spawn|status|output|` +
            String.raw`exec|pre_exec|before_exec)\b`, "u",
    );
    if (/\bstd\s*::\s*process\b/u.test(source)
        || commandItem.test(source)
        // A scoped worker can be created through a captured `Scope::spawn`
        // function item, even if this module never calls `.spawn(...)`.
        || /\b(?:std\s*::\s*)?thread\s*::\s*(?:spawn|scope|Builder|Scope)\b/u
            .test(source)
        || /\b(?:Builder|Scope)\s*::\s*spawn(?:_scoped|_unchecked)?\b/u
            .test(source)) {
        return true;
    }
    // A previously constructed Command can spawn through delegated methods
    // without a new Command::new or std::process reference in this module.
    if (/\.\s*(?:spawn|exec|status|output|pre_exec|before_exec)\s*\(/u
        .test(source)
        || /\.\s*spawn_(?:scoped|unchecked)\s*\(/u.test(source)
        // A generic trait bound can use the platform process extension
        // directly without importing it. Capture as a method item also
        // requires no adjacent call, and can execute later under an alias.
        || new RegExp(
            String.raw`\bstd\s*::\s*os\s*::\s*(?:unix|windows)\s*::` +
                String.raw`\s*process\s*::\s*CommandExt\b`, "u",
        ).test(source)
        || /\bCommandExt\s*::\s*(?:exec|pre_exec|before_exec)\b/u
            .test(source)) {
        return true;
    }
    const scopedThreadImport = new RegExp(
        String.raw`\bthread\s*::\s*\{[^;]*` +
            String.raw`(?:\b(?:spawn|scope|Builder|Scope)\b|\*)`, "u",
    );
    for (const match of source.matchAll(
        /\buse\s+(?:::\s*)?std\s*::\s*([^;]+);/gu,
    )) {
        // Keep raw braces for grouped thread imports, then flatten for the
        // existing process-extension checks below.
        const threadImport = /\bthread\s+as\b/u.test(match[1])
            || /\bthread\s*::\s*\{[^;]*\bself\s+as\b/u
                .test(match[1])
            || /\bthread\s*::\s*(?:spawn|scope|Builder|Scope|\*)/u
                .test(match[1])
            || scopedThreadImport.test(match[1]);
        const imported = match[1].replace(/[{}\s]/gu, "");
        // A renamed os root can hide a platform process extension.
        if (threadImport
            || /(?:^|,)process(?:::|as|,|$)/u.test(imported)
            || /(?:^|,)os(?:as\w+|(?=,|$))/u.test(imported)
            || /(?:^|,)os::selfas\w+/u.test(imported)
            || /\bos::(?:unix|windows)(?:as\w+|::selfas\w+|::\*|(?=,|$))/u
                .test(imported)
            || /(?:^|,)os::\*/u.test(imported)
            || /\bos::(?:unix|windows)::process(?:as\w+|(?:,|$))/u
                .test(imported)
            || /\bos::(?:unix|windows)::process::(?:CommandExt|self|\*)/u
                .test(imported)) {
            return true;
        }
    }
    return false;
}

// The selected-parent and CWD anchors assume no other backend authority
// changes process-wide pathname resolution or the default creation mask.
// These global effects require a dedicated zero-owner source inventory.
function usesProcessWidePathAuthority(source) {
    source = normalizeRustRawIdentifiers(source);
    // The pinned nix fs feature also exports Linux pivot_root, changing the
    // process filesystem root without std::env::set_current_dir or chroot.
    const mutation = /\b(?:chdir|fchdir|chroot|pivot_root|umask)\b/u;
    if (/\b(?:set_current_dir|chdir|fchdir|chroot|pivot_root|umask)\s*\(/u
        .test(source)
        || /\b(?:std\s*::\s*)?env\s*::\s*set_current_dir\b/u.test(source)) {
        return true;
    }
    for (const match of source.matchAll(
        /\buse\s+(?:::\s*)?std\s*::\s*([^;]+);/gu,
    )) {
        const imported = match[1];
        if (/\benv\s+as\b/u.test(imported)
            || /\benv\s*::\s*(?:\{[^;]*\b)?set_current_dir\b/u.test(imported)) {
            return true;
        }
    }
    for (const match of source.matchAll(
        /\bnix\s*::\s*(?:unistd|sys\s*::\s*stat)\s*::\s*([a-z_]+)\b/gu,
    )) {
        if (mutation.test(match[1])) {
            return true;
        }
    }
    for (const match of source.matchAll(
        /\buse\s+(?:::\s*)?nix\s*::\s*([^;]+);/gu,
    )) {
        const imported = match[1];
        if (/\b(?:unistd|stat)\s+as\b/u.test(imported)
            || (/\b(?:unistd|stat)\b/u.test(imported)
                && mutation.test(imported))) {
            return true;
        }
    }
    return false;
}

// Process-wide environment mutation can replace the launcher's PATH, GUI
// session markers, or other inherited subprocess variables. The runtime only
// reads its environment; a new writer must undergo explicit privacy review.
function usesProcessWideEnvironmentMutation(source) {
    source = normalizeRustRawIdentifiers(source);
    // Unsafe environment mutators can be captured as function items under
    // a later name. Do not require a call next to the authority reference.
    // Keep crate-local `crate::env::remove_var` out of this std review.
    const stdEnvironmentItem = new RegExp(
        String.raw`(?<![\w:])(?:::)?(?:std\s*::\s*)?env\s*::\s*` +
            String.raw`(?:set_var|remove_var)\b`, "u",
    );
    if (/\b(?:set_var|remove_var)\s*\(/u.test(source)
        || stdEnvironmentItem.test(source)) {
        return true;
    }
    for (const match of source.matchAll(
        /\buse\s+(?:::\s*)?std\s*::\s*([^;]+);/gu,
    )) {
        const imported = match[1];
        if (/\benv\s+as\b/u.test(imported)
            || /\benv\s*::\s*\{[^;]*\bself\s+as\b/u.test(imported)
            || /\benv\s*::\s*(?:\{[^;]*\b)?(?:set_var|remove_var)\b/u
                .test(imported)) {
            return true;
        }
    }
    return false;
}

// Environment and argv values can contain private host tokens or paths.
// Confine reads to the two GUI markers and explicit CLI argument inspection.
function unreviewedAmbientProcessInputReads(source, sourcePath) {
    source = normalizeRustRawIdentifiers(source);
    // This stable std API exposes current CPU parallelism without nix or env.
    // Reject even captured function items and renamed imports for review.
    if (/\bthread\s*::\s*available_parallelism\b/u.test(source)) {
        return true;
    }
    for (const match of source.matchAll(
        /\buse\s+(?:::\s*)?std\s*::\s*([^;]+);/gu,
    )) {
        const imported = match[1];
        if (/\bthread\s+as\b/u.test(imported)
            || /\bthread\s*::\s*\{?[^;]*\bavailable_parallelism\b/u
                .test(imported)
            || /\bthread\s*::\s*\{[^;]*\bself\s+as\b/u
                .test(imported)) {
            return true;
        }
    }
    // Pinned nix exposes Linux machine load, memory, uptime and process count
    // even with only `fs` enabled. Such ambient host-status reads have no
    // admitted owner; review direct, captured and renamed module access.
    if (/\bnix\s*::\s*sys\s*::\s*sysinfo\b/u.test(source)) {
        return true;
    }
    for (const match of source.matchAll(
        /\buse\s+(?:::\s*)?nix\s*::\s*([^;]+);/gu,
    )) {
        if (/\bsysinfo\b/u.test(match[1])) {
            return true;
        }
    }
    const readers = String.raw`(?:var|var_os|vars|vars_os|current_dir|` +
        String.raw`current_exe|home_dir|temp_dir)`;
    // `var::<&str>`, `current_dir`, and the argument iterator functions can
    // be stored and invoked later without a call token in this module.
    const anyRead = new RegExp(
        String.raw`\benv\s*::\s*` + readers + String.raw`\b`,
        "gu",
    );
    const importedRead = new RegExp(
        String.raw`\buse\s+(?:::)?std\s*::[^;]*\benv\s*::` +
            String.raw`\s*(?:\{[^;]*\b)?` + readers + String.raw`\b`,
        "u",
    );
    // Host CLI arguments can contain explicitly selected private paths.
    // The entrypoint owns precisely one OsString iterator, skipping argv[0].
    const importedArguments = new RegExp(
        String.raw`\buse\s+(?:::)?std\s*::[^;]*\benv\s*::` +
            String.raw`\s*(?:\{[^;]*\b)?(?:args|args_os)\b`,
        "u",
    );
    if (importedRead.test(source) || importedArguments.test(source)) {
        return true;
    }
    const argumentsRead = [...source.matchAll(
        /\b(?:std\s*::\s*)?env\s*::\s*(args|args_os)\b/gu,
    )];
    const reads = [...source.matchAll(anyRead)];
    if (sourcePath === RUNTIME_BOOTSTRAP) {
        return reads.length !== 0 || argumentsRead.length !== 1
            || argumentsRead[0][1] !== "args_os"
            || !/\benv\s*::\s*args_os\s*\(\s*\)\s*\.\s*skip\s*\(\s*1\s*\)/u
                .test(source);
    }
    // Both native CLI entrypoints read only their own argv once.
    if (sourcePath === PAPER_CLI || sourcePath === PRINT_CLI
        || sourcePath === PROFILE_EDITOR_CLI) {
        return reads.length !== 0 || argumentsRead.length !== 1
            || argumentsRead[0][1] !== "args"
            || !/\benv\s*::\s*args\s*\(\s*\)\s*\.\s*skip\s*\(\s*1\s*\)/u
                .test(source);
    }
    if (argumentsRead.length > 0) {
        return true;
    }
    if (sourcePath !== BROWSER_LAUNCH) {
        return reads.length > 0;
    }
    const markers = [...source.matchAll(
        /\benv\s*::\s*var_os\s*\(\s*"([^"]+)"\s*\)/gu,
    )].map((match) => match[1]).sort();
    return reads.length !== 2 || JSON.stringify(markers) !==
        JSON.stringify(["DISPLAY", "WAYLAND_DISPLAY"]);
}

test("ambient process input guard rejects unreviewed reads", () => {
    const privateModule = path.join(BACKEND_ROOT, "session-secret",
        "adapter-outbound", "lib.rs");
    const accepted = [
        'env::var_os("DISPLAY")',
        'env::var_os("WAYLAND_DISPLAY")',
    ].join(";\n");
    assert.equal(unreviewedAmbientProcessInputReads(accepted, BROWSER_LAUNCH),
        false);
    assert.equal(unreviewedAmbientProcessInputReads(
        `${accepted}; use std::{fmt, io, thread}; thread::sleep(duration);`,
        BROWSER_LAUNCH), false);
    for (const [source, file] of [
        ['std::env::var("PRIVATE_KEY")', privateModule],
        // Available CPU parallelism is ambient host configuration data.
        ['std::thread::available_parallelism()', privateModule],
        ['thread::available_parallelism()', privateModule],
        ['let read = std::thread::available_parallelism;', privateModule],
        ['use std::thread::available_parallelism as count;', privateModule],
        ['use std::thread::{available_parallelism as count};',
            privateModule],
        ['use ::std::{thread::{available_parallelism as count}};',
            privateModule],
        ['use r#std::r#thread::r#available_parallelism as count;',
            privateModule],
        ['std/**/::thread::available_parallelism()', privateModule],
        ['use std::thread as host_threads;', privateModule],
        ['use std::{thread as host_threads};', privateModule],
        ['nix::sys::sysinfo::sysinfo()', privateModule],
        ['let read = nix::sys::sysinfo::sysinfo;', privateModule],
        ['use nix::sys::sysinfo as host_info;', privateModule],
        ['use nix::sys::sysinfo::sysinfo as host_probe;', privateModule],
        ['use ::nix::{sys::{sysinfo as host_info}};', privateModule],
        ['use r#nix::r#sys::r#sysinfo as host_info;', privateModule],
        ['nix/**/::sys::sysinfo::sysinfo()', privateModule],
        ['std::env::current_dir()', privateModule],
        ['let get = std::env::current_dir;', privateModule],
        ['let get = std::env::var::<&str>;', privateModule],
        ['let get = env::vars_os;', privateModule],
        ['let get = env::temp_dir;', privateModule],
        ['let get = env::args_os;', privateModule],
        [`${accepted}; let get = env::var_os::<&str>;`, BROWSER_LAUNCH],
        ['env::args_os().skip(1); let get = env::args_os;',
            RUNTIME_BOOTSTRAP],
        ['std::env::current_exe()', privateModule],
        ['std::env::home_dir()', privateModule],
        ['std::env::temp_dir()', privateModule],
        ['use std::env::temp_dir as read_path;', privateModule],
        ['use ::std::{env::{current_exe as read_path}};', privateModule],
        ['use r#std::r#env::r#home_dir as read_path;', privateModule],
        ['env::current_dir()', RUNTIME_BOOTSTRAP],
        ['env::temp_dir()', BROWSER_LAUNCH],
        ['std/**/::env::var("PRIVATE_KEY")', privateModule],
        ['env::var_os("HOME")', privateModule],
        ['std::env::vars()', privateModule],
        ['env::vars_os()', privateModule],
        ['use std::env::var as read_key;', privateModule],
        ['use ::std::{env::{vars_os as leaked}};', privateModule],
        ['use r#std::r#env::r#var_os as read_key;', privateModule],
        ['env::var_os("HOME")', BROWSER_LAUNCH],
        ['env::var_os("DISPLAY"); env::var("SECRET")',
            BROWSER_LAUNCH],
    ]) {
        assert.equal(unreviewedAmbientProcessInputReads(source, file), true,
            source);
    }
    assert.equal(unreviewedAmbientProcessInputReads(
        'env::args_os().skip(1)', privateModule), true);
    assert.equal(unreviewedAmbientProcessInputReads(
        'std::env::split_paths(paths)', privateModule), false);
    assert.equal(unreviewedAmbientProcessInputReads(
        'env::args_os().skip(1)', RUNTIME_BOOTSTRAP), false);
    for (const [source, file] of [
        ['env::args()', privateModule],
        ['std::env::args_os()', privateModule],
        ['use std::env::args as read_cli;', privateModule],
        ['use ::std::{env::{args_os as leak}};', privateModule],
        ['use r#std::r#env::r#args_os as read_cli;', privateModule],
        ['env::args_os()', RUNTIME_BOOTSTRAP],
        ['env::args_os().skip(0)', RUNTIME_BOOTSTRAP],
        ['env::args_os().skip(1); env::args()', RUNTIME_BOOTSTRAP],
    ]) {
        assert.equal(unreviewedAmbientProcessInputReads(source, file), true,
            source);
    }
});

test("backend ambient input reads stay in reviewed adapters", async () => {
    const unreviewed = [];
    for (const sourcePath of await rustSources(BACKEND_ROOT)) {
        const source = await readFile(sourcePath, "utf8");
        if (unreviewedAmbientProcessInputReads(source, sourcePath)) {
            unreviewed.push(path.relative(BACKEND_ROOT, sourcePath));
        }
    }
    assert.deepEqual(unreviewed, [],
        "environment, argv, host-path or host-status reads need " +
            "privacy review");
});

// Locked nix::fcntl can open read-only files, but also supports O_RDWR,
// O_CREAT, O_TRUNC and anonymous writable staging. Scope *all* fcntl usage
// to the selected-file reader and explicit commit adapter; import-only tests
// cannot prove how a future caller configures its descriptor flags.
function usesNixFcntlAuthority(source) {
    source = normalizeRustRawIdentifiers(source);
    if (/\bnix\s*::\s*fcntl\b/u.test(source)) {
        return true;
    }
    // Nested grouped imports can omit `nix::` before an inner fcntl path.
    for (const match of source.matchAll(
        /\buse\s+(?:::\s*)?nix\s*::\s*\{([^;]+);/gu,
    )) {
        if (/\bfcntl\b/u.test(match[1])) {
            return true;
        }
    }
    return false;
}

// Rust `type Host = File` aliases preserve associated open/create methods
// but erase their original type name from a later `Host::open` source scan.
// Without Rust type resolution, even an unqualified File/Path/DirEntry alias
// needs review before it can hide host reads or filesystem mutations.
const FILE_TYPE_ALIAS = new RegExp(
    String.raw`\btype\s+[\p{ID_Start}_][\p{ID_Continue}]*\s*=\s*` +
        String.raw`(?:::)?(?:(?:std\s*::\s*)?fs\s*::\s*)?File\b`,
    "u",
);
const PATH_QUERY_TYPE_ALIAS = new RegExp(
    String.raw`\btype\s+[\p{ID_Start}_][\p{ID_Continue}]*\s*=\s*` +
        String.raw`(?:::)?(?:` +
        String.raw`(?:(?:std\s*::\s*)?fs\s*::\s*)?DirEntry|` +
        String.raw`(?:(?:std\s*::\s*)?path\s*::\s*)?Path(?:Buf)?` +
        String.raw`)\b`,
    "u",
);

// Short-lived writes can vanish before a process descriptor snapshot. This
// conservative source guard covers the ordinary direct Rust filesystem write
// constructors and this workspace's Linux profile-commit mutation operations.
// It does not claim to analyze arbitrary FFI or every indirect file effect.
const MUTATING_FS_FUNCTIONS = new Set([
    "chown", "chroot", "copy", "create_dir", "create_dir_all", "fchown",
    "hard_link", "lchown", "remove_dir",
    "remove_dir_all", "remove_file", "rename", "set_permissions", "symlink",
    "symlink_dir", "symlink_file", "write",
]);

function usesDirectFilesystemMutationAuthority(source) {
    source = normalizeRustRawIdentifiers(source);
    // nix fs also exposes memfd_create, which returns an anonymous writable
    // descriptor. It is not a persistent export, but can be inherited by a
    // child unless CLOEXEC is explicitly requested. Require owner review.
    if (/\bnix\s*::\s*sys\s*::\s*memfd\b/u.test(source)) {
        return true;
    }
    if (FILE_TYPE_ALIAS.test(source)) {
        return true;
    }
    // Aliasing std::fs or std::os::unix::fs, or its File constructor,
    // hides subsequent mutations behind unqualified names.
    for (const match of source.matchAll(
        /\buse\s+(?:::\s*)?std\s*::([^;]+);/gu,
    )) {
        const imported = match[1];
        const moduleAlias = /\bfs\s+as\b/u.test(imported);
        const fsPath = /^\s*fs\s*::/u.test(imported)
            || /^\s*\{[^;]*\bfs\s*::/u.test(imported);
        const fileAlias = /\b(?:File|self)\s+as\b/u.test(imported);
        // A renamed FileExt can hide `Trait::write_at` function items;
        // `as _` only imports extension methods, whose calls are scanned.
        const positionalTraitAlias =
            /\bFileExt\s+as\s+(?!_\b)[\p{ID_Start}_]/u
                .test(imported)
            && /\bos\s*::\s*(?:unix|windows)\s*::\s*fs\b/u
                .test(imported.replace(/[{}]/gu, ""));
        if (moduleAlias || (fsPath && fileAlias) || positionalTraitAlias) {
            return true;
        }
    }
    // DirBuilder can create directories without a direct fs::create_dir
    // call. Its constructor and imported aliases need the same review gate.
    if (/\bDirBuilder\s*::\s*new\s*\(/u.test(source)
        || /\bfs\s*::\s*DirBuilder\b/u.test(source)
        || /\bfs\s*::\s*\{[^;]*\bDirBuilder\b/u.test(source)) {
        return true;
    }
    // Also recognize imports such as `use std::fs::write` rather than only
    // call sites, so an alias cannot evade the source review boundary.
    const paths = source.matchAll(/\bfs\s*::\s*(?:r#)?([a-z_]+)\b/gu);
    for (const match of paths) {
        if (MUTATING_FS_FUNCTIONS.has(match[1])) {
            return true;
        }
    }
    const grouped = source.matchAll(/\bfs\s*::\s*\{([^}]*)\}/gu);
    for (const match of grouped) {
        if (match[1].split(",").some((field) =>
            MUTATING_FS_FUNCTIONS.has(
                field.trim().split(/\s+/u)[0].replace(/^r#/u, "")))) {
            return true;
        }
    }
    // Positional FileExt writes can mutate a borrowed descriptor. In Rust,
    // fully qualified trait methods may be captured as function items and
    // called later, without an adjacent `(` or a trait import here.
    const positionalWrite = String.raw`(?:seek_write|write_at|` +
        String.raw`write_all_at|write_vectored_at)`;
    const positionalCall = new RegExp(
        String.raw`\b` + positionalWrite + String.raw`\s*\(`, "u",
    );
    const positionalItem = new RegExp(
        String.raw`\bFileExt\s*>?\s*::\s*` + positionalWrite + String.raw`\b`,
        "u",
    );
    if (positionalCall.test(source) || positionalItem.test(source)) {
        return true;
    }
    // The locked nix fs feature also exposes direct directory, inode and
    // metadata mutations. Check grouped imports, renamed functions, and
    // module aliases, while allowing read-only imports in the same modules.
    const nixMutations = new RegExp(
        String.raw`\b(?:chown|fchown|fchownat|fchmod|fchmodat|ftruncate|` +
            String.raw`futimens|linkat|mkdir|mkdirat|mkfifo|mkfifoat|` +
            String.raw`mknod|mknodat|mkstemp|` +
            String.raw`renameat|renameat2|rmdir|symlinkat|truncate|` +
            String.raw`unlink|unlinkat|utimensat|write)\b`,
        "u",
    );
    if (/\bftruncate\s*\(/u.test(source)) {
        return true;
    }
    for (const pathMatch of source.matchAll(
        /\bnix\s*::\s*(?:unistd|fcntl|sys\s*::\s*stat)\s*::\s*([a-z_]+)\b/gu,
    )) {
        if (nixMutations.test(pathMatch[1])) {
            return true;
        }
    }
    for (const match of source.matchAll(
        /\buse\s+(?:::\s*)?nix\s*::\s*([^;]+);/gu,
    )) {
        const imported = match[1];
        const module = /\b(?:unistd|fcntl|stat)\b/u;
        const moduleAlias = /\b(?:unistd|fcntl|stat)\s+as\b/u;
        const groupedAlias =
            /\b(?:unistd|fcntl|stat)\s*::\s*\{[^;]*\bself\s+as\b/u;
        const wholeModule = /\b(?:unistd|fcntl|stat)\s*(?:,|\}|$)/u;
        if (/\bsys\s*::[^;]*\bmemfd\b/u.test(imported)
            || moduleAlias.test(imported) || groupedAlias.test(imported)
            || wholeModule.test(imported)
            || (module.test(imported) && nixMutations.test(imported))) {
            return true;
        }
    }
    // An already-open descriptor can mutate on-disk content or metadata
    // without opening a new writable file. Keep these direct File methods in
    // the same explicit review boundary as File::create and fs::write.
    const fileMutator = new RegExp(
        String.raw`(?:\.\s*(?:r#)?set_(?:len|modified|permissions|` +
            String.raw`times)\s*\(|\bFile\s*::\s*(?:r#)?set_` +
            String.raw`(?:len|modified|permissions|times)\b)`,
        "u",
    );
    const unixMutator =
        /\b(?:linkat|unlinkat|renameat|renameat2|mkdirat|mknodat)\s*\(/u;
    // Associated constructors can be stored as function items and invoked
    // later without a visible `File::create(...)` call.
    const fileConstructor =
        /\bFile\s*::\s*(?:r#)?(?:create|create_new|options)\b/u;
    return fileMutator.test(source)
        || fileConstructor.test(source)
        || /\bOpenOptions\b/u.test(source)
        || /\bO_(?:TMPFILE|CREAT|TRUNC)\b/u.test(source)
        || unixMutator.test(source);
}

// Opening a host file, enumerating a directory, or resolving metadata can
// disclose private host content or path identity without changing it. Keep
// direct and method-based path reads within the reviewed profile adapters.
function usesAmbientFilesystemReading(source) {
    source = normalizeRustRawIdentifiers(source);
    if (FILE_TYPE_ALIAS.test(source) || PATH_QUERY_TYPE_ALIAS.test(source)) {
        return true;
    }
    // `File::open` is also a first-class function item: it can be assigned
    // to a local variable before invocation without an `open(` token.
    if (/\bFile\s*::\s*open\b/u.test(source)) {
        return true;
    }
    // Path/DirEntry/File metadata methods do not need an `fs::` qualifier.
    // This deliberately reviews similarly named methods on other receivers:
    // without Rust type resolution, a newly added host-path read must not be
    // silently accepted as an unrelated local method.
    const queryMethod = String.raw`(?:exists|try_exists|is_file|` +
        String.raw`is_dir|is_symlink|metadata|symlink_metadata|` +
        String.raw`canonicalize|read_dir|read_link)`;
    const pathQuery = new RegExp(
        String.raw`\.\s*` + queryMethod + String.raw`\s*\(`,
        "u",
    );
    // Inherent Path/File/DirEntry methods can be captured as function items
    // and later invoked by another name without a `method(` token here.
    const pathFunctionItem = new RegExp(
        String.raw`\b(?:Path|PathBuf|File|DirEntry)\s*::\s*` +
            queryMethod + String.raw`\b`,
        "u",
    );
    // Stable std::path::absolute resolves relative paths from the process
    // CWD without a direct env::current_dir call in the importing module.
    const absolutePath =
        /\bpath\s*::\s*(?:absolute\b|\{[^;]*\babsolute\b)/u;
    // The pinned nix fs feature also offers host existence, metadata, and
    // symlink-target queries without using std::fs or Path methods.
    const nixPathQuery = new RegExp(
        String.raw`\b(?:unistd|fcntl|stat)\s*::\s*` +
            String.raw`(?:\{[^;]*\b)?(?:access|faccessat|getcwd|` +
            String.raw`stat|lstat|fstat|fstatat|readlink|readlinkat)\b`,
        "u",
    );
    // Pinned nix `fs` also exposes path and descriptor volume statistics.
    // Review the module itself: its function items and aliases can be called
    // later without an adjacent statfs/statvfs invocation in this source.
    if (/\bnix\s*::\s*sys\s*::\s*stat(?:fs|vfs)\b/u.test(source)) {
        return true;
    }
    for (const match of source.matchAll(
        /\buse\s+(?:::\s*)?nix\s*::\s*([^;]+);/gu,
    )) {
        if (/\bsys\b/u.test(match[1])
            && /\bstat(?:fs|vfs)\b/u.test(match[1])) {
            return true;
        }
    }
    if (pathQuery.test(source) || pathFunctionItem.test(source)
        || absolutePath.test(source) || nixPathQuery.test(source)) {
        return true;
    }
    // A Path/PathBuf namespace alias can hide UFCS path probes from the
    // method inventory even without using a direct std::fs function.
    for (const match of source.matchAll(
        /\buse\s+(?:::\s*)?std\s*::\s*([^;]+);/gu,
    )) {
        const imported = match[1];
        // Renaming fs or File hides read constructors and path readers
        // behind names whose subsequent calls cannot be type-resolved here.
        if (/\bfs\s+as\b/u.test(imported)
            || /\bfs\s*::\s*File\s+as\b/u.test(imported)
            || /\bfs\s*::\s*\{[^;]*\b(?:self|File)\s+as\b/u
                .test(imported)
            || /\bpath\s+as\b/u.test(imported)
            || /\bpath\s*::\s*Path(?:Buf)?\s+as\b/u.test(imported)
            || /\bpath\s*::\s*\{[^;]*\b(?:self|Path|PathBuf)\s+as\b/u
                .test(imported)) {
            return true;
        }
    }
    // In a use path, an imported std::fs read can be renamed to hide the
    // eventual call. Grouped imports retain an fs:: prefix after normalization.
    const direct = new RegExp(
        String.raw`\bfs\s*::\s*(?:exists|read|read_to_string|read_dir|` +
            String.raw`read_link|` +
            String.raw`metadata|symlink_metadata|canonicalize)\b`,
        "u",
    );
    if (direct.test(source)) {
        return true;
    }
    const pathRead = new RegExp(
        String.raw`(?:^|,)\s*(?:exists|read|read_to_string|read_dir|` +
            String.raw`read_link|metadata|symlink_metadata|canonicalize)\b`,
        "u",
    );
    for (const match of source.matchAll(/\bfs\s*::\s*\{([^}]*)\}/gu)) {
        if (pathRead.test(match[1])) {
            return true;
        }
    }
    return false;
}

test("ambient filesystem read detector distinguishes read-only APIs", () => {
    for (const source of [
        'std::fs::read("private")',
        'fs::read_to_string("private")',
        'std::fs::read_dir("private")',
        'std::fs::exists("private")',
        'std::path::absolute("private")',
        'nix::unistd::access("/private", flags);',
        'nix::unistd::faccessat(&dir, "private", flags, options);',
        'nix::unistd::getcwd();',
        'use nix::unistd::getcwd as reveal_cwd;',
        'use ::nix::{unistd::{getcwd as reveal_cwd}};',
        'use nix::unistd::access as query;',
        'use ::nix::{unistd::{faccessat as query}};',
        // Pinned nix `fs` enables filesystem statistics independent of
        // std::fs and the narrower nix::sys::stat path metadata readers.
        'nix::sys::statfs::statfs("/private");',
        'nix::sys::statvfs::statvfs("/private");',
        'nix::sys::statfs::fstatfs(&borrowed_file);',
        'nix::sys::statvfs::fstatvfs(&borrowed_file);',
        'let query = nix::sys::statfs::statfs::<str>;',
        'let query = nix::sys::statvfs::statvfs::<str>;',
        'use nix::sys::statfs as filesystem_stats;',
        'use nix::sys::{statvfs as volume_stats};',
        'use ::nix::{sys::{statfs::{statfs as query}}};',
        'use r#nix::r#sys::r#statvfs::r#statvfs as query;',
        'nix/**/::sys::statfs::statfs("/private");',
        'nix::sys::stat::stat("/private");',
        'nix::sys::stat::lstat("/private");',
        'nix::sys::stat::fstatat(&dir, "private", flags);',
        'use nix::sys::stat::stat as query;',
        'use nix::sys::stat::{lstat as query};',
        'use ::nix::{sys::{stat::{fstatat as query}}};',
        'nix::fcntl::readlink("/private");',
        'nix::fcntl::readlinkat(&dir, "private");',
        'use nix::fcntl::{readlinkat as query};',
        'use r#nix::r#unistd::r#access as query;',
        'path::absolute("private")',
        'use std::path::absolute as resolve_host;',
        'use ::std::{path::{absolute as resolve_host}};',
        'use r#std::r#path::r#absolute as resolve_host;',
        'fs::exists("private")',
        'use std::fs::exists as probe;',
        'use ::std::{fs::{exists as probe}};',
        'use r#std::r#fs::r#exists as probe;',
        'fs::read_link("private")',
        'std::fs::metadata("private")',
        'std::fs::symlink_metadata("private")',
        'std::fs::canonicalize("private")',
        'use std::fs::metadata as probe;',
        'use std::fs::{symlink_metadata as probe};',
        'use ::std::{fs::{canonicalize as probe}};',
        'use r#std::r#fs::r#symlink_metadata as probe;',
        'File::open("private")',
        'type HostFile = std::fs::File;',
        'type HostFile = File;',
        'type HostFile = ::std::fs::File;',
        'type HostEntry = std::fs::DirEntry;',
        'type HostPath = std::path::Path;',
        'type HostPathBuf = path::PathBuf;',
        'type λ = r#std::r#path::r#Path;',
        'let opener = File::open; opener("/private");',
        'let opener = std::fs::File::open; opener("/private");',
        'use std::fs::File as HostFile;',
        'use ::std::{fs::{File as HostFile}};',
        'use r#std::r#fs::r#File as HostFile;',
        'use std::fs as host_fs;',
        'use std::fs::{self as host_fs, File};',
        'use ::std::{fs as host_fs};',
        'File :: r#open ("private")',
        'std::path::Path::new("/private").exists()',
        'path.try_exists()',
        'path.is_file()',
        'path.is_dir()',
        'path.is_symlink()',
        'path.metadata()',
        'path.symlink_metadata()',
        'path.canonicalize()',
        'path.read_dir()',
        'path.read_link()',
        'Path::exists(path)',
        'let probe = Path::exists; probe(path);',
        'let query = std::path::Path::canonicalize;',
        'let query = Path::try_exists;',
        'let query = Path::read_link;',
        'let query = std::fs::File::metadata;',
        'let query = File::metadata;',
        'let query = DirEntry::metadata;',
        'let query = r#Path::r#exists;',
        'PathBuf::try_exists(&path)',
        'File::metadata(&file)',
        'DirEntry::metadata(&entry)',
        'path.r#is_symlink()',
        'path /* comment */ . is_file()',
        'use std::path as host_path;',
        'use std::path::Path as HiddenPath;',
        'use std::{path::{PathBuf as HiddenPath}, io};',
        'use ::std::{path::{self as host_path, Path}};',
        'use r#std::r#path::r#Path as HiddenPath;',
        'use std::fs::read as leak;',
        'use ::std::{fs::{read_dir as enumerate}};',
        'use r#std::r#fs::r#read_to_string as leak;',
    ]) {
        assert.equal(usesAmbientFilesystemReading(source), true, source);
    }
    for (const source of [
        'File::create("output")',
        'type Local = crate::File;',
        'type Local = crate::PathBuf;',
        'type Local = Metadata;',
        'use std::fs::{File, Metadata};',
        'Path::new("/private").file_name();',
        'Path::new("/private").is_absolute();',
        'let check = Path::is_absolute;',
        'let flush = File::sync_all;',
        'path.ancestors();',
        'file.read_exact(&mut bytes);',
        'metadata.len();',
        'use std::path::{Path, PathBuf};',
        'use nix::unistd::read;',
        'use nix::sys::stat::Mode;',
        'use nix::fcntl::openat2;',
        'std::io::Read::read_to_end(&mut input, &mut bytes);',
    ]) {
        assert.equal(usesAmbientFilesystemReading(source), false, source);
    }
});

test("ambient filesystem reads stay in reviewed profile adapters", async () => {
    const readingFiles = [];
    for (const sourcePath of await rustSources(BACKEND_ROOT)) {
        const source = await readFile(sourcePath, "utf8");
        if (usesAmbientFilesystemReading(source)) {
            readingFiles.push(sourcePath);
        }
    }
    assert.deepEqual(readingFiles.sort(), [
        // Native paper and notebook CLI no longer perform ambient reads:
        // selected input now goes through symlink-refusing file adapters.
        "profile-export-target-check/composition/lib.rs",
        "profile-export-session/composition/lib.rs",
        "profile-file-commit/adapter-outbound/lib.rs",
        "profile-file-reader/adapter-outbound/lib.rs",
        // Localhost reads are bounded and symlink-refusing in this adapter.
        "profile-local-file-read/adapter-outbound/lib.rs",
        "profile-selected-file-open/adapter-outbound/lib.rs",
        "profile-open-file-import/composition/lib.rs",
    ].map((relative) => path.join(BACKEND_ROOT, relative)).sort());
});

// The locked nix dependency exposes libc raw syscalls even without the nix
// socket feature. A direct extern block or inline assembly can bypass the
// reviewed standard-library net/process/fs APIs with no Cargo diff. Require
// explicit review before introducing native authority to any backend file.
// This is deliberately conservative source inventory, not Rust parsing or a
// runtime syscall sandbox.
function usesUnreviewedNativeAuthority(source) {
    source = normalizeRustRawIdentifiers(source);
    const nativePatterns = [
        /\b(?:nix\s*::\s*)?libc\s*::/u,
        /\b(?:use\s+(?:::)?\s*|extern\s+crate\s+)(?:nix\s*::\s*)?libc\b/u,
        /\buse\s+(?:::)?\s*nix\s*::\s*\{[^;]*\blibc\b/u,
        /\b(?:use\s+(?:::)?\s*|extern\s+crate\s+)nix\s+as\s+\w+/u,
        /\buse\s+(?:::)?\s*nix\s*::\s*\{[^;]*\bself\s+as\s+\w+/u,
        // Importing nix::sys itself hides its stat/socket modules from the
        // reviewed nix-qualified source paths. Review namespace aliases too.
        /\buse\s+(?:::\s*)?nix\s*::\s*sys\s*(?:as\b|;)/u,
        /\buse\s+(?:::\s*)?nix\s*::\s*\{[^;]*\bsys\s*(?:as\s+\w+)?\s*[,}]/u,
        new RegExp(
            String.raw`\buse\s+(?:::\s*)?nix\s*::[^;]*\bsys\s*::` +
                String.raw`\s*\{[^;]*\bself\s*(?:as\s+\w+)?\s*[,}]`,
            "u",
        ),
        /\bnix\s*::\s*sys\s*::\s*socket\b/u,
        /\bnix\s*::\s*sys\s*::\s*\{[^;]*\bsocket\b/u,
        /\bnix\s*::\s*\{[^;]*\bsys\s*::\s*socket\b/u,
        // The Linux test-only `ptrace`/`uio` features must never become
        // backend production source authority via Cargo feature unification.
        /\bnix\s*::\s*sys\s*::\s*(?:ptrace|uio)\b/u,
        /\buse\s+(?:::)?\s*nix\s*::\s*sys\s*::\s*\{[^;]*\b(?:ptrace|uio)\b/u,
        /\buse\s+(?:::)?\s*nix\s*::\s*\{[^;]*\bsys\s*::\s*(?:ptrace|uio)\b/u,
        new RegExp(
            String.raw`\buse\s+(?:::)?\s*nix\s*::\s*\{[^;]*\bsys\s*::` +
                String.raw`\s*\{[^;]*\b(?:ptrace|uio)\b`,
            "u",
        ),
        /\b(?:unsafe\s+)?extern\s*(?:"[^"]+"\s*)?(?:\{|fn\b)/u,
        /\b(?:core|std)\s*::\s*arch\b/u,
        /\b(?:asm|global_asm)\s*!/u,
    ];
    return nativePatterns.some((pattern) => pattern.test(source));
}

// Scan all authored backend .rs files; including Rust code by a macro or a
// redirected module path could bypass that inventory. Include! accepts all
// three macro delimiters; its std/core exports can also be imported by alias.
// Binary and text asset inclusion has a separate exact inventory.
function redirectsBackendRustSource(source) {
    source = normalizeRustRawIdentifiers(source);
    return /\binclude\s*!\s*[([{]/u.test(source)
        || /\buse\s+(?:::)?(?:std|core)\s*::[^;]*\binclude\b/u
            .test(source)
        || /#\s*\[\s*path\s*=/u.test(source)
        || /#\s*\[\s*cfg_attr\s*\(/u.test(source);
}

// Rust `mod child;` may follow a symbolic link into code absent from the
// backend source walk. Require review of any alias in the scanned tree.
function rejectBackendSourceLink(entry, sourcePath) {
    if (entry.isSymbolicLink()) {
        throw new Error(
            `backend symbolic links need explicit source review: ${sourcePath}`,
        );
    }
}

async function rustSources(directory) {
    const sources = [];
    for (const entry of await readdir(directory, { withFileTypes: true })) {
        const child = path.join(directory, entry.name);
        rejectBackendSourceLink(entry, child);
        if (entry.isDirectory()) {
            sources.push(...await rustSources(child));
        } else if (entry.isFile() && entry.name.endsWith(".rs")) {
            sources.push(child);
        }
    }
    return sources;
}

// A glob import can conceal a later filesystem, process, native, or network
// effect behind an unqualified symbol even without any direct API import.
function importsUnreviewedAuthorityGlob(source) {
    source = normalizeRustRawIdentifiers(source);
    for (const match of source.matchAll(
        /\buse\s+(?:::\s*)?(std|nix)\s*::\s*([^;]+);/gu,
    )) {
        const imported = match[2];
        const authorityModule = match[1] === "std"
            ? /\b(?:env|fs|io|net|process)\s*::\s*(?:\*|\{[^}]*\*)/u
            : /\b(?:unistd|fcntl|stat|sys)\s*::\s*(?:\*|\{[^}]*\*)/u;
        if (authorityModule.test(imported) || /^\s*\*\s*$/u.test(imported)) {
            return true;
        }
        // A crate-root grouped glob is a top-level `*` item, not e.g.
        // `fmt::*` or `errno::*`, which remain outside this guard.
        const groupedRoot = /^\s*\{([\s\S]*)\}\s*$/u.exec(imported);
        if (groupedRoot === null) {
            continue;
        }
        let nestedDepth = 0;
        let field = "";
        for (const character of groupedRoot[1]) {
            if (character === "{") nestedDepth += 1;
            if (character === "}") nestedDepth -= 1;
            if (character === "," && nestedDepth === 0) {
                if (field.trim() === "*") return true;
                field = "";
            } else {
                field += character;
            }
        }
        if (field.trim() === "*") return true;
    }
    return false;
}

test("authority glob-import detector covers direct and nested syntax", () => {
    for (const source of [
        'use std::*;',
        'use std::env::*;',
        'use std::{env::{args_os, *}, fmt};',
        'use ::std::{fmt, *};',
        'use r#std::r#fs::*;',
        'use std::fs::{read, *};',
        'use std::{io::{Read, *}, fmt};',
        'use std::process::*;',
        'use std::os::unix::net::{UnixStream, *};',
        'use nix::*;',
        'use ::nix::{errno::Errno, *};',
        'use nix::unistd::*;',
        'use nix::{unistd::{read, *}, errno::Errno};',
        'use nix::sys::stat::*;',
        'use nix::{sys::{stat::{SFlag, *}}};',
        'use nix::fcntl::*;',
        'use nix::sys::*;',
        'use nix::{sys::{stat, *}};',
    ]) {
        assert.equal(importsUnreviewedAuthorityGlob(source), true, source);
    }
    for (const source of [
        'use std::fmt::*;',
        'use std::{fmt::*, io::Read};',
        'use std::fs::File;',
        'use std::io::{self, Read as _, ErrorKind};',
        'use crate::fs::*;',
        'use nix::errno::*;',
        'use nix::sys::stat::{fstatat, SFlag};',
        'use nix::{errno::Errno, sys::stat::SFlag};',
    ]) {
        assert.equal(importsUnreviewedAuthorityGlob(source), false, source);
    }
});

test("backend authority namespaces cannot be glob-imported", async () => {
    const unreviewed = [];
    for (const sourcePath of await rustSources(BACKEND_ROOT)) {
        const source = await readFile(sourcePath, "utf8");
        if (importsUnreviewedAuthorityGlob(source)) {
            unreviewed.push(sourcePath);
        }
    }
    assert.deepEqual(unreviewed, [],
        "glob imports can bypass individual authority review");
});

test("standard-library network detector covers equivalent imports", () => {
    for (const source of [
        "use std::net::TcpListener;",
        "use std/**/::net::TcpListener;",
        "use std/* hidden */::net::TcpListener;",
        "use std// split module\n::net::TcpListener;",
        "use std::r#net::TcpListener;",
        "use r#std::r#net::TcpStream;",
        "use r#std::{r#net::TcpStream};",
        "use std :: net :: TcpStream;",
        "use std::{io, net::TcpListener};",
        "use std::{io::{Read, Write}, net::{TcpListener, TcpStream}};",
        "use std::{fmt, net as network};",
        "use ::std::{net::TcpStream};",
        "use ::std::{net as network};",
        "use :: std::{net::TcpStream};",
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
        "use std/* nested /* comment */ still */::process::Command;",
        "use std::r#process::Command;",
        "use r#std::r#process::r#Command;",
        "use r#std::{r#process::Command};",
        "use std :: process :: Command;",
        "use std::{io, process::Command};",
        "use std::{io::{Read, Write}, process::{Child, Command}};",
        "use std::{fmt, process as operating_system_process};",
        "use ::std::{process::Command};",
        "use ::std::{process as operating_system_process};",
        "use :: std::{process::Command};",
        'std::thread::spawn(|| {});',
        'thread::spawn(|| {});',
        'std::thread::scope(|scope| scope.spawn(|| {}));',
        'let scope = std::thread::scope;',
        'let start = Scope::spawn;',
        'let start = Builder::spawn_scoped;',
        'let start = std::thread::Scope::spawn;',
        'use std::thread::scope as with_threads;',
        'use std::thread::Scope as ScopedWorkers;',
        'use std::{thread::{scope as with_threads}, fmt};',
        'use ::std::{thread::{Scope as ScopedWorkers}};',
        'use std::thread::spawn as launch_worker;',
        'use std::{thread::{spawn as launch_worker}, fmt};',
        'use ::std::{thread::{Builder as WorkerBuilder}};',
        'use std::thread as worker_thread;',
        'use std::{thread::{self as worker_thread}, fmt};',
        'use std::thread::*;',
        'builder.spawn_scoped(scope, || {});',
        'builder.spawn_unchecked(|| {});',
        'launcher.spawn();',
        'launcher.exec();',
        'launcher.status();',
        'launcher.output();',
        'launcher.pre_exec(|| Ok(()));',
        // Generic trait bounds and captured method items require no
        // `use std::os::unix::process` or immediate invocation.
        'fn capture<T: std::os::unix::process::CommandExt>() {}',
        'let exec = <T as std::os::unix::process::CommandExt>::exec;',
        'fn capture<T: std::os::unix::process::CommandExt>() {' +
            ' let pre_exec = T::pre_exec::<fn() -> std::io::Result<()>>; }',
        'let exec = CommandExt::exec;',
        'let pre_exec = CommandExt::pre_exec;',
        'let before_exec = CommandExt::before_exec;',
        'fn capture<T: ::std::os::windows::process::CommandExt>() {}',
        'fn capture<T: r#std::r#os::r#unix::r#process::r#CommandExt>() {}',
        'CommandExt::before_exec(&mut launcher, || Ok(()));',
        'CommandExt::exec(&mut launcher);',
        'use std::os::unix::process::CommandExt;',
        'use std::os::windows::process::CommandExt as _;',
        'use ::std::{os::{unix::{process::{CommandExt as _}}}};',
        'use std::os::unix::process as native_process;',
        'use std::{os::unix::process, fmt};',
        'use std::os::unix::process::{self as native, ExitStatusExt};',
        'use std::{os as operating_system, fmt};',
        'use std::os::unix as native_unix;',
        'use std::os::{unix as native_unix, fd::AsRawFd};',
        'use std::os::unix::{self as native_unix, fs::MetadataExt};',
        'use std::os::unix::process::*;',
        'use std::{os::unix::*};',
        'use std::os::*;',
        'use std::os;',
        'use std::{os, fmt};',
        'use std::os::unix;',
        'use std::{os::{unix, fd::AsRawFd}};',
        "std::process::Command::new(\"curl\");",
        // Command constructors and execution functions are first-class
        // method items; no call is needed in the declaring module.
        'let construct = Command::new::<&str>;',
        'let start = Command::spawn;',
        'let status = Command::status;',
        'let output = Command::output;',
        'let before = Command::pre_exec;',
        'let exec = Command::exec;',
        'let start = r#Command::r#spawn;',
        'let output = Command/* comment */::output;',
        "Command::new(\"curl\");",
    ]) {
        assert.equal(usesStandardLibraryProcessAuthority(source), true,
            source);
    }
    for (const source of [
        "use std::{fmt, io};",
        "use crate::process::Policy;",
        "let process = false;",
        'use std::os::unix::process::ExitStatusExt;',
        'use std::os::unix::fs::MetadataExt;',
        'use std::{os::fd::AsRawFd, fmt};',
        'use ::std::{os::{unix::{process::ExitStatusExt}}};',
        'use std::os::windows::process::ExitStatusExt;',
        'use crate::os::unix::process::CommandExt;',
        'thread::sleep(std::time::Duration::ZERO);',
        'use std::{fmt, io, thread};',
        'use std::thread::sleep;',
        'use std::thread::{self, sleep};',
        'launcher.spawn_preview();',
        'let preview = Command::preview;',
        'launcher.preview_exec();',
    ]) {
        assert.equal(usesStandardLibraryProcessAuthority(source), false,
            source);
    }
});

// A narrow print exception must not hide an extra command, worker or shell.
// One named CUPS stdin writer is reviewed for bounded pipe submissions.
// Strip only its exact construction and fixed argv/stream routing, then
// scan everything left for otherwise unreviewed process authority.
function unreviewedPrintSubprocessAuthority(source) {
    let remaining = source;
    for (const site of [
        'use std::process::{Command, Stdio};',
        'Command::new(executable)',
        '.args(["-d", printer, "-"])',
        '.stdin(Stdio::piped())',
        '.stdout(Stdio::null())',
        '.stderr(Stdio::null())',
        '.spawn()',
        'thread::Builder::new()',
        '.name("atrament-cups-stdin".into())',
        '.spawn(move || input.write_all(&pdf))',
        'Path::new("/usr/bin/lp")',
    ]) {
        if (remaining.split(site).length - 1 !== 1) return true;
        remaining = remaining.replace(site, "");
    }
    return usesStandardLibraryProcessAuthority(remaining);
}

test("opt-in CUPS process cannot grow an extra subprocess", async () => {
    const print = await readFile(PRINT_CLI, "utf8");
    assert.equal(unreviewedPrintSubprocessAuthority(print), false);
    for (const extra of [
        'Command::new("sh").spawn();',
        'std::process::Command::new("curl").status();',
        'command.spawn().unwrap();',
        'std::thread::spawn(move || {});',
        'thread::Builder::new().name("second".into()).spawn(|| {});',
        '.spawn(move || input.write_all(&pdf))',
        'use std::process::Command as OtherCommand;',
    ]) {
        assert.equal(unreviewedPrintSubprocessAuthority(`${print}\n${extra}`),
            true, extra);
    }
});

test("backend subprocesses stay within browser and opt-in print CLI",
    async () => {
    const spawningFiles = [];
    for (const sourcePath of await rustSources(BACKEND_ROOT)) {
        const source = await readFile(sourcePath, "utf8");
        if (usesStandardLibraryProcessAuthority(source)) {
            spawningFiles.push(sourcePath);
        }
    }
    // Only a reviewed, fixed CUPS executable and named queue may be launched.
    // This is a narrow process exception, not a general subprocess adapter.
    const print = await readFile(PRINT_CLI, "utf8");
    assert.equal(unreviewedPrintSubprocessAuthority(print), false);
    assert.deepEqual(spawningFiles.sort(), [BROWSER_LAUNCH, PRINT_CLI].sort());
});

// A borrowed File or stream can be read without opening it in the current
// module. Review Read/BufRead, positional FileExt reads, and delegated reads
// separately from selected path opening; the receiver is not type-resolved.
// The locked nix fs dependency also exposes `unistd::read` on any AsFd,
// independently of std::io::Read trait imports or method-call spelling.
function usesBorrowedStreamReading(source) {
    source = normalizeRustRawIdentifiers(source);
    const readMethod = String.raw`(?:read|read_exact|read_to_end|` +
        String.raw`read_to_string|read_vectored|read_buf|read_buf_exact|` +
        String.raw`read_line|read_until|fill_buf|read_at|read_exact_at|` +
        String.raw`read_vectored_at|seek_read)`;
    const readCall = readMethod + String.raw`\s*\(`;
    const iteratorMethod = String.raw`(?:bytes|lines|split|take|chain|by_ref)`;
    // Trait method items can read a borrowed descriptor in a later call,
    // even when no `Read` import or `.read(...)` occurs in this module.
    const traitItem = new RegExp(
        String.raw`\b(?:Read|BufRead|FileExt)\s*::\s*` +
            String.raw`(?:` + readMethod + String.raw`|` +
            iteratorMethod + String.raw`)\b`, "u",
    );
    const qualifiedItem = new RegExp(
        String.raw`<[^;{}]*\bas\s+(?:::)?std\s*::\s*(?:io\s*::` +
            String.raw`\s*(?:Read|BufRead)|os\s*::\s*(?:unix|windows)` +
            String.raw`\s*::\s*fs\s*::\s*FileExt)\s*>\s*::\s*` +
            String.raw`(?:` + readMethod + String.raw`|` +
            iteratorMethod + String.raw`)\b`, "u",
    );
    // UFCS iterator adapters can consume an inherited File via `.next()`
    // without ever calling a visible `.read()` method in this module.
    const iteratingReader = new RegExp(
        String.raw`\b(?:Read|BufRead)\s*::\s*` +
            iteratorMethod + String.raw`\b`, "u",
    );
    // Qualified calls, function-item imports and whole-module aliases can
    // consume an inherited descriptor without any std::io read call. Only
    // check nix-rooted imports so unrelated crate-local `unistd` is inert.
    if (/\bnix\s*::\s*unistd\s*::\s*read\b/u.test(source)) {
        return true;
    }
    for (const match of source.matchAll(
        /\buse\s+(?:::\s*)?nix\s*::\s*([^;]+);/gu,
    )) {
        const imported = match[1];
        if (/\bunistd\s*::\s*(?:read\b|\{[^;]*\bread\b)/u
            .test(imported)
            || /\bunistd\s+as\b/u.test(imported)
            || /\bunistd\s*::\s*\{[^;]*\bself\s+as\b/u
                .test(imported)
            || /\bunistd\s*(?:[,}]|$)/u.test(imported)
            || /\bunistd\s*::\s*\*/u.test(imported)) {
            return true;
        }
    }
    if (new RegExp(String.raw`\.\s*` + readCall, "u").test(source)
        || traitItem.test(source) || qualifiedItem.test(source)
        || iteratingReader.test(source)) {
        return true;
    }
    for (const match of source.matchAll(
        /\buse\s+(?:::\s*)?std\s*::[^;]+;/gu,
    )) {
        const imported = match[0];
        if (/\bio\s*::\s*(?:Read|BufRead)\b/u.test(imported)
            || /\bio\s*::\s*\{[^;]*\b(?:Read|BufRead)\b/u
                .test(imported)
            || (/\b(?:unix|windows)\b/u.test(imported)
                && /\bfs\b/u.test(imported)
                && /\bFileExt\b/u.test(imported))) {
            return true;
        }
    }
    return false;
}

// The profile CLI may read an opt-in `-` source, but that is the only
// inherited input descriptor. Pin its bounded reader byte-for-byte; any
// additional reader elsewhere in the CLI must return to source review.
const REVIEWED_CLI_STDIN_READER =
    String.raw`fn read_bounded_stdin(limit: usize, label: &str) `
    + String.raw`-> Result<Vec<u8>, String> {
    let maximum = u64::try_from(
        limit.checked_add(1).ok_or("stdin input limit overflow")?,
    )
    .map_err(|_error| "stdin input limit exceeds u64")?;
    let mut bytes = Vec::new();
    let _read = io::stdin()
        .lock()
        .take(maximum)
        .read_to_end(&mut bytes)
        .map_err(|_error| format!("{label} stdin read failed"))?;
    if bytes.len() > limit {
        return Err(format!("{label} stdin exceeds {limit} UTF-8 bytes"));
    }
    Ok(bytes)
}`;

function reviewedProfileCliStdinReader(source) {
    if (source.split(REVIEWED_CLI_STDIN_READER).length !== 2
        || source.split("Read as _,").length !== 2) {
        return false;
    }
    const otherSource = source.replace(REVIEWED_CLI_STDIN_READER, "")
        .replace("Read as _,", "");
    return !usesBorrowedStreamReading(otherSource);
}

test("borrowed stream-reading detector reviews std and nix APIs", () => {
    for (const source of [
        'use std::io::Read as _;',
        'use r#std::r#io::r#Read as _;',
        'use ::std::{io::{Read as _, Cursor}};',
        'use std::io::{self, BufRead as _};',
        'use std::os::unix::fs::FileExt;',
        'use ::std::{os::{unix::{fs::{FileExt as _}}}};',
        'use std::os::windows::fs::FileExt as _;',
        'file.read(&mut bytes);',
        'file.r#read_exact(&mut bytes);',
        'file.read_to_end(&mut bytes);',
        'file.read_to_string(&mut text);',
        'file.read_vectored(&mut buffers);',
        'file.read_buf(&mut buffer);',
        'file.read_line(&mut text);',
        'file.fill_buf();',
        'file.read_at(&mut bytes, 0);',
        'file.read_exact_at(&mut bytes, 0);',
        'file.seek_read(&mut bytes, 0);',
        'Read::read_to_end(&mut file, &mut bytes);',
        'std::io::Read::bytes(file).next();',
        'std::io::Read::take(file, 2);',
        'std::io::Read::chain(file, other);',
        'std::io::Read::by_ref(&mut file);',
        'std::io::BufRead::lines(buffer).next();',
        "std::io::BufRead::split(buffer, b',').next();",
        'FileExt::read_at(&file, &mut bytes, 0);',
        'std/* comment */::io::Read::read(&mut file, &mut bytes);',
        'let query = std::io::Read::read_to_end;',
        'let query = ::std::io::BufRead::fill_buf;',
        'let query = <File as std::io::Read>::read_to_end;',
        'let query = <File as ::std::io::Read>::read_exact;',
        'let query = <File as std::os::unix::fs::FileExt>::read_at;',
        'let query = r#std::r#io::r#Read::r#read_to_end;',
        'let lines = std::io::BufRead::lines;',
        'let bytes = std::io::Read::bytes;',
        'let lines = <BufReader<File> as std::io::BufRead>::lines;',
        'nix::unistd::read(&borrowed_file, &mut bytes);',
        '::nix::unistd::read(&borrowed_file, &mut bytes);',
        'use nix::unistd::read as consume;',
        'use nix::unistd::{read as consume, getpid};',
        'use ::nix::{unistd::{read as consume}};',
        'use r#nix::r#unistd::r#read as consume;',
        'use nix::unistd as native_io;',
        'use nix::{unistd as native_io, errno::Errno};',
    ]) {
        assert.equal(usesBorrowedStreamReading(source), true, source);
    }
    for (const source of [
        'use std::io::{self, Write as _};',
        'use std::fmt::Write as _;',
        'use crate::io::Read as _;',
        'use std::os::unix::fs::MetadataExt;',
        'use nix::unistd::getpid as process_id;',
        'use nix::unistd::readlink as target;',
        'use nix::sys::stat::Mode;',
        'file.read_metadata();',
        'file.write_all(b"private");',
        'buffer.bytes();',
        'file.read_exact_size();',
    ]) {
        assert.equal(usesBorrowedStreamReading(source), false,
            source);
    }
});

test("profile CLI source pins only the bounded opt-in stdin reader",
    async () => {
    const source = await readFile(PROFILE_EDITOR_CLI, "utf8");
    assert.equal(reviewedProfileCliStdinReader(source), true);
    assert.equal(unreviewedStandardStreamAuthority(source,
        PROFILE_EDITOR_CLI), false);
    for (const [before, after] of [
        ["limit.checked_add(1)", "limit.checked_add(2)"],
        [".take(maximum)", ".take(u64::MAX)"],
        ["if bytes.len() > limit", "if bytes.is_empty()"],
    ]) {
        assert.equal(reviewedProfileCliStdinReader(
            source.replace(before, after)), false, before);
    }
    for (const extra of [
        "file.read_to_end(&mut contents);",
        "std::io::Read::bytes(file);",
        "use std::io::BufRead as _;",
    ]) {
        assert.equal(reviewedProfileCliStdinReader(
            `${source}\n${extra}`), false, extra);
    }
    assert.equal(unreviewedStandardStreamAuthority(
        `${source}\nlet extra = io::stdin();`, PROFILE_EDITOR_CLI),
    true, "second stdin access cannot borrow ambient input");
});

test("backend borrowed stream reads stay in reviewed adapters",
    async () => {
    const readers = [];
    for (const sourcePath of await rustSources(BACKEND_ROOT)) {
        const source = await readFile(sourcePath, "utf8");
        if (usesBorrowedStreamReading(source)) {
            readers.push(sourcePath);
        }
    }
    assert.deepEqual(readers.sort(), [
        FILESYSTEM_COMMIT,
        path.join(BACKEND_ROOT, "profile-file-reader", "adapter-outbound",
            "lib.rs"),
        path.join(BACKEND_ROOT, "profile-entry-stream-reader",
            "adapter-inbound", "lib.rs"),
        PROFILE_EDITOR_CLI,
        LOOPBACK_RUNTIME,
    ].sort(), "new stream reads require privacy review");
});

test("std::io stream-writing detector distinguishes reviewed authority", () => {
    for (const source of [
        'use std::io::Write as _;',
        'use r#std::r#io::r#Write as _;',
        'use ::std::io::{Read, Write as _};',
        'use :: std::{fmt, io::{Read as _, Write as _}};',
        'use std::{io as stream_io, fmt};',
        'use std::io::{self as stream_io, Read};',
        'std::io::Write::write_all(&mut file, b"private");',
        'file.write_all(b"private");',
        'file.r#write(b"private");',
        'file.write_vectored(&slices);',
        'Write::write_all(&mut file, b"private");',
        'std::io::copy(&mut source, &mut file);',
        'io::copy(&mut source, &mut file);',
        'use std::io::copy as delegated_write;',
        'use std::io::{Read, copy as delegated_write};',
        'use ::std::{io::{copy as delegated_write}, fmt};',
        'use r#std::r#io::r#copy as delegated_write;',
        'std/* comment */::io::copy(&mut input, &mut output);',
        'file.flush();',
        'file.r#flush();',
        'Write::flush(&mut file);',
        'let save = std::io::Write::write_all;',
        'let save = <File as std::io::Write>::write_all;',
        'let save = <File as ::std::io::Write>::flush;',
        'let save = r#std::r#io::r#Write::r#write_all;',
    ]) {
        assert.equal(usesStandardLibraryStreamWriting(source), true, source);
    }
    for (const source of [
        'use std::fmt::Write as _;',
        'use std::io::{self, Read as _, ErrorKind};',
        'use std::{fmt::Write as _, io::Read as _};',
        'use crate::io::Write as _;',
        'file.read_exact(buffer);',
        'file.sync_all();',
        'file.write_all_at(b"private", 0);',
        'file.flush_pending_buffer();',
        'use std::io::{self, Read as _, Cursor};',
        'source.read_to_end(&mut data);',
    ]) {
        assert.equal(usesStandardLibraryStreamWriting(source), false, source);
    }
});

test("std::io stream writing stays in reviewed output adapters", async () => {
    const writers = [];
    for (const sourcePath of await rustSources(BACKEND_ROOT)) {
        const source = await readFile(sourcePath, "utf8");
        if (usesStandardLibraryStreamWriting(source)) {
            writers.push(sourcePath);
        }
    }
    assert.deepEqual(writers.sort(), [
        PAPER_CLI,
        PRINT_CLI,
        PROFILE_EDITOR_CLI,
        FILESYSTEM_COMMIT,
        path.join(BACKEND_ROOT, "runtime-bootstrap", "composition", "main.rs"),
        LOOPBACK_RUNTIME,
    ].sort());
});

test("nix fcntl authority includes writable descriptor constructors", () => {
    for (const source of [
        'use nix::fcntl::{OFlag, OpenHow, openat2};',
        'use nix::fcntl::openat;',
        'use nix::fcntl as descriptor_api;',
        'use ::nix::{errno::Errno, fcntl::{OFlag, openat2}};',
        'use r#nix::{r#fcntl::r#openat};',
        'use nix::{fcntl as descriptor_api, errno};',
        'nix::fcntl::openat(&directory, path, flags, mode);',
        '::nix::fcntl::openat2(&directory, path, options);',
    ]) {
        assert.equal(usesNixFcntlAuthority(source), true, source);
    }
    for (const source of [
        'use nix::errno::Errno;',
        'use nix::{errno::Errno, unistd::read};',
        'use crate::fcntl::openat2;',
        'std::fs::read("path");',
        'File::open("path");',
    ]) {
        assert.equal(usesNixFcntlAuthority(source), false, source);
    }
});

test("nix descriptor-open authority stays in reviewed adapters", async () => {
    const fcntlFiles = [];
    for (const sourcePath of await rustSources(BACKEND_ROOT)) {
        const source = await readFile(sourcePath, "utf8");
        if (usesNixFcntlAuthority(source)) {
            fcntlFiles.push(sourcePath);
        }
    }
    assert.deepEqual(fcntlFiles.sort(), [
        FILESYSTEM_COMMIT,
        path.join(BACKEND_ROOT, "profile-selected-file-open",
            "adapter-outbound", "lib.rs"),
    ].sort());
});

test("process-global path detector covers cwd and umask", () => {
    for (const source of [
        'std::env::set_current_dir("/untrusted");',
        'env::set_current_dir("/untrusted");',
        'use std::env::set_current_dir as change_cwd;',
        'use ::std::{env::{set_current_dir as change_cwd}};',
        'use std::env as environment;',
        'use std::{env as environment, fs::File};',
        'nix::unistd::chdir("/untrusted");',
        'nix::unistd::fchdir(&directory);',
        'nix::unistd::chroot("/untrusted");',
        'nix::unistd::pivot_root("/untrusted", "/old");',
        'let mutate = nix::unistd::pivot_root::<str, str>;',
        'use nix::unistd::pivot_root as change_root;',
        'use nix::unistd::{pivot_root as change_root};',
        'use ::nix::{unistd::{pivot_root as change_root}};',
        'use r#nix::r#unistd::r#pivot_root as change_root;',
        'nix/**/::unistd::pivot_root("/untrusted", "/old");',
        'nix::sys::stat::umask(mode);',
        'use nix::unistd::{chdir as change_cwd, fchdir as change_fd};',
        'use ::nix::{unistd::{chroot as change_root}};',
        'use r#nix::r#sys::r#stat::r#umask;',
        'use nix::sys::stat::umask as change_mask;',
        'use nix::{sys::{stat::{umask as change_mask}}};',
        'use nix::unistd as operating_system;',
        'use nix::sys::stat as metadata;',
        'use nix::{unistd as operating_system, errno::Errno};',
    ]) {
        assert.equal(usesProcessWidePathAuthority(source), true, source);
    }
    for (const source of [
        'use std::env;',
        'std::env::current_dir();',
        'env::var_os("TMPDIR");',
        'use std::env::var_os as get_env;',
        'use nix::unistd::{read, getpid};',
        'nix::unistd::read(&file, bytes);',
        'use nix::sys::stat::{SFlag, fstatat};',
        'nix::sys::stat::fstatat(&dir, path, flags);',
    ]) {
        assert.equal(usesProcessWidePathAuthority(source), false, source);
    }
});

test("backend has no process-global path mutation authority", async () => {
    const authorityFiles = [];
    for (const sourcePath of await rustSources(BACKEND_ROOT)) {
        const source = await readFile(sourcePath, "utf8");
        if (usesProcessWidePathAuthority(source)) {
            authorityFiles.push(sourcePath);
        }
    }
    assert.deepEqual(authorityFiles, []);
});

test("process-wide environment mutators require source review", () => {
    for (const source of [
        'std::env::set_var("PATH", "untrusted");',
        'env::remove_var("DISPLAY");',
        'let change = std::env::set_var::<&str, &str>;',
        'let change = env::remove_var::<&str>;',
        'let change = r#std/**/::r#env::r#set_var::<&str, &str>;',
        'r#std::r#env::r#set_var("PATH", "untrusted");',
        'use std::env::set_var;',
        'use std::env::remove_var as forget;',
        'use ::std::env::{set_var as change, remove_var};',
        'use std::{env::{remove_var as forget}};',
        'use std::{env as process_env, fs::File};',
        'use std::env::{self as process_env, var_os};',
    ]) {
        assert.equal(usesProcessWideEnvironmentMutation(source), true,
            source);
    }
    for (const source of [
        'use std::env;',
        'std::env::var_os("DISPLAY");',
        'use std::env::{current_dir, var_os};',
        'use std::{env::{var_os, args_os}, fs::File};',
        'use crate::env::remove_var;',
        'let set_var = false;',
    ]) {
        assert.equal(usesProcessWideEnvironmentMutation(source), false,
            source);
    }
});

test("backend has no process-global environment mutation authority",
    async () => {
    const authorityFiles = [];
    for (const sourcePath of await rustSources(BACKEND_ROOT)) {
        const source = await readFile(sourcePath, "utf8");
        if (usesProcessWideEnvironmentMutation(source)) {
            authorityFiles.push(sourcePath);
        }
    }
    assert.deepEqual(authorityFiles, []);
});

test("direct filesystem mutation detector covers write authority", () => {
    for (const source of [
        'use std::os::unix::fs as unix_fs;',
        'use std::os::unix::{fs as unix_fs};',
        'use ::std::{os::unix::fs as private_fs};',
        'std::os::unix::fs::chown("path", Some(1), Some(2));',
        'std::os::unix::fs::lchown("path", None, Some(2));',
        'std::os::unix::fs::fchown(&file, None, Some(2));',
        'std::os::unix::fs::chroot("/private");',
        'use std::os::unix::fs::{chown as owner, fchown as file_owner};',
        'use std::os::unix::fs::lchown as link_owner;',
        'use std::os::unix::fs::chroot as change_root;',
        'use r#std::r#os::r#unix::r#fs::r#fchown;',
        'use ::std::{os::{unix::{fs::{chown as owner}}}};',
        'use std::fs::DirBuilder;',
        'use std::fs::DirBuilder as HiddenBuilder;',
        'use std::{fs::{DirBuilder as HiddenBuilder}};',
        'use ::std::{fs::{DirBuilder}};',
        'use r#std::r#fs::r#DirBuilder;',
        'DirBuilder::new().recursive(true).create("private");',
        'std::fs::DirBuilder::new().create("private");',
        // The pinned nix fs feature also enables anonymous writable files.
        'nix::sys::memfd::memfd_create("secret", MFdFlags::empty());',
        'let creator = nix::sys::memfd::memfd_create::<str>;',
        'use nix::sys::memfd::memfd_create as creator;',
        'use nix::sys::memfd as anon_files;',
        'use nix::sys::{memfd as anon_files, stat::Mode};',
        'use ::nix::{sys::{memfd::{memfd_create as creator}}};',
        'use r#nix::r#sys::r#memfd::r#memfd_create as creator;',
        'nix/**/::sys::memfd::memfd_create("secret", flags);',
        // nix 0.31.3 `fs` enables mkstemp: it creates a persistent file
        // without std::fs, nix::fcntl, or a borrowed stream write.
        'nix::unistd::mkstemp("private_XXXXXX");',
        'let create = nix::unistd::mkstemp::<str>;',
        'use nix::unistd::mkstemp as create_file;',
        'use ::nix::{unistd::{mkstemp as create_file}};',
        'use r#nix::r#unistd::r#mkstemp as create_file;',
        'nix/**/::unistd::mkstemp("private_XXXXXX");',
        'nix::sys::stat::mknod("private", kind, perms, dev);',
        'nix::sys::stat::mknodat(&dir, "private", kind, perms, dev);',
        'use nix::sys::stat::mknod as create_inode;',
        'use nix::sys::stat::{mknodat as create_inode};',
        'use ::nix::{sys::{stat::{mknod as create_inode}}};',
        'use r#nix::r#sys::r#stat::r#mknodat;',
        'std::fs::write("state", b"private");',
        'std/**/::fs::write("state", b"private");',
        'std::fs::r#write("state", b"private");',
        'use std::fs::r#write as hidden_write;',
        'use std::fs::{read, r#write as hidden_write};',
        'File::r#create("state");',
        'file.r#set_len(0);',
        'file.r#set_permissions(mode);',
        'file.r#set_modified(time);',
        'file.r#set_times(times);',
        'file.write_at(b"secret", 0);',
        'file.r#write_all_at(b"secret", 0);',
        'FileExt::write_at(&file, b"secret", 0);',
        'FileExt :: write_all_at ( &file, b"secret", 0 );',
        'let write = FileExt::write_at;',
        'let write_all = FileExt::write_all_at;',
        'let vectored = FileExt::write_vectored_at;',
        'let windows_write = FileExt::seek_write;',
        'let write = <File as std::os::unix::fs::FileExt>::write_at;',
        'let write = <File as std::os::windows::fs::FileExt>::seek_write;',
        'let write = <File as ::std::os::unix::fs::FileExt>::write_at;',
        'let write = FileExt::r#write_all_at;',
        'use std::os::unix::fs::FileExt as OutputTrait;',
        'use std::{os::{unix::{fs::{FileExt as OutputTrait}}}};',
        'use std::os::windows::fs::FileExt as OutputTrait;',
        'use std::os::unix::fs as hidden_fs;',
        'nix::unistd::ftruncate(&file, 0);',
        'use nix::unistd::ftruncate as hidden_truncate;',
        'use nix::unistd::{read, truncate as hidden_truncate};',
        'use nix::unistd::{read, write as hidden_write};',
        'use nix::unistd as unix_io;',
        'use ::nix::{unistd::{ftruncate as hidden_truncate}};',
        'use r#nix::r#unistd::r#ftruncate;',
        'nix::sys::stat::fchmod(&file, mode);',
        'use nix::sys::stat::fchmod as change_mode;',
        'use ::nix::{sys::{stat::{futimens as change_times}}};',
        'use r#nix::r#sys::r#stat::r#fchmodat;',
        'use nix::unistd::{fchown as change_owner};',
        'use nix::unistd::{symlinkat as create_link};',
        'use nix::unistd::{unlinkat as remove_link};',
        'use nix::fcntl::renameat2 as move_file;',
        'use nix::{fcntl::{renameat as move_file}};',
        'use nix::unistd::{self as unix_io, read};',
        'use nix::sys::stat as file_metadata;',
        'use nix::fcntl as file_control;',
        'use nix::{sys::{stat as metadata}, errno::Errno};',
        'use nix::unistd;',
        'use nix::{fcntl, errno::Errno};',
        'file . r#set_modified(time);',
        'File :: r#set_permissions(&file, mode);',
        'fs :: remove_file ("state");',
        'std::fs::create_dir_all("state");',
        'File::create("state");',
        'let creator = File::create; creator("private");',
        'let creator = std::fs::File::create_new;',
        'let options = File::options;',
        'let changer = File::set_permissions;',
        'type HiddenFile = std::fs::File;',
        'type HiddenFile = File;',
        'type HiddenFile = ::std::fs::File;',
        'type HiddenFile = fs::File;',
        'type λ = r#std::r#fs::r#File;',
        'File::create_new("state");',
        'use std::fs::write as hidden_write;',
        'use std::{fs::{read, write as hidden_write}};',
        'use std::fs as disk;',
        'use r#std::r#fs as disk;',
        'use r#std::r#fs::r#File as Writable;',
        'use std::{io, fs as disk};',
        'use :: std::{fs as disk};',
        'use std::fs::File as Writable;',
        'use std::fs::{File as Writable, Metadata};',
        'use std::{fs::{File as Writable}, fmt};',
        'use std::fs::{self as disk, Metadata};',
        'use std::os::unix::fs::symlink;',
        'std::os::windows::fs::symlink_file("src", "dest");',
        'std::os::windows::fs::symlink_dir("src", "dest");',
        'use std::os::windows::fs::symlink_file as file_link;',
        'use std::os::windows::fs::{symlink_dir as dir_link};',
        'use ::std::{os::{windows::{fs::{symlink_file as link}}}};',
        'use r#std::r#os::r#windows::r#fs::r#symlink_dir;',
        'file.seek_write(b"private", 0);',
        'FileExt::seek_write(&file, b"private", 0);',
        'file.r#seek_write(b"private", 0);',
        'File::options().write(true);',
        'file.set_len(0);',
        'file.set_permissions(mode);',
        'file.set_modified(time);',
        'file.set_times(times);',
        'File::set_len(&file, 0);',
        'File::set_permissions(&file, mode);',
        'File::set_times(&file, times);',
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
        'use std::fs;',
        'use std::os::unix::fs::FileExt as _;',
        'use std::os::unix::fs::MetadataExt as _;',
        'use crate::fs as disk;',
        'File::open("state");',
        'type Local = crate::File;',
        'type Local = std::fs::Metadata;',
        'type Local = FileInfo;',
        'file.metadata();',
        'file.sync_all();',
        'fs::read("state");',
        'std::fs::r#read("state");',
        'file.r#metadata();',
        'file.read_at(bytes, 0);',
        'file.read_exact_at(bytes, 0);',
        'use std::os::windows::fs::FileExt as _;',
        'use std::os::windows::fs::MetadataExt as _;',
        'file.seek_read(bytes, 0);',
        'FileExt::seek_read(&file, bytes, 0);',
        'let read_only = FileExt::read_at;',
        'let read_only = <File as std::os::unix::fs::FileExt>::read_at;',
        'let read_only = <File as std::os::unix::fs::FileExt>::read_exact_at;',
        'std::fs::symlink_metadata("state");',
        'use nix::unistd::read as input;',
        'use nix::unistd::{read, getpid};',
        'use nix::sys::stat::{fstatat, Mode};',
        'use nix::fcntl::{OpenHow, OFlag, openat2};',
        'nix::sys::stat::fstatat(&dir, path, flags);',
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

test("Unix-domain socket detector covers common Rust constructors", () => {
    for (const source of [
        "use std::os::unix::net::UnixStream;",
        "use std::r#os::r#unix::r#net::UnixStream;",
        "use r#std::{r#os::r#unix::r#net::UnixDatagram};",
        "use ::std::{os::unix::net::UnixListener};",
        "use std::os::{unix::net::UnixDatagram};",
        "use std::os::unix::{net::UnixStream};",
        "use ::std::{os::{unix::{net::UnixStream}}};",
        "std::os::unix::net::UnixDatagram::unbound();",
        'UnixStream::connect("/run/private.sock");',
        'UnixListener::bind("/run/private.sock");',
        "UnixDatagram::pair();",
    ]) {
        assert.equal(usesUnixDomainNetworking(source), true, source);
    }
    for (const source of [
        "use std::os::unix::fs::PermissionsExt;",
        "use std::{os::unix::fs::MetadataExt, fmt};",
        "use std::os::fd::AsRawFd;",
        "use crate::net::UnixDatagram;",
    ]) {
        assert.equal(usesUnixDomainNetworking(source), false, source);
    }
});

test("backend Unix-domain socket authority needs explicit review", async () => {
    const socketFiles = [];
    for (const sourcePath of await rustSources(BACKEND_ROOT)) {
        const source = await readFile(sourcePath, "utf8");
        if (usesUnixDomainNetworking(source)) {
            socketFiles.push(sourcePath);
        }
    }
    assert.deepEqual(socketFiles, [],
        "Unix-domain socket creation is not admitted in backend source");
});

test("standard-library root aliases require explicit review", () => {
    for (const source of [
        "use std as platform;",
        "use core as platform;",
        "extern crate core as platform;",
        "use core::{self as platform, fmt};",
        "use ::core::{fmt, self as platform};",
        "use r#std as platform;",
        "use ::r#std as platform;",
        "extern crate r#std as platform;",
        "use r#std::{self as platform, fmt};",
        "pub use ::std as platform;",
        "extern crate std as platform;",
        "use std::{self as platform, fmt};",
        "use std::{fmt, self as platform};",
        "use std::{io::{Read, Write}, self as platform};",
        "use std as λ;",
        "use std::{self as λ, io};",
        "use :: std as alias;",
    ]) {
        assert.equal(renamesStandardLibraryRoot(source), true, source);
    }
    for (const source of [
        "use std::fmt;",
        "use std::{fmt, io};",
        "extern crate std;",
        "use std::{io::{self as io_alias}, fmt};",
        "use crate::std as domain_model;",
    ]) {
        assert.equal(renamesStandardLibraryRoot(source), false, source);
    }
});

test("backend never renames the standard-library root", async () => {
    const aliasedFiles = [];
    for (const sourcePath of await rustSources(BACKEND_ROOT)) {
        const source = await readFile(sourcePath, "utf8");
        if (renamesStandardLibraryRoot(source)) {
            aliasedFiles.push(sourcePath);
        }
    }
    assert.deepEqual(aliasedFiles, [],
        "renaming std would bypass reviewed network/process/file guards");
});

test("native authority detector covers kernel escape hatches", () => {
    for (const source of [
        "use nix::libc::socket;",
        "use r#nix::r#libc::socket;",
        "use nix::r#sys::r#socket::socket;",
        "nix :: libc :: sendto(fd, data, flags, addr, len);",
        "use nix::libc as native;",
        "use ::nix::{libc as native, errno::Errno};",
        "use nix::{errno::Errno, libc};",
        "use ::libc as native;",
        "extern crate libc as raw;",
        "extern crate nix as native;",
        "use nix as native;",
        "use nix::{self as native, errno::Errno};",
        "use nix::sys as native_sys;",
        "use ::r#nix::r#sys as native_sys;",
        "use nix::sys;",
        "use nix::{sys, errno::Errno};",
        "use nix::{sys as native_sys, errno::Errno};",
        "use nix::sys::{self as native_sys, stat::fstatat};",
        "use nix::{sys::{self as native_sys, stat::Mode}};",
        "use nix::sys::{self, stat::Mode};",
        "nix::sys::socket::socket(domain, ty, flags, None);",
        "use nix::sys::{socket as sockets, stat};",
        "use nix::{sys::socket::socket, errno::Errno};",
        // These wrappers gain kernel inspection capability only in test builds.
        // Production source must never begin depending on that feature union.
        "use nix::sys::ptrace;",
        "use r#nix::r#sys::r#ptrace as debugger;",
        "nix::sys::ptrace::seize(pid, Options::empty());",
        "use nix::sys::uio::{process_vm_readv, RemoteIoVec};",
        "use nix::sys::{ptrace as debugger, prctl};",
        "use nix::sys::{uio::process_vm_writev, stat};",
        "use nix::{sys::ptrace::attach, errno::Errno};",
        "use nix::{sys::{uio::process_vm_readv}, errno::Errno};",
        'unsafe extern "C" { fn socket(); }',
        'unsafe extern "C-unwind" { fn socket(); }',
        'unsafe extern { fn socket(); }',
        'extern "system" fn native_entry() {}',
        'use core::arch::asm as native;',
        'std::arch::asm!("syscall");',
        'global_asm!("syscall");',
    ]) {
        assert.equal(usesUnreviewedNativeAuthority(source), true, source);
    }
    for (const source of [
        "use nix::errno::Errno;",
        "use nix::fcntl::{OFlag, openat2};",
        "use nix::sys::stat::{SFlag, fstatat};",
        "use nix::sys::prctl::{set_dumpable, get_dumpable};",
        "use nix::{sys::stat::Mode, errno::Errno};",
        "use std::os::fd::AsRawFd;",
        "use std::ffi::OsStr;",
        'fn interface() { let syscall = "unavailable"; }',
        'let socket_addr = "127.0.0.1:80";',
    ]) {
        assert.equal(usesUnreviewedNativeAuthority(source), false, source);
    }
});

test("backend raw native authority requires explicit review", async () => {
    const nativeFiles = [];
    for (const sourcePath of await rustSources(BACKEND_ROOT)) {
        const source = await readFile(sourcePath, "utf8");
        let reviewed = source;
        if (sourcePath === PRINT_CLI) {
            // The CLI's nix dev-dependency is referenced only in its test
            // build, not through any production raw native API.
            const site = "#[cfg(all(test, unix))]\nuse nix as _;";
            assert.equal(source.split(site).length - 1, 1);
            reviewed = source.replace(site, "");
        }
        if (usesUnreviewedNativeAuthority(reviewed)) {
            nativeFiles.push(sourcePath);
        }
    }
    assert.deepEqual(nativeFiles, [],
        "libc, native extern, and assembly bypass reviewed Rust authority");
});

// The Linux launcher is the single reviewed owner of native group signalling.
// A new direct signal import elsewhere must trigger security review, even if
// the Cargo feature is already unified through the browser-launch package.
function usesNativeSignalAuthority(source) {
    source = normalizeRustRawIdentifiers(source);
    const groupedImport = new RegExp(
        String.raw`\buse\s+(?:::)?\s*nix\s*::\s*\{[^;]*` +
        String.raw`\bsys\s*::\s*(?:\{[^;]*\bsignal\b|signal\b)`,
        "u",
    );
    return /\bnix\s*::\s*sys\s*::\s*signal\b/u.test(source)
        || groupedImport.test(source);
}

test("native Linux signalling is confined to the browser opener", async () => {
    for (const source of [
        "use nix::sys::signal::{killpg, Signal};",
        "use r#nix::r#sys::r#signal as signals;",
        "nix::sys::signal::killpg(group, SIGKILL);",
        "use nix::{sys::{signal as signals}, errno::Errno};",
        "use nix::{sys::signal::{killpg, Signal}, errno::Errno};",
    ]) {
        assert.equal(usesNativeSignalAuthority(source), true, source);
    }
    assert.equal(usesNativeSignalAuthority(
        "use nix::sys::stat::Mode;",
    ), false);

    const owners = [];
    for (const sourcePath of await rustSources(BACKEND_ROOT)) {
        const source = await readFile(sourcePath, "utf8");
        if (usesNativeSignalAuthority(source)) {
            owners.push(sourcePath);
        }
    }
    assert.deepEqual(owners, [BROWSER_LAUNCH],
        "new Linux process-signal authority needs an explicit owner review");
    const launcher = normalizeRustRawIdentifiers(
        await readFile(BROWSER_LAUNCH, "utf8"),
    );
    assert.equal([...launcher.matchAll(/\bkillpg\s*\(/gu)].length, 1,
        "one reviewed Linux group-kill call");
    assert.equal([...launcher.matchAll(/\bprocess_group\s*\(/gu)].length, 1,
        "one reviewed Linux group-creation call");
});

// A later PR_SET_DUMPABLE(true) invalidates the startup memory guard.
// Inspect all backend callers; this conservative source check does not parse
// macro expansions or prove that privileged host tools honor the guard.
function dumpabilityMutationArgs(source) {
    source = normalizeRustRawIdentifiers(source);
    const calls = [...source.matchAll(
        /\bset_dumpable\s*\(\s*([^)]*?)\s*\)/gu,
    )].map((match) => match[1].trim());
    // An imported alias must not hide a later call to set_dumpable(true).
    const aliases = [...source.matchAll(
        /\bset_dumpable\s+as\s+[a-zA-Z_][a-zA-Z_0-9]*/gu,
    )].map(() => "aliased");
    return [...calls, ...aliases];
}

test("backend has exactly one production dumpability mutation", async () => {
    assert.deepEqual(dumpabilityMutationArgs(
        "prctl::set_dumpable(false); prctl::set_dumpable(true);",
    ), ["false", "true"]);
    assert.deepEqual(dumpabilityMutationArgs(
        "prctl::get_dumpable(); prctl::set_dumpable(0 == 1);",
    ), ["0 == 1"]);
    assert.deepEqual(dumpabilityMutationArgs("prctl::get_dumpable();"), []);
    assert.deepEqual(dumpabilityMutationArgs(
        "use nix::sys::prctl::{set_dumpable as enable}; enable(true);",
    ), ["aliased"]);

    const mutations = [];
    for (const sourcePath of await rustSources(BACKEND_ROOT)) {
        const source = await readFile(sourcePath, "utf8");
        for (const argument of dumpabilityMutationArgs(source)) {
            mutations.push([path.relative(BACKEND_ROOT, sourcePath), argument]);
        }
    }
    assert.deepEqual(mutations, [
        [path.join("runtime-bootstrap", "composition", "main.rs"), "false"],
    ], "the Linux crash-memory guard must never be re-enabled");
});

// A raw inherited descriptor can be adopted as a File or socket without an
// fs::open, stdin(), or std::net constructor in the adopting module. This is
// distinct from observing an existing descriptor via AsRawFd or AsFd.
function usesUnreviewedRawDescriptorAuthority(source) {
    source = normalizeRustRawIdentifiers(source);
    const rawType = new RegExp(
        String.raw`\b(?:FromRawFd|FromRawHandle|FromRawSocket|from_raw_fd|` +
            String.raw`from_raw_handle|from_raw_socket|borrow_raw|BorrowedFd|` +
            String.raw`BorrowedHandle|BorrowedSocket|OwnedFd|OwnedHandle|` +
            String.raw`OwnedSocket)\b`,
        "u",
    );
    if (rawType.test(source)) {
        return true;
    }
    // A renamed whole os::fd or platform io namespace hides its raw-ownership
    // constructors behind arbitrary identifiers in later source.
    for (const match of source.matchAll(
        /\buse\s+(?:::\s*)?std\s*::\s*([^;]+);/gu,
    )) {
        const imported = match[1].replace(/[{}\s]/gu, "");
        const rawModule = /\bos::(?:fd|unix::io|windows::io)/u;
        const wholeModule = /(?:fd|io)(?:as[^,:]+|::selfas[^,:]+|,|$)/u;
        if (rawModule.test(imported) && wholeModule.test(imported)) {
            return true;
        }
    }
    return false;
}

test("raw descriptor inventory rejects unreviewed host FD adoption", () => {
    for (const source of [
        'use std::os::fd::FromRawFd;',
        'use r#std::r#os::r#fd::r#FromRawFd as _;',
        'use ::std::{os::{fd::{FromRawFd as Adopt}}};',
        'let file = unsafe { File::from_raw_fd(0) };',
        'let owned = unsafe { OwnedFd::from_raw_fd(0) };',
        'let input = unsafe { BorrowedFd::borrow_raw(0) };',
        'let input = unsafe { BorrowedFd::borrow_raw/* x */(0) };',
        'use std::os::fd as borrowed_descriptor;',
        'use std::os::fd::{self as borrowed_descriptor, AsRawFd};',
        'use std::{os::{fd as borrowed_descriptor}, fmt};',
        'use std::os::unix::io as inherited;',
        'use ::std::{os::{windows::io as inherited}};',
        'use std::os::windows::io::FromRawHandle;',
        'use std::os::windows::io::FromRawSocket;',
        'unsafe { File::from_raw_handle(handle) }',
        'unsafe { TcpStream::from_raw_socket(socket) }',
        'unsafe { BorrowedHandle::borrow_raw(handle) }',
    ]) {
        assert.equal(usesUnreviewedRawDescriptorAuthority(source), true,
            source);
    }
    for (const source of [
        'use std::os::fd::AsRawFd;',
        'use std::os::fd::RawFd;',
        'use std::os::fd::{AsFd, AsRawFd};',
        'use std::os::unix::io::AsRawFd;',
        'use std::os::windows::io::AsRawHandle;',
        'file.as_raw_fd();',
        'file.as_fd();',
        'file.into_raw_fd();',
        'use std::{os::unix::fs::MetadataExt, fmt};',
        'let output = buffer.borrow();',
    ]) {
        assert.equal(usesUnreviewedRawDescriptorAuthority(source), false,
            source);
    }
});

test("backend cannot adopt inherited raw descriptors without review",
    async () => {
    const descriptorFiles = [];
    for (const sourcePath of await rustSources(BACKEND_ROOT)) {
        const source = await readFile(sourcePath, "utf8");
        if (usesUnreviewedRawDescriptorAuthority(source)) {
            descriptorFiles.push(path.relative(BACKEND_ROOT, sourcePath));
        }
    }
    assert.deepEqual(descriptorFiles, [],
        "raw descriptor adoption bypasses reviewed host input boundaries");
});

test("backend source inclusion requires explicit review", () => {
    for (const source of [
        'include!("separate.inc");',
        'include!{"separate.inc"}',
        'include!["separate.inc"];',
        'include/* comment */!("separate.inc");',
        'use std::include as hidden; hidden!("separate.inc");',
        'use core::include as hidden; hidden!("separate.inc");',
        'use ::std::{include as hidden};',
        'use ::core::{include as hidden};',
        'use r#std::r#include as hidden;',
        'include ! ( concat!(env!("OUT_DIR"), "/generated.rs") );',
        '#[path = "../../unreviewed.rs"] mod external;',
        '# [ path = "../../unreviewed.rs" ] mod external;',
        '#[cfg_attr(target_os = "linux", path = "other.rs")] mod extra;',
    ]) {
        assert.equal(redirectsBackendRustSource(source), true, source);
    }
    for (const source of [
        'include_bytes!("workspace.html");',
        'include_str!("notes.txt");',
        '#[cfg(target_os = "linux")] mod local;',
        'mod semantic_model;',
    ]) {
        assert.equal(redirectsBackendRustSource(source), false, source);
    }
});

test("backend Rust sources cannot redirect compilation out of inventory",
    async () => {
    const auditedPaths = new Map([
        [PAPER_APPLICATION, '#[cfg(test)]\n' +
            '#[path = "../../../../tests/backend/' +
            'notebook-paper-cli/composition/lib.rs"]\nmod tests;'],
        [PAPER_CRATE_ROOT,
            '#[path = "../application/lib.rs"]\npub mod application;'],
        [NOTEBOOK_PROFILE, '#[cfg(test)]\n' +
            '#[path = "../../../../tests/backend/' +
            'notebook-profile/composition/lib.rs"]\nmod tests;'],
        [PROFILE_FLOW, '#[cfg(test)]\n' +
            '#[path = "../../../../tests/backend/' +
            'profile-editor-cli/composition/flow.rs"]\nmod source_tests;'],
        [PROFILE_EDITOR_CLI, '#[cfg(test)]\n' +
            '#[path = "../../../../tests/backend/' +
            'profile-editor-cli/composition/lib.rs"]\nmod tests;'],
        [PRINT_CLI, '#[cfg(test)]\n' +
            '#[path = "../../../../tests/backend/' +
            'notebook-paper-cli/composition/print.rs"]\nmod tests;'],
    ]);
    const redirectFiles = [];
    for (const sourcePath of await rustSources(BACKEND_ROOT)) {
        const source = await readFile(sourcePath, "utf8");
        const site = auditedPaths.get(sourcePath);
        let reviewed = source;
        if (site !== undefined) {
            // Each approved source may select only its exact reviewed module;
            // a changed path or extra source inclusion must fail closed.
            assert.equal(source.split(site).length - 1, 1,
                `reviewed Rust inclusion changed: ${sourcePath}`);
            reviewed = source.replace(site, "");
        }
        if (redirectsBackendRustSource(reviewed)) {
            redirectFiles.push(sourcePath);
        }
    }
    assert.deepEqual(redirectFiles, [],
        "extra included Rust code needs source-level privacy review");
});

// Build-time embedding can disclose arbitrary local files or environment
// values through the shipped binary without any runtime filesystem call.
// Keep its reviewed source/asset inventory exact.
function unreviewedBuildTimeInputs(source, sourcePath) {
    source = normalizeRustRawIdentifiers(source);
    // Reject macro aliases, including grouped imports from core and std.
    // Aliases would otherwise obscure the embedded input's actual macro.
    const aliasedMacro = new RegExp(
        String.raw`\buse\s+(?:::)?(?:std|core)\s*::[^;]*` +
            String.raw`\b(?:include_bytes|include_str|option_env)\b`,
        "u",
    );
    if (aliasedMacro.test(source)
        || /\buse\s+(?:::)?(?:std|core)\s*::[^;]*\benv\s+as\s+\w+/u
            .test(source)) {
        return true;
    }
    // Rust permits comments between a macro name, bang, and delimiter.
    // Treat those comments as lexical whitespace, not as an escape hatch.
    const rustTrivia = String.raw`(?:\s|/\*[\s\S]*?\*/|//[^\n]*\n)*`;
    const invocation = new RegExp(
        String.raw`\b(?:include_bytes|include_str|env|option_env)` +
            rustTrivia + String.raw`!` + rustTrivia + String.raw`[([{]`,
        "gu",
    );
    const invoked = [...source.matchAll(invocation)];
    const quotedMacro = new RegExp(
        String.raw`\b(?:include_bytes|include_str|env|option_env)` +
            String.raw`\s*!\s*\(\s*"[^"\r\n]*"\s*\)`,
        "gu",
    );
    const literals = [...source.matchAll(quotedMacro)];
    // Dynamic paths such as concat!(env!("OUT_DIR"), ...) are never admitted.
    if (invoked.length !== literals.length) {
        return true;
    }
    const actual = literals.map((match) => match[0].replace(/\s+/gu, ""))
        .sort();
    let allowed = [];
    const assetRoot = "../../../browser/workspace/adapter-inbound/";
    if (sourcePath === LOOPBACK_RUNTIME) {
        allowed = [
            "index.html", "workspace.css", "generated/main.js",
            "generated/session-diagnostic.js", "generated/session-draft.js",
            "generated/session-fragment.js", "generated/session-handshake.js",
            "generated/paper-profile.js", "generated/handwriting-profile.js",
            "generated/glyph-strokes.js", "generated/markdown-export.js",
        ].map((asset) =>
            `include_bytes!("${assetRoot}${asset}")`
        );
    } else if (sourcePath === path.join(BACKEND_ROOT,
        "session-handshake", "application", "lib.rs")) {
        allowed = ['option_env!("CARGO_PKG_VERSION")'];
    }
    return JSON.stringify(actual) !== JSON.stringify(allowed.sort());
}

test("backend build-time embed inventory refuses undeclared inputs", () => {
    const versionSource = path.join(BACKEND_ROOT, "session-handshake",
        "application", "lib.rs");
    const assets = [
        "index.html", "workspace.css", "generated/main.js",
        "generated/session-diagnostic.js", "generated/session-draft.js",
        "generated/session-fragment.js", "generated/session-handshake.js",
        "generated/paper-profile.js", "generated/handwriting-profile.js",
            "generated/glyph-strokes.js", "generated/markdown-export.js",
    ];
    const assetRoot = "../../../browser/workspace/adapter-inbound/";
    const approvedAssets = assets.map((asset) =>
        `include_bytes!("${assetRoot}${asset}")`
    ).join(";\n");
    assert.equal(unreviewedBuildTimeInputs(approvedAssets, LOOPBACK_RUNTIME),
        false);
    assert.equal(unreviewedBuildTimeInputs(
        'option_env!("CARGO_PKG_VERSION")', versionSource), false);
    for (const [source, sourcePath] of [
        [`${approvedAssets}; include_bytes!("/etc/passwd")`,
            LOOPBACK_RUNTIME],
        [`${approvedAssets}; include_bytes!{"/etc/hostname"}`,
            LOOPBACK_RUNTIME],
        [`${approvedAssets}; include_bytes/* comment */!{"/etc/hostname"}`,
            LOOPBACK_RUNTIME],
        [`${approvedAssets}; include_str!["/etc/hostname"]`,
            LOOPBACK_RUNTIME],
        ['include_bytes!("/etc/passwd")', LOOPBACK_RUNTIME],
        ['include_bytes!{"/etc/hostname"}', LOOPBACK_RUNTIME],
        ['include_str!["/etc/hostname"]', LOOPBACK_RUNTIME],
        ['include_str!("/etc/passwd")', versionSource],
        ['env!("PRIVATE_BUILD_SECRET")', versionSource],
        ['env!["PRIVATE_BUILD_SECRET"]', versionSource],
        ['option_env!("PRIVATE_BUILD_SECRET")', versionSource],
        ['option_env!("CARGO_PKG_VERSION"); env!("PRIVATE")',
            versionSource],
        ['option_env!("CARGO_PKG_VERSION"); env!["PRIVATE"]',
            versionSource],
        ['option_env!("CARGO_PKG_VERSION"); env /* comment */ !["HOME"]',
            versionSource],
        ['include_bytes!(concat!(env!("OUT_DIR"), "/private"))',
            LOOPBACK_RUNTIME],
        ['use std::include_bytes as embed;', versionSource],
        ['use std::env as embedded;', versionSource],
        ['use r#std::r#include_bytes as embed;', versionSource],
        ['use core::{include_str as embed};', versionSource],
    ]) {
        assert.equal(unreviewedBuildTimeInputs(source, sourcePath), true,
            source);
    }
    assert.equal(unreviewedBuildTimeInputs(
        'std::env::var_os("HOME")', path.join(BACKEND_ROOT,
            "session-secret", "adapter-outbound", "lib.rs")), false);
});

test("backend build-time file and environment inputs are pinned", async () => {
    const unreviewed = [];
    for (const sourcePath of await rustSources(BACKEND_ROOT)) {
        const source = await readFile(sourcePath, "utf8");
        if (unreviewedBuildTimeInputs(source, sourcePath)) {
            unreviewed.push(path.relative(BACKEND_ROOT, sourcePath));
        }
    }
    assert.deepEqual(unreviewed, [],
        "new compile-time inputs need explicit privacy review");
});

// Pinned nix can rebind stdin/stdout/stderr through descriptor duplication,
// or create IPC pipes whose endpoints may survive a child exec by default.
// Neither authority needs an std::io operation or Unix socket constructor.
// No backend module owns an unreviewed descriptor channel.
function usesNixDescriptorChannelAuthority(source) {
    const channel = String.raw`(?:dup|dup2|dup2_raw|dup2_stdin|` +
        String.raw`dup2_stdout|dup2_stderr|dup3|dup3_raw|pipe|pipe2)`;
    if (new RegExp(
        String.raw`\bnix\s*::\s*unistd\s*::\s*` + channel +
            String.raw`\b`, "u",
    ).test(source)) {
        return true;
    }
    for (const match of source.matchAll(
        /\buse\s+(?:::\s*)?nix\s*::\s*([^;]+);/gu,
    )) {
        const imported = match[1];
        if (/\bunistd\s*(?:as\b|[,}]|$)/u.test(imported)
            || /\bunistd\s*::\s*\{[^;]*\bself\s+as\b/u
                .test(imported)
            || /\bunistd\s*::\s*\*/u.test(imported)
            || new RegExp(String.raw`\bunistd\s*::\s*\{[^;]*\b` +
                channel + String.raw`\b`, "u").test(imported)) {
            return true;
        }
    }
    return false;
}

// Standard streams can disclose session text or read inherited host data
// without filesystem or network APIs. Only the CLI bootstrap's two startup
// and status writes are admitted; all stdin and stderr need source review.
function unreviewedStandardStreamAuthority(source, sourcePath) {
    source = normalizeRustRawIdentifiers(source);
    if (usesNixDescriptorChannelAuthority(source)) {
        return true;
    }
    // Rust macros accept all three delimiters and comments as whitespace.
    const rustTrivia = String.raw`(?:\s|/\*[\s\S]*?\*/|//[^\n]*\n)*`;
    const outputMacro = new RegExp(
        String.raw`\b(?:print|println|eprint|eprintln|dbg)` +
            rustTrivia + String.raw`!` + rustTrivia + String.raw`[([{]`,
        "u",
    );
    const importedMacro = new RegExp(
        String.raw`\buse\s+(?:::)?(?:std|core)\s*::[^;]*` +
            String.raw`\b(?:print|println|eprint|eprintln|dbg)\b`,
        "u",
    );
    const renamedIo = new RegExp(
        String.raw`\buse\s+(?:::)?std\s*::[^;]*\bio\s*` +
            String.raw`(?:as\b|::\s*\{[^;]*\bself\s+as\b)`,
        "u",
    );
    const groupedStream = new RegExp(
        String.raw`\buse\s+(?:::)?std\s*::[^;]*\bio\s*::` +
            String.raw`\s*\{[^;]*\b(?:stdin|stdout|stderr)\b`,
        "u",
    );
    if (outputMacro.test(source) || importedMacro.test(source)
        || renamedIo.test(source) || groupedStream.test(source)) {
        return true;
    }
    const stdout = [...source.matchAll(
        /\b(?:std\s*::\s*)?io\s*::\s*stdout\b/gu,
    )].length;
    const stderr = [...source.matchAll(
        /\b(?:std\s*::\s*)?io\s*::\s*stderr\b/gu,
    )].length;
    const stdin = [...source.matchAll(
        /\b(?:std\s*::\s*)?io\s*::\s*stdin\b/gu,
    )].length;
    if (sourcePath === PAPER_CLI) {
        return stderr !== 0 || stdin !== 0 || stdout !== 2
            || !source.includes('writeln!(io::stdout().lock(), "{USAGE}")');
    }
    if (sourcePath === PRINT_CLI) {
        // Help, reviewed digest and explicit CUPS acceptance: no stderr.
        return stderr !== 0 || stdin !== 0 || stdout !== 3
            || !source.includes('writeln!(io::stdout().lock(), "{USAGE}")')
            || !source.includes('"PDF validated for {}; SHA-256 {}; '
                + 'no job submitted"')
            || !source.includes('"CUPS accepted a job for printer {}"');
    }
    if (sourcePath === PROFILE_EDITOR_CLI) {
        // Seven explicit outputs after shared glyph/handwriting help,
        // and one selected, bounded stdin reader.
        return stderr !== 0 || stdin !== 1 || stdout !== 7
            || !reviewedProfileCliStdinReader(source)
            || !source.includes('writeln!(io::stdout().lock(), "{USAGE}")')
            || !source.includes(
                'writeln!(io::stdout().lock(), "{NOTEBOOK_USAGE}")'
            )
            || !source.includes("io::stdout().lock().write_all(&bytes)");
    }
    const expectedStdout = sourcePath === path.join(BACKEND_ROOT,
        "runtime-bootstrap", "composition", "main.rs") ? 2 : 0;
    return stderr !== 0 || stdin !== 0 || stdout !== expectedStdout;
}

test("standard stream inventory blocks unreviewed host I/O", () => {
    const bootstrap = path.join(BACKEND_ROOT, "runtime-bootstrap",
        "composition", "main.rs");
    const privateModule = path.join(BACKEND_ROOT, "session-secret",
        "adapter-outbound", "lib.rs");
    assert.equal(unreviewedStandardStreamAuthority(
        "io::stdout(); io::stdout();", bootstrap), false);
    for (const [source, sourcePath] of [
        ['println!("session text")', privateModule],
        ['eprintln!("session text")', privateModule],
        ['dbg!(secret)', privateModule],
        ['print!{"private"}', privateModule],
        ['eprint!["private"]', privateModule],
        ['use std::println as leak;', privateModule],
        ['use ::std::{eprintln as leak};', privateModule],
        ['use core::dbg as leak;', privateModule],
        ['std::io::stderr()', privateModule],
        ['std::io::stdin()', privateModule],
        ['io::stdin().lock()', privateModule],
        ['use std::io::stdin as input;', privateModule],
        ['use std::io::{stdin as input};', privateModule],
        ['use ::std::{io::{stdin as input}};', privateModule],
        ['use r#std::r#io::r#stdin as input;', privateModule],
        ['io::stdout()', privateModule],
        ['use std::io::stderr as leak;', privateModule],
        ['use std::io::{stdout as leak};', privateModule],
        ['use ::std::{io::{stderr as leak}};', privateModule],
        ['use std::io as hidden;', privateModule],
        ['use std::{io as hidden};', privateModule],
        ['use std::io::{self as hidden, Write};', privateModule],
        // Inherited Unix pipes create new IPC channels without sockets.
        ['nix::unistd::pipe();', privateModule],
        ['let creator = nix::unistd::pipe;', privateModule],
        ['nix::unistd::pipe2(OFlag::O_CLOEXEC);', privateModule],
        ['let creator = nix::unistd::pipe2;', privateModule],
        ['use nix::unistd::pipe as channel;', privateModule],
        ['use nix::unistd::pipe2 as channel;', privateModule],
        ['use ::nix::{unistd::{pipe2 as channel}};', privateModule],
        ['use r#nix::r#unistd::r#pipe as channel;', privateModule],
        ['nix/**/::unistd::pipe();', privateModule],
        ['nix::unistd::dup2_stdout(&file);', privateModule],
        ['use nix::unistd::dup2_stderr as redirect;', privateModule],
        ['use nix::unistd::{dup2_stdin as redirect};', privateModule],
        ['use ::nix::{unistd::{dup3_raw as redirect}};', privateModule],
        ['use r#nix::r#unistd::r#dup2 as redirect;', privateModule],
        ['use nix::unistd as descriptor_ops;', privateModule],
        ['use nix::{unistd as descriptor_ops};', privateModule],
        ['io::stdout(); io::stdout(); io::stdout();', bootstrap],
        ['io::stdout(); io::stdout(); eprintln!("private")', bootstrap],
        ['io::stdout(); io::stdout(); io::stdin()', bootstrap],
    ]) {
        assert.equal(
            unreviewedStandardStreamAuthority(source, sourcePath),
            true, source,
        );
    }
    assert.equal(unreviewedStandardStreamAuthority('format!("visible")',
        privateModule), false);
    for (const source of [
        'use nix::unistd::getpid;',
        'use nix::unistd::read as input;',
        'nix::unistd::read(&file, bytes);',
        'use nix::sys::stat::Mode;',
        'use std::os::fd::AsRawFd;',
    ]) {
        assert.equal(unreviewedStandardStreamAuthority(source, privateModule),
            false, source);
    }
});

test("backend standard-stream policy stays pinned", async () => {
    const unreviewed = [];
    for (const sourcePath of await rustSources(BACKEND_ROOT)) {
        const source = await readFile(sourcePath, "utf8");
        if (unreviewedStandardStreamAuthority(source, sourcePath)) {
            unreviewed.push(path.relative(BACKEND_ROOT, sourcePath));
        }
    }
    assert.deepEqual(unreviewed, [],
        "new backend standard-stream authority needs explicit review");
});

// Cargo can point a new library or executable at an arbitrary .rs file.
// Without this admission, the backend file inventory can miss that code even
// with an unchanged dependency set and no build.rs target. Resolve paths and
// symbolic links before deciding whether a target stays in the scanned tree.
function unreviewedCargoSourceTargets(metadata, resolveSource = realpathSync) {
    const owners = new Set(metadata.workspace_members);
    const backend = `${realpathSync(BACKEND_ROOT)}${path.sep}`;
    const tests = `${realpathSync(path.join(REPOSITORY_ROOT,
        "tests", "backend"))}${path.sep}`;
    const unreviewed = [];
    for (const pkg of metadata.packages) {
        if (!owners.has(pkg.id)) continue;
        for (const target of pkg.targets) {
            const [kind] = target.kind;
            // Cargo examples are executable fixtures, not shipped adapters.
            // Admit them only under the same canonical test source root.
            const root = target.kind.length !== 1 ? null
                : kind === "test" || kind === "example" ? tests
                : kind === "lib" || kind === "bin" ? backend : null;
            let source = null;
            try {
                source = resolveSource(target.src_path);
            } catch {
                // Missing or unresolved sources cannot prove confinement.
            }
            if (root === null || source === null
                || !source.startsWith(root) || !source.endsWith(".rs")) {
                unreviewed.push(`${pkg.name}:${target.name}`);
            }
        }
    }
    return unreviewed;
}

test("backend source inventory refuses symlinked module files", () => {
    const link = {
        isSymbolicLink: () => true,
    };
    const regular = {
        isSymbolicLink: () => false,
    };
    assert.throws(() => rejectBackendSourceLink(link, "alias.rs"),
        /symbolic links need explicit source review/);
    assert.doesNotThrow(() => rejectBackendSourceLink(regular, "lib.rs"));
});

test("Cargo executable source targets cannot escape reviewed trees", () => {
    const backend = path.join(BACKEND_ROOT, "session-secret",
        "adapter-outbound", "lib.rs");
    const testRoot = path.join(REPOSITORY_ROOT, "tests", "backend",
        "session-secret", "adapter-outbound", "lib.rs");
    const metadata = (kind, source) => ({
        workspace_members: ["owned"],
        packages: [{
            id: "owned",
            name: "test_owner",
            targets: [{
                kind: [kind],
                name: "test_target",
                src_path: source,
            }],
        }],
    });
    for (const [kind, source] of [
        ["lib", backend],
        ["bin", backend],
        ["test", testRoot],
        ["example", testRoot],
        ["test", path.join(BACKEND_ROOT, "session-secret",
            "../../../tests/backend/session-secret/adapter-outbound/lib.rs")],
    ]) {
        assert.deepEqual(unreviewedCargoSourceTargets(metadata(kind, source)),
            [], `${kind}: ${source}`);
    }
    // A symlink may look inside src/backend lexically but resolve outside it.
    const alias = path.join(BACKEND_ROOT, "injected.rs");
    const aliasTarget = (source) => source === alias
        ? testRoot : realpathSync(source);
    assert.deepEqual(unreviewedCargoSourceTargets(
        metadata("lib", alias), aliasTarget,
    ), ["test_owner:test_target"]);
    assert.deepEqual(unreviewedCargoSourceTargets(
        metadata("example", testRoot), () =>
            path.join(BACKEND_ROOT, "session-secret", "injected.rs"),
    ), ["test_owner:test_target"],
    "symlinked Cargo examples outside the test root must still refuse");
    for (const [kind, source] of [
        ["lib", testRoot],
        ["bin", testRoot],
        ["test", backend],
        ["example", backend],
        ["example", path.join(REPOSITORY_ROOT, "README.md")],
        ["example", path.join(BACKEND_ROOT, "missing.rs")],
        ["lib", path.join(REPOSITORY_ROOT, "README.md")],
        ["bin", path.join(BACKEND_ROOT, "missing.rs")],
    ]) {
        assert.deepEqual(unreviewedCargoSourceTargets(metadata(kind, source)),
            ["test_owner:test_target"], `${kind}: ${source}`);
    }
});

test("backend registry dependencies remain explicitly reviewed", () => {
    const cargo = path.join(
        REPOSITORY_ROOT,
        ".dependencies",
        "rust",
        "1.99.1",
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
    assert.deepEqual(unreviewedCargoSourceTargets(metadata), [],
        "Cargo targets must remain inside reviewed Rust source trees");
    const workspaceIds = new Set(metadata.workspace_members);
    const unreviewedBuildScripts = metadata.packages
        .filter((pkg) => workspaceIds.has(pkg.id))
        .flatMap((pkg) => pkg.targets
            .filter((target) => target.kind.includes("custom-build"))
            .map((target) => `${pkg.name}:${target.name}`));
    assert.deepEqual(unreviewedBuildScripts, [],
        "new Cargo build scripts can execute unreviewed native code");
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
            // The only Linux launcher native capability is group signalling
            // on a failed or timed-out opener, not general process control.
            defaultFeatures: false,
            features: ["fs", "signal"],
            kind: null,
            name: "nix",
            optional: false,
            owner: "atrament_browser_launch",
            rename: null,
            requirement: "=0.31.3",
            source: "registry+https://github.com/rust-lang/crates.io-index",
            target: 'cfg(target_os = "linux")',
        },
        {
            // Test-only POSIX FIFO creation exercises nonblocking CLI refusal.
            defaultFeatures: false,
            features: ["fs"],
            kind: "dev",
            name: "nix",
            optional: false,
            owner: "atrament_notebook_paper_cli",
            rename: null,
            requirement: "=0.31.3",
            source: "registry+https://github.com/rust-lang/crates.io-index",
            target: 'cfg(unix)',
        },
        {
            // Test-only POSIX FIFO creation exercises nonblocking CLI refusal.
            defaultFeatures: false,
            features: ["fs"],
            kind: "dev",
            name: "nix",
            optional: false,
            owner: "atrament_profile_editor_cli",
            rename: null,
            requirement: "=0.31.3",
            source: "registry+https://github.com/rust-lang/crates.io-index",
            target: 'cfg(unix)',
        },
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
            defaultFeatures: false,
            features: ["fs", "process"],
            kind: null,
            name: "nix",
            optional: false,
            owner: "atrament_runtime_bootstrap",
            rename: null,
            requirement: "=0.31.3",
            source: "registry+https://github.com/rust-lang/crates.io-index",
            target: 'cfg(target_os = "linux")',
        },
        {
            // process_vm and ptrace attach run in isolated Linux tests;
            // production retains the narrower `process` feature set.
            defaultFeatures: false,
            features: ["fs", "process", "ptrace", "uio"],
            kind: "dev",
            name: "nix",
            optional: false,
            owner: "atrament_runtime_bootstrap",
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
