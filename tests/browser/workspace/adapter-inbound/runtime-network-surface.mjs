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


// Rust accepts `r#` on ordinary identifiers, not just keywords. Canonicalize
// that spelling before source-level authority review; raw string delimiters
// are untouched because they are not followed by an identifier character.
function normalizeRustRawIdentifiers(source) {
    return source.replace(/\br#(?=[A-Za-z_])/gu, "");
}

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

// Renaming the std root can hide net/process/fs uses from the narrow source
// detectors. This guard requires review before adding any root alias.
function renamesStandardLibraryRoot(source) {
    source = normalizeRustRawIdentifiers(source);
    const directAlias = /\buse\s+(?:::\s*)?std\s+as\s+\S+/u;
    const externAlias = /\bextern\s+crate\s+std\s+as\s+\S+/u;
    if (directAlias.test(source) || externAlias.test(source)) {
        return true;
    }
    // A root-level `self as` inside a grouped import also renames std.
    // Ignore nested `io::{self as alias}`: that does not rename std itself.
    for (const match of source.matchAll(/\buse\s+(?:::\s*)?std\s*::\s*\{/gu)) {
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

// A child process can bypass std::net through an external program. Require
// explicit review for all standard-library process authority outside the
// platform browser-launch adapter, including grouped imports and constructors.
function usesStandardLibraryProcessAuthority(source) {
    source = normalizeRustRawIdentifiers(source);
    if (/\bstd\s*::\s*process\b/u.test(source)
        || /\bCommand\s*::\s*new\s*\(/u.test(source)) {
        return true;
    }
    const groupedImports = source.matchAll(
        /\buse\s+(?:::\s*)?std\s*::\s*\{([\s\S]*?)\}\s*;/gu,
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
    source = normalizeRustRawIdentifiers(source);
    // Renaming std::fs or its File constructor hides subsequent mutations
    // behind names the ordinary write-call inventory cannot recognize.
    for (const match of source.matchAll(
        /\buse\s+(?:::\s*)?std\s*::([^;]+);/gu,
    )) {
        const imported = match[1];
        const moduleAlias = /^\s*fs\s+as\b/u.test(imported)
            || /^\s*\{[^;]*\bfs\s+as\b/u.test(imported);
        const fsPath = /^\s*fs\s*::/u.test(imported)
            || /^\s*\{[^;]*\bfs\s*::/u.test(imported);
        const fileAlias = /\b(?:File|self)\s+as\b/u.test(imported);
        if (moduleAlias || (fsPath && fileAlias)) {
            return true;
        }
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
    // FileExt positional writes can mutate a caller-supplied descriptor even
    // when it was opened elsewhere. Reading through the same trait is safe.
    if (/\b(?:write_at|write_all_at|write_vectored_at)\s*\(/u.test(source)) {
        return true;
    }
    // The locked nix fs feature also exposes direct directory, inode and
    // metadata mutations. Check grouped imports, renamed functions, and
    // module aliases, while allowing read-only imports in the same modules.
    const nixMutations = new RegExp(
        String.raw`\b(?:chown|fchown|fchownat|fchmod|fchmodat|ftruncate|` +
            String.raw`futimens|linkat|mkdir|mkdirat|mkfifo|mkfifoat|` +
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
        if (moduleAlias.test(imported) || groupedAlias.test(imported)
            || wholeModule.test(imported)
            || (module.test(imported) && nixMutations.test(imported))) {
            return true;
        }
    }
    // An already-open descriptor can mutate on-disk content or metadata
    // without opening a new writable file. Keep these direct File methods in
    // the same explicit review boundary as File::create and fs::write.
    const fileMutator = new RegExp(
        String.raw`(?:\.|\bFile\s*::)\s*(?:r#)?set_` +
            String.raw`(?:len|modified|permissions|times)\s*\(`,
        "u",
    );
    const unixMutator =
        /\b(?:linkat|unlinkat|renameat|renameat2|mkdirat|mknodat)\s*\(/u;
    const fileConstructor =
        /\bFile\s*::\s*(?:r#)?(?:create|create_new|options)\s*\(/u;
    return fileMutator.test(source)
        || fileConstructor.test(source)
        || /\bOpenOptions\b/u.test(source)
        || /\bO_(?:TMPFILE|CREAT|TRUNC)\b/u.test(source)
        || unixMutator.test(source);
}

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
        /\bnix\s*::\s*sys\s*::\s*socket\b/u,
        /\bnix\s*::\s*sys\s*::\s*\{[^;]*\bsocket\b/u,
        /\bnix\s*::\s*\{[^;]*\bsys\s*::\s*socket\b/u,
        /\b(?:unsafe\s+)?extern\s*(?:"[^"]+"\s*)?(?:\{|fn\b)/u,
        /\b(?:core|std)\s*::\s*arch\b/u,
        /\b(?:asm|global_asm)\s*!/u,
    ];
    return nativePatterns.some((pattern) => pattern.test(source));
}

// Scan all authored backend .rs files; embedding a separate Rust source via
// include! or a redirected module path could bypass that inventory. Binary and
// text asset inclusion remains deliberately separate from Rust compilation.
function redirectsBackendRustSource(source) {
    return /\binclude\s*!\s*\(/u.test(source)
        || /#\s*\[\s*path\s*=/u.test(source)
        || /#\s*\[\s*cfg_attr\s*\(/u.test(source);
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
        'use std::os::unix::fs as unix_fs;',
        'use crate::fs as disk;',
        'File::open("state");',
        'file.metadata();',
        'file.sync_all();',
        'fs::read("state");',
        'std::fs::r#read("state");',
        'file.r#metadata();',
        'file.read_at(bytes, 0);',
        'file.read_exact_at(bytes, 0);',
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

test("native authority detector covers raw syscall escape hatches", () => {
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
        "nix::sys::socket::socket(domain, ty, flags, None);",
        "use nix::sys::{socket as sockets, stat};",
        "use nix::{sys::socket::socket, errno::Errno};",
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
        if (usesUnreviewedNativeAuthority(source)) {
            nativeFiles.push(sourcePath);
        }
    }
    assert.deepEqual(nativeFiles, [],
        "libc, native extern, and assembly bypass reviewed Rust authority");
});

test("backend source inclusion requires explicit review", () => {
    for (const source of [
        'include!("separate.inc");',
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
    const redirectFiles = [];
    for (const sourcePath of await rustSources(BACKEND_ROOT)) {
        const source = await readFile(sourcePath, "utf8");
        if (redirectsBackendRustSource(source)) {
            redirectFiles.push(sourcePath);
        }
    }
    assert.deepEqual(redirectFiles, [],
        "extra included Rust code needs source-level privacy review");
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
