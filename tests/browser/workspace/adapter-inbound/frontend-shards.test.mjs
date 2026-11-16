// Copyright:
//   - Copyright © 2026 Alberto Villa Osorno.
// SPDX-License-Identifier:
//   - MIT
// Confidential:
//   - true
// License-File:
//   - LICENSE
// Test discovery, rejection, and exact TAP receipts independently of Firefox.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

import { completeBrowserTap, discoverShards } from './frontend-shards.mjs';

const root = fileURLToPath(new URL('../../../../', import.meta.url));
const runner = fileURLToPath(new URL('./frontend-shards.mjs', import.meta.url));
const childEnv = { ...process.env };
delete childEnv.NODE_TEST_CONTEXT;

function run(...args) {
    const result = spawnSync(process.execPath, [runner, ...args], {
        cwd: root, encoding: 'utf8', timeout: 25_000, env: childEnv,
    });
    assert.equal(result.error, undefined, result.error?.message);
    return result;
}

function tap(names) {
    return [
        'TAP version 13',
        ...names.flatMap((name, index) => [
            `# Subtest: ${name}`, `ok ${index + 1} - ${name}`,
        ]),
        `# tests ${names.length}`,
        `# pass ${names.length}`,
        '# fail 0', '# cancelled 0', '# skipped 0', '# todo 0',
    ].join('\n') + '\n';
}

test('browser shards cover every unique test in every file in order', () => {
    const shards = discoverShards();
    assert.ok(shards.length > 15);
    const declared = new Map();
    for (const shard of shards) {
        const existing = declared.get(shard.name) ?? [];
        assert.equal(shard.start, existing.length);
        assert.ok(shard.names.length > 0);
        declared.set(shard.name, [...existing, ...shard.names]);
        assert.equal(new Set(shard.names).size, shard.names.length);
    }
    assert.equal(declared.size, 15);
    for (const shard of shards) {
        const source = readFileSync(shard.file, 'utf8');
        const declarations = [...source.matchAll(/^test\(/gm)];
        assert.equal(declared.get(shard.name).length, declarations.length);
    }
    const listed = run('--list');
    assert.equal(listed.status, 0, listed.stderr);
    const [first, ...lines] = listed.stdout.trimEnd().split('\n');
    const count = shards.reduce((sum, shard) => sum + shard.names.length, 0);
    assert.equal(first, `${count} tests in ${shards.length} browser shards`);
    assert.deepEqual(lines, shards.map((shard, index) =>
        `${index + 1}: ${shard.name}: ${shard.names.join(' | ')}`));
});

test('joined literal names match registered browser tests', () => {
    const names = discoverShards().find((shard) =>
        shard.name === 'handwriting-profile.mjs').names;
    assert.ok(names.includes('native numeric preview refuses malformed'
        + ' response without retaining output'));
    // The case-discovery test above asserts the exact count from source.
    // Do not require fixture maintenance for every added handwriting test.
    assert.ok(names.length >= 15);
});

test('browser shard selector refuses invalid requests', () => {
    for (const args of [
        ['--shard', '0'], ['--shard', '-1'], ['--shard', '1.0'],
        ['--shard', '999999999999999999999'], ['--shard'],
        ['--shard', '1', '--list'], ['--unknown'], ['--all', '--list'],
    ]) {
        const result = run(...args);
        assert.equal(result.status, 2, `${args}: ${result.stderr}`);
        assert.match(result.stderr, /Usage: node frontend-shards\.mjs/);
        assert.doesNotMatch(result.stdout, /Browser shard \d+\//);
    }
});

test('browser TAP rejects empty selections and false counts', () => {
    const valid = tap(['first test', 'second test']);
    assert.equal(
        completeBrowserTap(valid, ['first test', 'second test']), true,
    );
    assert.equal(completeBrowserTap(valid, ['first test']), false);
    assert.equal(
        completeBrowserTap(valid, ['second test', 'first test']), false,
    );
    assert.equal(completeBrowserTap(valid, []), false);
    assert.equal(completeBrowserTap(tap(['file.mjs']), ['real test']), false);
    for (const [from, to] of [
        ['# tests 2', '# tests 1'], ['# pass 2', '# pass 1'],
        ['# fail 0', '# fail 1'], ['# cancelled 0', '# cancelled 1'],
        ['# skipped 0', '# skipped 1'], ['# todo 0', '# todo 1'],
        ['# Subtest: first test', '# Subtest: missing test'],
    ]) {
        assert.equal(completeBrowserTap(valid.replace(from, to),
            ['first test', 'second test']), false, to);
        assert.equal(completeBrowserTap(valid.replace(from, ''),
            ['first test', 'second test']), false, `${from} missing`);
        assert.equal(completeBrowserTap(valid + `${from}\n`,
            ['first test', 'second test']), false, `${from} duplicate`);
    }
});

test('generic Node test discovery cannot recursively launch runner', () => {
    const result = spawnSync(process.execPath, ['--test', runner], {
        cwd: root, encoding: 'utf8', timeout: 10_000, env: childEnv,
    });
    assert.equal(result.error, undefined, result.error?.message);
    assert.equal(result.status, 0, result.stderr);
    assert.doesNotMatch(result.stdout, /Browser shard \d+\//);
    assert.doesNotMatch(result.stdout, /browser TAP rejects/);
});

test('browser shard runs a real exact test receipt', () => {
    const result = run('--shard', '1');
    assert.equal(result.status, 0, result.stderr);
    const shard = discoverShards()[0];
    assert.match(result.stdout, /Browser shard 1\/\d+/);
    assert.equal(completeBrowserTap(result.stdout, shard.names), true);
    assert.match(result.stdout, /Passed 4 selected browser acceptance cases\./);
});
