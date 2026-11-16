// Copyright:
//   - Copyright © 2026 Alberto Villa Osorno.
// SPDX-License-Identifier:
//   - MIT
// Confidential:
//   - true
// License-File:
//   - LICENSE
// Validate test discovery and refusal without running expensive PDF/PNG tests.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

import { completeShardTap } from './flow-shards.mjs';

const root = fileURLToPath(new URL('../../../../', import.meta.url));
const runner = fileURLToPath(new URL('./flow-shards.mjs', import.meta.url));
const flow = fileURLToPath(new URL('./flow.mjs', import.meta.url));

// Nested node --test inherits this marker and refuses to run any files.
// The acceptance child is intentionally independent of this test harness.
const childEnv = { ...process.env };
delete childEnv.NODE_TEST_CONTEXT;

function run(...args) {
    const result = spawnSync(process.execPath, [runner, ...args], {
        cwd: root, encoding: 'utf8', timeout: 20_000, env: childEnv,
    });
    assert.equal(result.error, undefined, result.error?.message);
    return result;
}

test('flow shard listing covers each literal source test once in order', () => {
    const source = readFileSync(flow, 'utf8');
    const declared = [...source.matchAll(/^test\('([^'\n]+)'/gm)]
        .map((match) => match[1]);
    assert.ok(declared.length > 0);
    assert.equal([...source.matchAll(/^test\(/gm)].length, declared.length);
    assert.equal(new Set(declared).size, declared.length);

    const listed = run('--list');
    assert.equal(listed.status, 0, listed.stderr);
    const [heading, ...lines] = listed.stdout.trimEnd().split('\n');
    const shardSize = 3;
    const shardCount = Math.ceil(declared.length / shardSize);
    assert.equal(heading, `${declared.length} tests in ${shardCount} shards`);
    assert.equal(lines.length, shardCount);
    const selected = [];
    for (let index = 0; index < lines.length; index += 1) {
        const names = declared.slice(index * shardSize,
            (index + 1) * shardSize);
        assert.equal(lines[index], `${index + 1}: ${names.join(' | ')}`);
        selected.push(...names);
    }
    assert.deepEqual(selected, declared);
});

test('flow shard runner rejects invalid selectors before native export', () => {
    for (const args of [
        ['--shard', '0'], ['--shard', '-1'], ['--shard', '1.0'],
        ['--shard', '999999999999999999999'], ['--shard'],
        ['--shard', '1', '--list'], ['--unknown'], ['--all', '--list'],
    ]) {
        const result = run(...args);
        assert.equal(result.status, 2, `${args}: ${result.stderr}`);
        assert.match(result.stderr, /Usage: node flow-shards\.mjs/);
        assert.doesNotMatch(result.stdout, /Flow shard \d+\//);
    }
});

function tap(names) {
    return [
        'TAP version 13',
        ...names.flatMap((name, index) => [
            `# Subtest: ${name}`, `ok ${index + 1} - ${name}`,
        ]),
        `1..${names.length}`,
        `# tests ${names.length}`, `# pass ${names.length}`,
        '# fail 0', '# cancelled 0', '# skipped 0', '# todo 0',
    ].join('\n') + '\n';
}

test('flow shard receipts require exact names and complete TAP counters',
    () => {
    const names = ['first native export', 'second native export',
        'third native export'];
    const accepted = tap(names);
    assert.equal(completeShardTap(accepted, names), true);
    assert.equal(completeShardTap(accepted, []), false);
    assert.equal(completeShardTap(accepted, names.slice(0, 2)), false);
    assert.equal(completeShardTap(accepted,
        [names[1], names[0], names[2]]), false);
    assert.equal(completeShardTap(accepted,
        [names[0], names[0], names[2]]), false);
    assert.equal(completeShardTap(tap(['flow.mjs']), names), false);
    for (const [from, to] of [
        ['# tests 3', '# tests 0'],
        ['# pass 3', '# pass 0'],
        ['# pass 3', '# pass 2'],
        ['# fail 0', '# fail 1'],
        ['# cancelled 0', '# cancelled 1'],
        ['# skipped 0', '# skipped 1'],
        ['# todo 0', '# todo 1'],
        ['# Subtest: first native export', '# Subtest: wrong export'],
    ]) {
        assert.equal(completeShardTap(accepted.replace(from, to), names),
            false, to);
        assert.equal(completeShardTap(accepted.replace(from, ''), names),
            false, `${from} missing`);
        assert.equal(completeShardTap(accepted + `${from}\n`, names),
            false, `${from} duplicate`);
    }
    assert.equal(completeShardTap('', names), false);
});

test('generic Node test discovery never launches native export shards',
    () => {
    const result = spawnSync(process.execPath, [
        '--test', '--test-reporter=tap', runner,
    ], { cwd: root, encoding: 'utf8', timeout: 10_000, env: childEnv });
    assert.equal(result.error, undefined, result.error?.message);
    assert.equal(result.status, 0, result.stderr);
    assert.doesNotMatch(result.stdout, /Flow shard \d+\//);
    assert.doesNotMatch(result.stdout, /Passed \d+ selected native flow/);
});

test('shards build and run the native CLI with complete selected receipts',
    () => {
    const result = run('--shard', '1');
    assert.equal(result.status, 0, result.stderr);
    const source = readFileSync(flow, 'utf8');
    const names = [...source.matchAll(/^test\('([^'\n]+)'/gm)]
        .map((match) => match[1]);
    assert.match(result.stdout, /Flow shard 1\/\d+ \(3 cases\)/);
    assert.ok(completeShardTap(result.stdout, names.slice(0, 3)));
    assert.match(result.stdout, /Passed 3 selected native flow cases\./);
});

test('prebuilt CLI selection cannot silently fall back to Cargo', () => {
    const result = spawnSync(process.execPath, [
        '--test', '--test-reporter=tap', '--test-name-pattern',
        '^stdout nominal flow rows predict independently authored PDF/PNG ink$',
        flow,
    ], { cwd: root, encoding: 'utf8', timeout: 20_000,
        env: { ...childEnv,
            ATRAMENT_FLOW_TEST_CLI: '/nonexistent/atrament-flow-test-cli' },
    });
    assert.equal(result.error, undefined, result.error?.message);
    assert.notEqual(result.status, 0);
    assert.match(result.stdout, /null !== 0/);
    assert.doesNotMatch(result.stdout, /Finished .*target/);
    assert.equal(completeShardTap(result.stdout, [
        'stdout nominal flow rows predict independently authored PDF/PNG ink',
    ]), false);
});
