// Copyright:
//   - Copyright © 2026 Alberto Villa Osorno.
// SPDX-License-Identifier:
//   - MIT
// Confidential:
//   - true
// License-File:
//   - LICENSE
// Run browser acceptance as independently bounded, receipt-checked batches.
// Usage: node tests/browser/workspace/adapter-inbound/frontend-shards.mjs
//        --list | --shard N | --all
import { spawnSync } from 'node:child_process';
import { readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../../../../', import.meta.url));
const directory = fileURLToPath(new URL('./', import.meta.url));
const runnerFile = fileURLToPath(import.meta.url);
const excluded = new Set(['frontend-shards.mjs', 'frontend-shards.test.mjs']);
const escapeRegex = (name) => name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');

// The integration file starts a separate browser for many of its cases.
// Keep those batches small, while ordinary pure-Node fixtures run per file.
const browserBatchSize = 4;

export function discoverShards() {
    const files = readdirSync(directory)
        .filter((name) => name.endsWith('.mjs') && !excluded.has(name))
        .sort();
    if (files.length === 0) {
        throw new Error('browser acceptance has no test files');
    }
    const shards = [];
    for (const name of files) {
        const file = join(directory, name);
        const source = readFileSync(file, 'utf8');
        const declarations = [...source.matchAll(/^test\(/gm)];
        const names = [];
        for (const declaration of declarations) {
            // Long test names may be joined from adjacent string literals.
            // Never evaluate an arbitrary source expression to discover names.
            const expression = source.slice(declaration.index).match(
                // jig-ignore-next-line: Keep one atomic test-name grammar.
                /^test\(\s*("(?:[^"\\\n]|\\.)*"(?:\s*\+\s*"(?:[^"\\\n]|\\.)*")*)\s*,/,
            );
            if (!expression) {
                throw new Error(name + ' needs literal top-level test names');
            }
            const chunks = [...expression[1].matchAll(
                /"(?:[^"\\\n]|\\.)*"/g,
            )].map((match) => JSON.parse(match[0]));
            names.push(chunks.join(''));
        }
        if (names.length === 0 || names.some((testName) => !testName)
            || new Set(names).size !== names.length) {
            throw new Error(name + ' needs unique top-level test names');
        }
        const batchSize = name === 'runtime-frame-ancestors.mjs'
            ? browserBatchSize : names.length;
        for (let start = 0; start < names.length; start += batchSize) {
            shards.push({ file, name, start,
                names: names.slice(start, start + batchSize) });
        }
    }
    return shards;
}

// Node may exit 0 with '# tests 1' for an empty name filter: its sole
// 'passing test' is then the containing file, not an authored test. Check
// each exact selected top-level name as well as complete TAP counters.
export function completeBrowserTap(stdout, expected) {
    if (expected.length === 0) {
        return false;
    }
    const actual = [...stdout.matchAll(/^# Subtest: (.+)\r?$/gm)]
        .map((match) => match[1]);
    if (actual.length !== expected.length
        || actual.some((name, index) => name !== expected[index])) {
        return false;
    }
    const count = (label) => {
        const rows = [...stdout.matchAll(
            new RegExp(`^# ${label} (\\d+)\\r?$`, 'gm'),
        )];
        return rows.length === 1 ? Number(rows[0][1]) : null;
    };
    return count('tests') === expected.length
        && count('pass') === expected.length
        && count('fail') === 0 && count('cancelled') === 0
        && count('skipped') === 0 && count('todo') === 0;
}

function run() {
    const shards = discoverShards();
    const args = process.argv.slice(2);
    if (args.length === 1 && args[0] === '--list') {
        const cases = shards.reduce(
            (sum, shard) => sum + shard.names.length, 0,
        );
        console.log(`${cases} tests in ${shards.length} browser shards`);
        for (const [index, shard] of shards.entries()) {
            console.log(`${index + 1}: ${shard.name}: `
                + shard.names.join(' | '));
        }
        return;
    }
    const selected = args.length === 0
        || (args.length === 1 && args[0] === '--all') ? null
            : args.length === 2 && args[0] === '--shard'
                && /^[1-9]\d*$/.test(args[1])
                ? Number(args[1]) : NaN;
    if (Number.isNaN(selected) || (selected !== null
        && (!Number.isSafeInteger(selected) || selected > shards.length))) {
        console.error('Usage: node frontend-shards.mjs'
            + ` [--all | --list | --shard N] (1 <= N <= ${shards.length})`);
        process.exitCode = 2;
        return;
    }
    if (selected === null) {
        const regression = spawnSync(process.execPath, [
            '--test', fileURLToPath(new URL('./frontend-shards.test.mjs',
                import.meta.url)),
        ], { cwd: root, stdio: 'inherit', timeout: 30_000 });
        if (regression.status !== 0 || regression.error) {
            console.error('Browser runner regression checks failed:',
                regression.error?.message ?? regression.signal
                    ?? regression.status);
            process.exitCode = 1;
            return;
        }
    }
    const selectedShards = selected === null
        ? shards.map((_, index) => index) : [selected - 1];
    let passed = 0;
    for (const index of selectedShards) {
        const shard = shards[index];
        const pattern = `^(?:${shard.names.map(escapeRegex).join('|')})$`;
        console.log(`Browser shard ${index + 1}/${shards.length}`
            + ` (${shard.name}, ${shard.names.length} cases)`);
        const result = spawnSync(process.execPath, [
            '--test', '--test-reporter=tap', '--test-concurrency=1',
            '--test-name-pattern', pattern, shard.file,
        ], { cwd: root, encoding: 'utf8', timeout: 240_000,
            maxBuffer: 8 * 1024 * 1024 });
        process.stdout.write(result.stdout ?? '');
        process.stderr.write(result.stderr ?? '');
        const complete = completeBrowserTap(result.stdout ?? '', shard.names);
        if (result.status !== 0 || result.error || !complete) {
            console.error(`Browser shard ${index + 1} failed:`,
                result.error?.message ?? result.signal ?? result.status,
                complete ? '' : 'incomplete or mismatched TAP receipt');
            process.exitCode = 1;
            return;
        }
        passed += shard.names.length;
    }
    console.log(`Passed ${passed} selected browser acceptance cases.`);
}

// Generic node --test file glob must not recursively launch this runner.
if (process.argv[1] === runnerFile && !process.env.NODE_TEST_CONTEXT) {
    run();
}
